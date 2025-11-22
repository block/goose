use super::styles::Theme;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static THEME_SET: OnceLock<ThemeSet> = OnceLock::new();

pub struct MarkdownRenderer<'a> {
    text: &'a str,
    width: usize,

    // Internal state
    lines: Vec<Line<'static>>,
    current_line: Vec<Span<'static>>,
    current_width: usize,
    style_stack: Vec<Style>,
    current_style: Style,
    code_block_lang: Option<String>,
}

impl<'a> MarkdownRenderer<'a> {
    pub fn new(text: &'a str, width: usize, theme: &'a Theme, base_style: Option<Style>) -> Self {
        let current_style =
            base_style.unwrap_or_else(|| Style::default().fg(theme.base.foreground));
        Self {
            text,
            width,
            lines: Vec::new(),
            current_line: Vec::new(),
            current_width: 0,
            style_stack: Vec::new(),
            current_style,
            code_block_lang: None,
        }
    }

    fn get_syntax_set() -> &'static SyntaxSet {
        SYNTAX_SET.get_or_init(SyntaxSet::load_defaults_newlines)
    }

    fn get_theme_set() -> &'static ThemeSet {
        THEME_SET.get_or_init(ThemeSet::load_defaults)
    }

    fn syntect_to_ratatui(color: syntect::highlighting::Color) -> Color {
        Color::Rgb(color.r, color.g, color.b)
    }

    pub fn render_lines(&mut self) -> Vec<Line<'static>> {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        let parser = Parser::new_ext(self.text, options);

        for event in parser {
            match event {
                Event::Text(ref t) | Event::Code(ref t) => {
                    let is_code_span = matches!(event, Event::Code(_));
                    self.handle_text(t, is_code_span);
                }
                Event::Start(tag) => self.handle_start_tag(tag),
                Event::End(tag) => self.handle_end_tag(tag),
                Event::SoftBreak | Event::HardBreak => self.new_line(),
                _ => {}
            }
        }

        if !self.current_line.is_empty() {
            self.lines
                .push(Line::from(std::mem::take(&mut self.current_line)));
        }

        std::mem::take(&mut self.lines)
    }

    fn new_line(&mut self) {
        self.lines
            .push(Line::from(std::mem::take(&mut self.current_line)));
        self.current_width = 0;
    }

    fn handle_text(&mut self, text: &str, is_code_span: bool) {
        let content = text.to_string();

        if self.code_block_lang.is_some() {
            self.handle_code_block_text(&content);
            return;
        }

        let mut style = self.current_style;
        if is_code_span {
            style = style.fg(Color::Yellow);
        }

        let parts: Vec<&str> = content.split(' ').collect();
        for (i, part) in parts.iter().enumerate() {
            let is_last = i == parts.len() - 1;
            let mut word = part.to_string();
            if !is_last {
                word.push(' ');
            }

            if word.is_empty() {
                continue;
            }

            let sub_parts: Vec<&str> = word.split('\n').collect();
            for (j, sub) in sub_parts.iter().enumerate() {
                if j > 0 {
                    self.new_line();
                }

                if sub.is_empty() {
                    continue;
                }

                let sub_width = sub.chars().count();

                if self.current_width + sub_width > self.width && self.current_width > 0 {
                    self.new_line();
                }

                self.current_line.push(Span::styled(sub.to_string(), style));
                self.current_width += sub_width;
            }
        }
    }

    fn handle_code_block_text(&mut self, content: &str) {
        let ss = Self::get_syntax_set();
        let ts = Self::get_theme_set();
        let theme = &ts.themes["base16-ocean.dark"];

        let lang = self.code_block_lang.as_ref().unwrap();
        let syntax = ss
            .find_syntax_by_token(lang)
            .unwrap_or_else(|| ss.find_syntax_plain_text());

        let mut h = HighlightLines::new(syntax, theme);

        for line in content.split_inclusive('\n') {
            let ranges: Vec<(syntect::highlighting::Style, &str)> =
                h.highlight_line(line, ss).unwrap_or_default();

            for (style, text) in ranges {
                let fg = Self::syntect_to_ratatui(style.foreground);
                let span_style = Style::default().fg(fg);

                if text.ends_with('\n') {
                    let trimmed = text.trim_end_matches('\n').trim_end_matches('\r');
                    if !trimmed.is_empty() {
                        self.current_line
                            .push(Span::styled(trimmed.to_string(), span_style));
                    }
                    self.new_line();
                } else {
                    self.current_line
                        .push(Span::styled(text.to_string(), span_style));
                    self.current_width += text.chars().count();
                }
            }
        }
    }

    fn handle_start_tag(&mut self, tag: Tag) {
        match tag {
            Tag::Emphasis => {
                self.style_stack.push(self.current_style);
                self.current_style = self.current_style.add_modifier(Modifier::ITALIC);
            }
            Tag::Strong => {
                self.style_stack.push(self.current_style);
                self.current_style = self.current_style.add_modifier(Modifier::BOLD);
            }
            Tag::CodeBlock(kind) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(l) => l.to_string(),
                    CodeBlockKind::Indented => "text".to_string(),
                };
                self.code_block_lang = Some(lang);

                if !self.current_line.is_empty() {
                    self.new_line();
                }
            }
            Tag::List(_) | Tag::Item => {
                if !self.current_line.is_empty() {
                    self.new_line();
                }
                if matches!(tag, Tag::Item) {
                    self.current_line
                        .push(Span::styled("• ", self.current_style));
                    self.current_width += 2;
                }
            }
            _ => {}
        }
    }

    fn handle_end_tag(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::Emphasis | TagEnd::Strong => {
                if let Some(s) = self.style_stack.pop() {
                    self.current_style = s;
                }
            }
            TagEnd::CodeBlock => {
                self.code_block_lang = None;
                if !self.current_line.is_empty() {
                    self.new_line();
                }
            }
            TagEnd::List(_) | TagEnd::Item => {
                if !self.current_line.is_empty() {
                    self.new_line();
                }
            }
            TagEnd::Paragraph => {
                if !self.current_line.is_empty() {
                    self.new_line();
                }
                self.lines.push(Line::from(""));
            }
            _ => {}
        }
    }
}
