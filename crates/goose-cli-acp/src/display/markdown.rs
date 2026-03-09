use std::io::Write;
use std::time::{Duration, Instant};

use console::measure_text_width;
use crossterm::{cursor, queue, terminal};
use pulldown_cmark::{Alignment, Event, Options, Parser, Tag, TagEnd};
use syntect::easy::HighlightLines;
use syntect::highlighting::Theme;
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

use super::style;

struct SyntaxHighlighter {
    syntax_set: SyntaxSet,
    theme: Theme,
}

impl SyntaxHighlighter {
    // ~50ms construction cost — called once per session.
    fn new() -> Self {
        let syntax_set = two_face::syntax::extra_newlines();
        let theme_name = super::style::syntect_theme_name();
        let theme = two_face::theme::extra().get(theme_name).clone();
        Self { syntax_set, theme }
    }

    fn highlight(&self, code: &str, lang: &str) -> Vec<String> {
        if style::no_color() {
            return LinesWithEndings::from(code)
                .map(|line| line.trim_end_matches('\n').to_string())
                .collect();
        }

        let syntax = self
            .syntax_set
            .find_syntax_by_token(lang)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
        let mut h = HighlightLines::new(syntax, &self.theme);
        let mut lines = Vec::new();
        for line in LinesWithEndings::from(code) {
            let rendered = match h.highlight_line(line, &self.syntax_set) {
                Ok(ranges) => as_24_bit_terminal_escaped(&ranges[..], false),
                Err(_) => line.to_string(),
            };
            lines.push(rendered.trim_end_matches('\n').to_string());
        }
        lines
    }
}

const DEFAULT_LIVE_WINDOW: usize = 6;
const REWRITE_INTERVAL: Duration = Duration::from_millis(33);

/// Override via `GOOSE_LIVE_WINDOW` (integer or "auto" for full terminal height).
///
/// Clamped to `max(term_rows - 1, 1)` logical lines so the unstable region
/// is unlikely to exceed what `MoveUp` can reach. Not fully sufficient — long
/// wrapped lines can still inflate physical rows beyond the terminal height.
/// `move_up_and_print` applies the same clamp on the actual `MoveUp` distance.
fn live_window(term_rows: usize) -> usize {
    use std::sync::OnceLock;
    static OVERRIDE: OnceLock<Option<usize>> = OnceLock::new();

    let max = term_rows.saturating_sub(1).max(1);

    let parsed = OVERRIDE.get_or_init(|| {
        std::env::var("GOOSE_LIVE_WINDOW").ok().and_then(|v| {
            if v.eq_ignore_ascii_case("auto") {
                Some(0) // sentinel: use term_rows at call time
            } else {
                v.parse::<usize>().ok().map(|n| n.max(1))
            }
        })
    });

    let raw = match parsed {
        Some(0) => max,              // "auto" → full terminal height (clamped)
        Some(n) => *n,               // explicit override
        None => DEFAULT_LIVE_WINDOW, // default
    };
    raw.min(max)
}

struct RenderedLine {
    content: String,
    visible_width: usize,
}

fn physical_rows(visible_width: usize, term_width: usize) -> usize {
    if visible_width == 0 {
        return 1; // empty line still occupies one row (\r\n)
    }
    if term_width == 0 {
        return 1; // guard against division by zero
    }
    visible_width.div_ceil(term_width)
}

fn style_heading(lvl: u8, text: &str) -> String {
    match lvl {
        1 => style::heading_h1(text).to_string(),
        2 => style::heading_h2(text).to_string(),
        3 => style::heading_h3(text).to_string(),
        _ => style::heading_h2(text).to_string(),
    }
}

fn push_line(lines: &mut Vec<RenderedLine>, current_line: &mut String) {
    let width = measure_text_width(current_line);
    lines.push(RenderedLine {
        content: std::mem::take(current_line),
        visible_width: width,
    });
}

struct TableState {
    alignments: Vec<Alignment>,
    rows: Vec<Vec<String>>,
    current_row: Vec<String>,
    current_cell: String,
}

impl TableState {
    fn new() -> Self {
        Self {
            alignments: Vec::new(),
            rows: Vec::new(),
            current_row: Vec::new(),
            current_cell: String::new(),
        }
    }
}

#[derive(Default)]
struct InlineState {
    strong: bool,
    emphasis: bool,
    link: bool,
    link_url: String,
}

pub struct MarkdownStreamer {
    buffer: String,
    committed_count: usize,
    prev_unstable_rows: usize, // physical rows of previous unstable window (for MoveUp)
    printed_rows: usize, // committed stable rows — combined with prev_unstable_rows to clamp MoveUp
    last_rewrite: Instant, // debounce timer (~30fps)
    highlighter: SyntaxHighlighter,
}

impl Default for MarkdownStreamer {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownStreamer {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            committed_count: 0,
            prev_unstable_rows: 0,
            printed_rows: 0,
            last_rewrite: Instant::now(),
            highlighter: SyntaxHighlighter::new(),
        }
    }

    pub fn push(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }

        self.buffer.push_str(text);

        if self.last_rewrite.elapsed() >= REWRITE_INTERVAL {
            self.commit_and_repaint();
            self.last_rewrite = Instant::now();
        }
    }

    pub fn finish(&mut self) {
        if self.buffer.is_empty() {
            return;
        }

        let (_, rows) = super::term_size();
        let term_rows = rows as usize;
        let lines = self.render_to_lines();

        // Re-parse may produce fewer lines than previously committed.
        let committed_count = self.committed_count.min(lines.len());

        let mut out = std::io::stdout();
        let total_on_screen = self.printed_rows + self.prev_unstable_rows;
        move_up_and_print(
            &mut out,
            &lines[committed_count..],
            self.prev_unstable_rows,
            total_on_screen,
            term_rows,
        );
        queue!(out, terminal::Clear(terminal::ClearType::FromCursorDown)).ok();
        out.flush().ok();

        self.reset();

        write!(out, "\r\n").ok();
        out.flush().ok();
    }

    /// Call before switching from stdout (text) to stderr (tool calls, thinking).
    pub fn finish_if_active(&mut self) {
        if !self.buffer.is_empty() {
            self.finish();
        }
    }

    fn commit_and_repaint(&mut self) {
        let (cols, rows) = super::term_size();
        let term_width = cols as usize;
        let term_rows = rows as usize;
        let lines = self.render_to_lines();
        let total = lines.len();

        let stable_end = total.saturating_sub(live_window(term_rows));

        // Re-parse may produce fewer lines than previously committed.
        let committed_count = self.committed_count.min(stable_end);

        let mut out = std::io::stdout();

        let total_on_screen = self.printed_rows + self.prev_unstable_rows;
        move_up_and_print(
            &mut out,
            &lines[committed_count..stable_end],
            self.prev_unstable_rows,
            total_on_screen,
            term_rows,
        );

        // Track physical rows for newly committed stable lines.
        for line in &lines[committed_count..stable_end] {
            self.printed_rows += physical_rows(line.visible_width, term_width);
        }
        self.committed_count = stable_end;

        queue!(out, terminal::BeginSynchronizedUpdate).ok();
        let mut unstable_rows = 0;
        for line in &lines[stable_end..] {
            queue!(out, terminal::Clear(terminal::ClearType::CurrentLine)).ok();
            write!(out, "{}\r\n", line.content).ok();
            unstable_rows += physical_rows(line.visible_width, term_width);
        }
        queue!(out, terminal::Clear(terminal::ClearType::FromCursorDown)).ok();
        queue!(out, terminal::EndSynchronizedUpdate).ok();
        out.flush().ok();

        self.prev_unstable_rows = unstable_rows;
    }

    fn reset(&mut self) {
        self.buffer.clear();
        self.committed_count = 0;
        self.prev_unstable_rows = 0;
        self.printed_rows = 0;
    }

    fn render_to_lines(&self) -> Vec<RenderedLine> {
        let opts =
            Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;
        let parser = Parser::new_ext(&self.buffer, opts);

        let mut lines: Vec<RenderedLine> = Vec::new();
        let mut current_line = String::new();

        let mut in_code = false;
        let mut code_lang = String::new();
        let mut code_buf = String::new();
        let mut in_table = false;
        let mut table = TableState::new();
        let mut inline = InlineState::default();
        let mut heading_level: Option<u8> = None;
        let mut blockquote_depth: usize = 0;
        let mut list_stack: Vec<Option<u64>> = Vec::new();

        for event in parser {
            match event {
                Event::Start(Tag::CodeBlock(kind)) => {
                    in_code = true;
                    code_lang = match kind {
                        pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                        pulldown_cmark::CodeBlockKind::Indented => String::new(),
                    };
                    code_buf.clear();
                }
                Event::End(TagEnd::CodeBlock) => {
                    let highlighted = self.highlighter.highlight(&code_buf, &code_lang);
                    for hl in &highlighted {
                        let line_content = format!("  {}{}", hl, style::reset());
                        let width = measure_text_width(&line_content);
                        lines.push(RenderedLine {
                            content: line_content,
                            visible_width: width,
                        });
                    }
                    in_code = false;
                    code_lang.clear();
                    code_buf.clear();
                }
                Event::Start(Tag::Table(alignments)) => {
                    in_table = true;
                    table.alignments = alignments;
                    table.rows.clear();
                }
                Event::End(TagEnd::Table) => {
                    let mut table_buf: Vec<u8> = Vec::new();
                    render_table(&table.rows, &table.alignments, &mut table_buf);
                    let table_str = String::from_utf8_lossy(&table_buf);
                    for table_line in table_str.split("\r\n") {
                        if !table_line.is_empty() {
                            let width = measure_text_width(table_line);
                            lines.push(RenderedLine {
                                content: table_line.to_string(),
                                visible_width: width,
                            });
                        }
                    }
                    in_table = false;
                    table.rows.clear();
                    table.alignments.clear();
                }
                Event::Start(Tag::TableHead | Tag::TableRow) => {
                    table.current_row.clear();
                }
                Event::End(TagEnd::TableHead | TagEnd::TableRow) => {
                    table.rows.push(std::mem::take(&mut table.current_row));
                }
                Event::Start(Tag::TableCell) => {
                    table.current_cell.clear();
                }
                Event::End(TagEnd::TableCell) => {
                    table
                        .current_row
                        .push(std::mem::take(&mut table.current_cell));
                }
                Event::Start(Tag::Heading { level, .. }) => {
                    let lvl = level as u8;
                    heading_level = Some(lvl);
                    let prefix = "#".repeat(lvl as usize);
                    let styled_prefix = style_heading(lvl, &prefix);
                    current_line.push_str(&styled_prefix);
                    current_line.push(' ');
                }
                Event::End(TagEnd::Heading(_)) => {
                    heading_level = None;
                    push_line(&mut lines, &mut current_line);
                }
                Event::Start(Tag::Strong) => inline.strong = true,
                Event::End(TagEnd::Strong) => inline.strong = false,
                Event::Start(Tag::Emphasis) => inline.emphasis = true,
                Event::End(TagEnd::Emphasis) => inline.emphasis = false,
                Event::Start(Tag::Strikethrough) => {
                    if !in_table {
                        current_line.push_str(style::strikethrough_on());
                    }
                }
                Event::End(TagEnd::Strikethrough) => {
                    if !in_table {
                        current_line.push_str(style::strikethrough_off());
                    }
                }
                Event::Start(Tag::Link { dest_url, .. }) => {
                    inline.link = true;
                    inline.link_url = dest_url.to_string();
                }
                Event::End(TagEnd::Link) => {
                    if in_table {
                        table
                            .current_cell
                            .push_str(&format!(" ({})", inline.link_url));
                    } else {
                        let url_display = format!("({})", inline.link_url);
                        current_line.push_str(&format!(" {}", style::link_url(&url_display)));
                    }
                    inline.link = false;
                    inline.link_url.clear();
                }
                Event::Start(Tag::BlockQuote(_)) => {
                    blockquote_depth += 1;
                    for _ in 0..blockquote_depth {
                        current_line.push_str(&format!("  {} ", style::blockquote_bar("│")));
                    }
                }
                Event::End(TagEnd::BlockQuote(_)) => {
                    blockquote_depth = blockquote_depth.saturating_sub(1);
                }
                Event::Start(Tag::List(start)) => {
                    list_stack.push(start);
                }
                Event::End(TagEnd::List(_)) => {
                    list_stack.pop();
                }
                Event::Start(Tag::Item) => {
                    let indent = "  ".repeat(list_stack.len());
                    match list_stack.last_mut() {
                        Some(Some(n)) => {
                            let marker = format!("{}. ", n);
                            current_line
                                .push_str(&format!("{indent}{}", style::list_marker(&marker)));
                            *n += 1;
                        }
                        _ => {
                            current_line.push_str(&format!("{indent}{} ", style::list_marker("•")));
                        }
                    }
                }
                Event::End(TagEnd::Item) => {
                    push_line(&mut lines, &mut current_line);
                }
                Event::Start(Tag::Paragraph) => {}
                Event::End(TagEnd::Paragraph) => {
                    if !in_table {
                        push_line(&mut lines, &mut current_line);
                    }
                }
                Event::Code(code) => {
                    if in_table {
                        table.current_cell.push_str(&code);
                    } else {
                        current_line.push_str(&style::inline_code(&code).to_string());
                    }
                }
                Event::Text(t) => {
                    if in_code {
                        code_buf.push_str(&t);
                    } else if in_table {
                        table.current_cell.push_str(&t);
                    } else if let Some(lvl) = heading_level {
                        current_line.push_str(&style_heading(lvl, t.as_ref()));
                    } else if inline.link {
                        current_line.push_str(&style::link_text(t.as_ref()).to_string());
                    } else if blockquote_depth > 0 {
                        current_line.push_str(&style::blockquote_text(t.as_ref()).to_string());
                    } else if inline.strong && inline.emphasis {
                        current_line.push_str(&style::strong_emphasis(t.as_ref()).to_string());
                    } else if inline.strong {
                        current_line.push_str(&style::strong(t.as_ref()).to_string());
                    } else if inline.emphasis {
                        current_line.push_str(&style::emphasis(t.as_ref()).to_string());
                    } else {
                        current_line.push_str(&t);
                    }
                }
                Event::SoftBreak => {
                    if in_table {
                        table.current_cell.push(' ');
                    } else if blockquote_depth > 0 {
                        push_line(&mut lines, &mut current_line);
                        for _ in 0..blockquote_depth {
                            current_line.push_str(&format!("  {} ", style::blockquote_bar("│")));
                        }
                    } else {
                        current_line.push(' ');
                    }
                }
                Event::HardBreak => {
                    push_line(&mut lines, &mut current_line);
                }
                Event::Rule => {
                    let (cols, _) = super::term_size();
                    let width = (cols as usize).min(80).saturating_sub(2);
                    let hr = "─".repeat(width);
                    current_line.push_str(&style::rule(&hr).to_string());
                    push_line(&mut lines, &mut current_line);
                }
                Event::TaskListMarker(checked) => {
                    let marker = if checked { "☑" } else { "☐" };
                    current_line.push_str(&format!("{marker} "));
                }
                _ => {}
            }
        }

        // Handle trailing partial line (no final \n)
        if !current_line.is_empty() {
            push_line(&mut lines, &mut current_line);
        }

        lines
    }
}

fn move_up_and_print(
    out: &mut impl Write,
    lines: &[RenderedLine],
    prev_unstable_rows: usize,
    total_rows_on_screen: usize,
    term_rows: usize,
) {
    if prev_unstable_rows > 0 {
        // Clamp to both terminal height and total rows actually on screen — prevents
        // overshooting into scrollback when output is near the top of the screen.
        let max_up = total_rows_on_screen.min(term_rows.saturating_sub(1).max(1));
        let up = prev_unstable_rows.min(max_up) as u16;
        queue!(out, cursor::MoveUp(up)).ok();
    }
    queue!(out, cursor::MoveToColumn(0)).ok();
    for line in lines {
        queue!(out, terminal::Clear(terminal::ClearType::CurrentLine)).ok();
        write!(out, "{}\r\n", line.content).ok();
    }
}

// `out` is generic over `Write` for testability; in production always `Vec<u8>` (infallible).
fn render_table(rows: &[Vec<String>], alignments: &[Alignment], out: &mut impl Write) {
    if rows.is_empty() {
        return;
    }

    let col_count = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    let mut widths = vec![0usize; col_count];
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_count && measure_text_width(cell) > widths[i] {
                widths[i] = measure_text_width(cell);
            }
        }
    }

    for (i, row) in rows.iter().enumerate() {
        if i == 1 && rows.len() > 1 {
            let sep: Vec<String> = widths.iter().map(|&w| "─".repeat(w + 2)).collect();
            let separator = sep.join("┼");
            write!(out, "  {}\r\n", style::table_border(&separator)).ok();
        }

        write!(out, "  ").ok();
        for (j, width) in widths.iter().enumerate() {
            let cell = row.get(j).map(|s| s.as_str()).unwrap_or("");
            let align = alignments.get(j).copied().unwrap_or(Alignment::None);
            let padded = pad_cell(cell, *width, align);
            if i == 0 {
                write!(out, "{}", style::table_header(&padded)).ok();
            } else {
                write!(out, "{padded}").ok();
            }
            if j + 1 < widths.len() {
                write!(out, "{}", style::table_border("│")).ok();
            }
        }
        write!(out, "\r\n").ok();
    }
}

fn pad_cell(text: &str, width: usize, align: Alignment) -> String {
    let text_width = measure_text_width(text);
    let padding = width.saturating_sub(text_width);
    let (left, right) = match align {
        Alignment::Right => (padding, 0),
        Alignment::Center => (padding / 2, padding - padding / 2),
        _ => (0, padding),
    };
    format!(" {}{}{} ", " ".repeat(left), text, " ".repeat(right))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_rows_zero_width() {
        assert_eq!(physical_rows(0, 80), 1);
    }

    #[test]
    fn physical_rows_wrap_many() {
        assert_eq!(physical_rows(200, 40), 5);
    }

    #[test]
    fn physical_rows_zero_term_width() {
        assert_eq!(physical_rows(100, 0), 1);
    }

    #[test]
    fn visible_width_with_ansi() {
        assert_eq!(measure_text_width("\x1b[1mhello\x1b[0m"), 5);
    }

    #[test]
    fn visible_width_unicode_wide() {
        assert_eq!(measure_text_width("你好"), 4);
    }

    #[test]
    fn render_to_lines_simple_paragraph() {
        let mut s = MarkdownStreamer::new();
        s.buffer = "Hello world\n".to_string();
        let lines = s.render_to_lines();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].content, "Hello world");
    }

    #[test]
    fn render_to_lines_multiple_paragraphs() {
        let mut s = MarkdownStreamer::new();
        s.buffer = "First\n\nSecond\n".to_string();
        let lines = s.render_to_lines();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn render_to_lines_trailing_partial() {
        let mut s = MarkdownStreamer::new();
        s.buffer = "no newline".to_string();
        let lines = s.render_to_lines();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].content, "no newline");
    }

    #[test]
    fn stable_unstable_split_more_than_live_window() {
        let mut s = MarkdownStreamer::new();
        s.buffer = (1..=10)
            .map(|i| format!("Paragraph {}\n", i))
            .collect::<Vec<_>>()
            .join("\n");
        let lines = s.render_to_lines();
        let total = lines.len();
        let window = live_window(24);
        let stable_end = total.saturating_sub(window);
        assert!(stable_end > 0, "Should have stable lines");
        assert_eq!(total - stable_end, window.min(total));
    }

    #[test]
    fn live_window_clamped_to_term() {
        // With a 1-row terminal, live_window is clamped to 1 (can't exceed visible area)
        let w = live_window(1);
        assert_eq!(w, 1);
    }

    #[test]
    fn move_up_clamps_to_total_rows_on_screen() {
        // When near the top of the screen (few rows printed), MoveUp must not
        // overshoot past the start of output.
        let mut buf = Vec::new();
        let lines = vec![RenderedLine {
            content: "replacement".into(),
            visible_width: 11,
        }];

        // Scenario: 3 rows on screen, but prev_unstable was 5 (e.g. table flush).
        // MoveUp should clamp to 3, not 5.
        move_up_and_print(&mut buf, &lines, 5, 3, 24);
        let output = String::from_utf8(buf).unwrap();
        // CSI MoveUp(3) = "\x1b[3A", not "\x1b[5A"
        assert!(
            output.contains("\x1b[3A"),
            "expected MoveUp(3), got: {output:?}"
        );
        assert!(!output.contains("\x1b[5A"), "should not MoveUp(5)");
    }

    #[test]
    fn move_up_clamps_to_term_rows() {
        // When lots of rows printed but terminal is small, clamp to term_rows - 1.
        let mut buf = Vec::new();
        let lines = vec![RenderedLine {
            content: "line".into(),
            visible_width: 4,
        }];
        // 100 rows on screen, prev_unstable=50, but terminal is only 10 rows.
        move_up_and_print(&mut buf, &lines, 50, 100, 10);
        let output = String::from_utf8(buf).unwrap();
        // Should clamp to term_rows - 1 = 9
        assert!(
            output.contains("\x1b[9A"),
            "expected MoveUp(9), got: {output:?}"
        );
    }

    #[test]
    fn nested_list_preserves_outer_numbering() {
        let mut s = MarkdownStreamer::new();
        s.buffer = "1. First\n   - nested bullet\n2. Second\n".to_string();
        let lines = s.render_to_lines();
        let plain: Vec<String> = lines
            .iter()
            .map(|l| console::strip_ansi_codes(&l.content).into_owned())
            .collect();
        // Outer list items should be numbered 1. and 2.
        assert!(plain[0].contains("1."), "first item: {:?}", plain[0]);
        assert!(
            plain.iter().any(|l| l.contains("2.")),
            "second item should be numbered 2., got: {plain:?}"
        );
    }
}
