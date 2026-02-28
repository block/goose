pub mod markdown;
pub mod style;
pub mod theme;

use std::io::Write;
use std::time::Duration;

use crossterm::style::StyledContent;

const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

pub(crate) fn term_size() -> (u16, u16) {
    crossterm::terminal::size().unwrap_or((80, 24))
}

/// Strip terminal control sequences from untrusted text (agent output, tool results).
/// Preserves `\n`, `\t`, `\r` but removes CSI/OSC escape sequences and other C0/C1 controls.
/// Returns the input unchanged (no allocation) when no control chars are present.
pub(crate) fn sanitize(s: &str) -> std::borrow::Cow<'_, str> {
    // Fast path: most strings are clean.
    // Check for C0 controls (except \n, \t, \r), DEL (0x7F), ESC, and C1 range (U+0080..U+009F).
    let needs_sanitize = s.chars().any(|ch| {
        ch == '\x1b' || ch == '\x7f' || (ch.is_control() && ch != '\n' && ch != '\t' && ch != '\r')
    });
    if !needs_sanitize {
        return std::borrow::Cow::Borrowed(s);
    }

    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            if let Some(&next) = chars.peek() {
                if next == '[' {
                    // CSI: skip until final byte (ECMA-48: 0x40..=0x7E, i.e. '@'..='~')
                    chars.next();
                    while let Some(&c) = chars.peek() {
                        chars.next();
                        if ('@'..='~').contains(&c) {
                            break;
                        }
                    }
                    continue;
                } else if next == ']' {
                    chars.next();
                    while let Some(&c) = chars.peek() {
                        chars.next();
                        if c == '\x07' {
                            break;
                        }
                        if c == '\x1b' {
                            if chars.peek() == Some(&'\\') {
                                chars.next();
                            }
                            break;
                        }
                    }
                    continue;
                }
            }
            continue;
        }
        if ch == '\n' || ch == '\t' || ch == '\r' || !ch.is_control() {
            result.push(ch);
        }
    }
    std::borrow::Cow::Owned(result)
}

/// Public wrapper that always returns an owned String (for call sites that need String).
pub(crate) fn sanitize_control_chars(s: &str) -> String {
    sanitize(s).into_owned()
}

fn flush_stderr() {
    std::io::stderr().flush().ok();
}

pub(crate) fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let end: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{end}…")
    }
}

/// Available width for tool args, accounting for the fixed prefix.
/// `prefix_len` is the visible character count of everything before the args
/// (indent + icon + space + title).
fn args_width(prefix_len: usize) -> usize {
    let (cols, _) = term_size();
    let cols = cols as usize;
    cols.saturating_sub(prefix_len + 4).max(10)
}

pub fn update_spinner(frame: usize) {
    let ch = SPINNER[frame % SPINNER.len()];
    eprint!("\r  {} ", style::pending(&ch.to_string()));
    flush_stderr();
}

pub fn clear_spinner() {
    use crossterm::terminal::{Clear, ClearType};
    eprint!("\r{}", Clear(ClearType::CurrentLine));
    flush_stderr();
}

pub fn print_thinking(text: &str) {
    for line in text.lines() {
        eprint!("  {} {}\r\n", style::dim("│"), style::dim(line));
    }
    flush_stderr();
}

/// Shared: icon + tool name + optional args suffix.
fn print_tool_line(
    icon: StyledContent<&str>,
    title: &str,
    args: Option<&str>,
    number: Option<usize>,
) {
    let title = sanitize(title);
    let num_prefix = number.map(|n| format!("[{n}] ")).unwrap_or_default();
    // .len() is correct here: tool titles and num_prefix are ASCII (ACP protocol convention).
    let prefix_len = 4 + num_prefix.len() + title.len();
    let suffix = match args {
        Some(a) => {
            let a = sanitize(a);
            format!(" ({})", truncate(&a, args_width(prefix_len)))
        }
        None => String::new(),
    };
    eprint!(
        "  {} {}{}{}\r\n",
        icon,
        style::dim(&num_prefix),
        style::tool_name(&title),
        style::dim(&suffix)
    );
    flush_stderr();
}

/// Shared: icon + tool name + optional args + timing.
fn print_tool_result(
    icon: StyledContent<&str>,
    title: &str,
    elapsed: Duration,
    args: Option<&str>,
    number: Option<usize>,
) {
    let title = sanitize(title);
    let num_suffix = number.map(|n| format!(" [{n}]")).unwrap_or_default();
    let timing = format!("({:.1}s)", elapsed.as_secs_f64());
    // .len() is correct: all components are ASCII (tool titles, timing format, number suffix).
    let prefix_len = 4 + title.len() + 1 + timing.len() + num_suffix.len();
    let suffix = match args {
        Some(a) => {
            let a = sanitize(a);
            format!(" ({})", truncate(&a, args_width(prefix_len)))
        }
        None => String::new(),
    };
    eprint!(
        "  {} {}{} {}{}\r\n",
        icon,
        style::tool_name(&title),
        style::dim(&suffix),
        style::dim(&timing),
        style::dim(&num_suffix),
    );
    flush_stderr();
}

/// `rich`: true = styled TUI output (raw mode active); false = plain text to stderr.
pub fn print_tool_start(title: &str, input: Option<&serde_json::Value>, rich: bool) {
    if rich {
        let summary = input.and_then(summarize_args);
        print_tool_line(style::pending("⚙"), title, summary.as_deref(), None);
    } else {
        let title = sanitize(title);
        let args = input
            .map(|v| serde_json::to_string_pretty(v).unwrap_or_default())
            .unwrap_or_default();
        writeln!(std::io::stderr(), "[tool_call] {title}").ok();
        if !args.is_empty() {
            for line in args.lines() {
                writeln!(std::io::stderr(), "  {line}").ok();
            }
        }
    }
}

pub fn print_tool_complete(
    title: &str,
    elapsed: Duration,
    args: Option<&str>,
    number: Option<usize>,
    rich: bool,
) {
    if rich {
        print_tool_result(style::success("✓"), title, elapsed, args, number);
    } else {
        let title = sanitize(title);
        writeln!(std::io::stderr(), "[tool_result] ✓ {title}").ok();
    }
}

pub fn print_tool_failed(title: &str, elapsed: Duration, number: Option<usize>, rich: bool) {
    if rich {
        print_tool_result(style::error("✗"), title, elapsed, None, number);
    } else {
        let title = sanitize(title);
        writeln!(std::io::stderr(), "[tool_result] ✗ {title}").ok();
    }
}

/// `rich`: true = styled output with `\r\n` (raw mode active); false = plain lines.
pub fn print_tool_output(content: &str, mode: ToolOutputMode, rich: bool) {
    if matches!(mode, ToolOutputMode::None) {
        return;
    }

    let content = sanitize(content);

    fn emit(line: &str, rich: bool) {
        if rich {
            eprint!("    {}\r\n", style::dim(line));
        } else {
            writeln!(std::io::stderr(), "  {line}").ok();
        }
    }

    match mode {
        ToolOutputMode::None => {}
        ToolOutputMode::Full => {
            for line in content.lines() {
                emit(line, rich);
            }
        }
        ToolOutputMode::Truncated(max) => {
            let lines: Vec<&str> = content.lines().collect();
            let head = max / 2;
            let tail = max - head;
            if lines.len() <= max || head + tail >= lines.len() {
                for line in &lines {
                    emit(line, rich);
                }
            } else {
                for line in &lines[..head] {
                    emit(line, rich);
                }
                emit(
                    &format!("... ({} lines omitted)", lines.len() - head - tail),
                    rich,
                );
                for line in &lines[lines.len() - tail..] {
                    emit(line, rich);
                }
            }
        }
    }

    if rich {
        flush_stderr();
    }
}

pub fn print_hint(msg: &str) {
    let msg = sanitize(msg);
    writeln!(std::io::stderr(), "  {}", style::dim(&format!("({msg})"))).ok();
}

pub fn print_full_tool_output(number: usize, title: &str, output: &str) {
    let title = sanitize(title);
    let output = sanitize(output);
    eprintln!("  Output of [{}] {}:", number, style::tool_name(&title));
    for line in output.lines() {
        eprintln!("    {}", style::dim(line));
    }
}

pub fn print_plain_text(text: &str) {
    let mut out = std::io::stdout();
    // Strip \r to prevent carriage-return overwriting in non-raw-mode output.
    // Raw mode uses \r\n explicitly; plain/pipe mode should not pass \r through.
    let clean = text.replace('\r', "");
    if out.write_all(clean.as_bytes()).is_err() {
        return;
    }
    let _ = out.flush();
}

pub fn print_permission_prompt(title: &str, input: Option<&serde_json::Value>) {
    let summary = input.and_then(summarize_args);
    print_tool_line(style::pending("●"), title, summary.as_deref(), None);
}

pub fn print_help(commands: &[(&str, &str)]) {
    for (name, desc) in commands {
        eprintln!("  {}  {}", style::dim(&format!("/{name}")), desc);
    }
    eprintln!();
    eprintln!("  {}  Insert newline", style::dim("Alt+Enter"));
    eprintln!("  {}       Command completion", style::dim("Tab"));
}

/// How tool call output is displayed in the terminal.
///
/// Set via `GOOSE_TOOL_OUTPUT` env var:
///   "collapsed"  — suppress output (show ✓/✗ + timing only)
///   "truncated"  — head/tail preview (default, 10 lines: 5 head + 5 tail)
///   "full"       — show everything
///   "<number>"   — truncated with custom line limit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolOutputMode {
    None,
    /// Show first N/2 + last N/2 lines, eliding the middle.
    Truncated(usize),
    Full,
}

impl Default for ToolOutputMode {
    fn default() -> Self {
        Self::Truncated(10)
    }
}

impl ToolOutputMode {
    pub fn from_env() -> Self {
        Self::parse(std::env::var("GOOSE_TOOL_OUTPUT").ok().as_deref())
    }

    fn parse(val: Option<&str>) -> Self {
        match val {
            Some("none" | "collapsed") => Self::None,
            Some("full") => Self::Full,
            Some("truncated") | None => Self::default(),
            Some(n) => n
                .parse::<usize>()
                .map(|n| Self::Truncated(n.max(2)))
                .unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tool_output_mode_tests {
    use super::*;

    #[test]
    fn parse_collapsed() {
        assert_eq!(
            ToolOutputMode::parse(Some("collapsed")),
            ToolOutputMode::None
        );
        assert_eq!(ToolOutputMode::parse(Some("none")), ToolOutputMode::None);
    }

    #[test]
    fn parse_numeric_clamped() {
        assert_eq!(
            ToolOutputMode::parse(Some("0")),
            ToolOutputMode::Truncated(2)
        );
        assert_eq!(
            ToolOutputMode::parse(Some("1")),
            ToolOutputMode::Truncated(2)
        );
        assert_eq!(
            ToolOutputMode::parse(Some("20")),
            ToolOutputMode::Truncated(20)
        );
    }

    #[test]
    fn parse_garbage_is_default() {
        assert_eq!(
            ToolOutputMode::parse(Some("banana")),
            ToolOutputMode::Truncated(10)
        );
    }
}

/// Request type for the /show command.
pub(crate) enum ShowRequest {
    List,
    ByNumber(usize),
    Last,
}

pub(crate) fn handle_show(
    req: ShowRequest,
    tool_outputs: &std::collections::VecDeque<crate::stream::StoredToolOutput>,
) {
    match req {
        ShowRequest::List => {
            if tool_outputs.is_empty() {
                print_hint("No tool outputs in this session");
            } else {
                eprintln!("Tool outputs:");
                for stored in tool_outputs {
                    let title = sanitize(&stored.title);
                    let lines = stored.output.lines().count();
                    eprintln!(
                        "  [{}] {} ({} lines)",
                        stored.id,
                        style::tool_name(&title),
                        lines
                    );
                }
            }
        }
        ShowRequest::ByNumber(n) => {
            if let Some(stored) = tool_outputs.iter().find(|s| s.id == n) {
                if stored.output.is_empty() {
                    print_hint(&format!(
                        "Tool call #{n} ({}) produced no output",
                        stored.title
                    ));
                } else {
                    print_full_tool_output(n, &stored.title, &stored.output);
                }
            } else {
                print_hint(&format!("No tool output #{n}. Use /show to list."));
            }
        }
        ShowRequest::Last => {
            if let Some(stored) = tool_outputs.back() {
                if stored.output.is_empty() {
                    print_hint(&format!(
                        "Last tool call ({}) produced no output",
                        stored.title
                    ));
                } else {
                    print_full_tool_output(stored.id, &stored.title, &stored.output);
                }
            } else {
                print_hint("No tool outputs in this session");
            }
        }
    }
}

pub(crate) fn display_history_item(update: &sacp::schema::SessionUpdate) {
    use sacp::schema::{ContentBlock, SessionUpdate};
    match update {
        SessionUpdate::UserMessageChunk(chunk) => {
            if let ContentBlock::Text(t) = &chunk.content {
                let text = sanitize(t.text.trim());
                if !text.is_empty() {
                    let preview = truncate(&text, 120);
                    eprintln!("  {} {}", style::dim("▸"), style::dim(&preview));
                }
            }
        }
        SessionUpdate::AgentMessageChunk(chunk) => {
            if let ContentBlock::Text(t) = &chunk.content {
                let text = sanitize(t.text.trim());
                if !text.is_empty() {
                    let preview = truncate(&text, 120);
                    eprintln!("  {} {}", style::dim("◂"), style::dim(&preview));
                }
            }
        }
        SessionUpdate::ToolCall(tc) => {
            let title = sanitize(&tc.title);
            eprintln!("  {} {}", style::dim("⚙"), style::dim(&title));
        }
        _ => {}
    }
}

pub fn summarize_args(val: &serde_json::Value) -> Option<String> {
    let obj = val.as_object()?;
    if obj.is_empty() {
        return None;
    }
    let parts: Vec<String> = obj
        .iter()
        .map(|(k, v)| {
            let val_str = match v {
                serde_json::Value::String(s) => {
                    // Collapse whitespace (newlines, tabs, runs of spaces) to single spaces
                    let collapsed: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
                    format!("\"{collapsed}\"")
                }
                other => other.to_string(),
            };
            format!("{k}: {val_str}")
        })
        .collect();
    Some(parts.join(", "))
}

#[cfg(test)]
mod display_tests {
    use super::*;
    use serde_json::json;

    // --- truncate ---

    #[test]
    fn truncate_over() {
        let result = truncate("hello world", 6);
        assert!(result.ends_with('…'));
        assert_eq!(result.chars().count(), 6);
    }

    // --- summarize_args ---

    #[test]
    fn summarize_non_object() {
        assert!(summarize_args(&json!("string")).is_none());
        assert!(summarize_args(&json!(42)).is_none());
    }

    #[test]
    fn summarize_collapses_whitespace() {
        let result = summarize_args(&json!({"text": "hello\n  world\ttab"})).unwrap();
        assert_eq!(result, r#"text: "hello world tab""#);
    }

    #[test]
    fn summarize_non_string_values() {
        let result = summarize_args(&json!({"count": 42, "flag": true})).unwrap();
        assert!(result.contains("count: 42"));
        assert!(result.contains("flag: true"));
    }

    // --- sanitize_control_chars ---

    #[test]
    fn sanitize_preserves_newlines_and_tabs() {
        assert_eq!(sanitize_control_chars("a\nb\tc\r\n"), "a\nb\tc\r\n");
    }

    #[test]
    fn sanitize_strips_csi_sequences() {
        // CSI color: ESC[31m (red)
        assert_eq!(
            sanitize_control_chars("\x1b[31mred text\x1b[0m"),
            "red text"
        );
    }

    #[test]
    fn sanitize_strips_osc_sequences() {
        // OSC title change: ESC]0;title BEL
        assert_eq!(
            sanitize_control_chars("\x1b]0;evil title\x07normal"),
            "normal"
        );
        // OSC with ST terminator: ESC]0;title ESC\
        assert_eq!(
            sanitize_control_chars("\x1b]0;evil title\x1b\\normal"),
            "normal"
        );
    }

    #[test]
    fn sanitize_strips_bare_esc() {
        assert_eq!(sanitize_control_chars("before\x1bafter"), "beforeafter");
    }

    #[test]
    fn sanitize_strips_c0_controls() {
        // BEL, BS, etc. should be stripped
        assert_eq!(sanitize_control_chars("a\x07b\x08c"), "abc");
    }

    #[test]
    fn sanitize_preserves_unicode() {
        assert_eq!(sanitize_control_chars("日本語 🦆"), "日本語 🦆");
    }

    #[test]
    fn sanitize_strips_del() {
        assert_eq!(sanitize_control_chars("a\x7fb"), "ab");
    }

    #[test]
    fn sanitize_strips_c1_controls() {
        // U+0085 (NEL), U+008D (RI), U+009B (CSI) — all in C1 range
        assert_eq!(sanitize_control_chars("a\u{0085}b\u{009B}c"), "abc");
    }

    #[test]
    fn sanitize_strips_csi_non_alpha_terminators() {
        // ESC[200~ (bracketed paste) and ESC[1@ (insert char) — final bytes outside A-Za-z
        assert_eq!(sanitize_control_chars("a\x1b[200~b"), "ab");
        assert_eq!(sanitize_control_chars("a\x1b[1@b"), "ab");
    }
}
