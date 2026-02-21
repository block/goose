use crossterm::style::Color;
use crossterm::{cursor, execute, terminal};
use std::io::{self, Write};
use std::sync::{Arc, RwLock};

const BAR_HEIGHT: u16 = 2;

#[derive(Clone)]
pub struct StatusBarState {
    pub model_name: String,
    pub provider_name: String,
    pub total_tokens: usize,
    pub context_limit: usize,
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub cost_usd: Option<f64>,
    pub extension_count: usize,
    pub goose_mode: String,
    pub is_processing: bool,
    pub session_id: String,
}

impl Default for StatusBarState {
    fn default() -> Self {
        Self {
            model_name: String::new(),
            provider_name: String::new(),
            total_tokens: 0,
            context_limit: 0,
            input_tokens: 0,
            output_tokens: 0,
            cost_usd: None,
            extension_count: 0,
            goose_mode: "auto".to_string(),
            is_processing: false,
            session_id: String::new(),
        }
    }
}

pub struct StatusBar {
    state: Arc<RwLock<StatusBarState>>,
    active: bool,
}

impl StatusBar {
    pub fn new(initial_state: StatusBarState) -> Self {
        Self {
            state: Arc::new(RwLock::new(initial_state)),
            active: false,
        }
    }

    /// Set up the scroll region to reserve the bottom lines for the status bar
    pub fn setup(&mut self) -> io::Result<()> {
        let (_, rows) = terminal::size()?;
        let scroll_end = rows.saturating_sub(BAR_HEIGHT);

        // Set scroll region to exclude the bottom BAR_HEIGHT lines
        // CSI n ; m r — set scrolling region from row n to row m (1-indexed)
        write!(io::stdout(), "\x1b[1;{}r", scroll_end)?;

        // Move cursor to the top of the scroll region
        execute!(io::stdout(), cursor::MoveTo(0, 0))?;

        self.active = true;
        self.render()?;
        Ok(())
    }

    /// Reset scroll region and clean up the status bar area
    pub fn teardown(&mut self) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }
        self.active = false;

        // Reset scroll region to full terminal
        write!(io::stdout(), "\x1b[r")?;

        // Clear the status bar area
        let (_, rows) = terminal::size()?;
        let bar_start = rows.saturating_sub(BAR_HEIGHT);
        for row in bar_start..rows {
            execute!(io::stdout(), cursor::MoveTo(0, row))?;
            write!(io::stdout(), "\x1b[2K")?; // clear line
        }

        io::stdout().flush()?;
        Ok(())
    }

    /// Temporarily expand scroll region for interactive prompts (cliclack, rustyline)
    pub fn pause(&mut self) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }
        // Reset scroll region to full terminal
        write!(io::stdout(), "\x1b[r")?;

        // Clear the status bar lines
        let (_, rows) = terminal::size()?;
        let bar_start = rows.saturating_sub(BAR_HEIGHT);
        for row in bar_start..rows {
            execute!(io::stdout(), cursor::MoveTo(0, row))?;
            write!(io::stdout(), "\x1b[2K")?;
        }

        // Move cursor up to where content should go
        execute!(io::stdout(), cursor::MoveTo(0, bar_start))?;
        io::stdout().flush()?;
        Ok(())
    }

    /// Restore scroll region and re-render the status bar after interactive prompt
    pub fn resume(&mut self) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }
        let (_, rows) = terminal::size()?;
        let scroll_end = rows.saturating_sub(BAR_HEIGHT);

        // Restore scroll region
        write!(io::stdout(), "\x1b[1;{}r", scroll_end)?;

        // Position cursor within the scroll region
        execute!(
            io::stdout(),
            cursor::MoveTo(0, scroll_end.saturating_sub(1))
        )?;

        self.render()?;
        Ok(())
    }

    /// Render the status bar in the reserved area
    pub fn render(&self) -> io::Result<()> {
        if !self.active {
            return Ok(());
        }

        let (cols, rows) = terminal::size()?;
        let bar_start = rows.saturating_sub(BAR_HEIGHT);
        let width = cols as usize;

        let state = self.state.read().unwrap();

        // Build the content segments for the status bar
        let segments = build_segments(&state, width);

        // Render line 1: top border
        let border_line = build_border_line(width);

        // Render line 2: content + bottom border chars
        let content_line = build_content_line(&segments, width);

        // Save cursor position
        write!(io::stdout(), "\x1b[s")?;

        // Move to the status bar area and write
        execute!(io::stdout(), cursor::MoveTo(0, bar_start))?;
        write!(io::stdout(), "\x1b[2K")?; // clear line
        write_styled_line(&border_line)?;

        execute!(io::stdout(), cursor::MoveTo(0, bar_start + 1))?;
        write!(io::stdout(), "\x1b[2K")?; // clear line
        write_styled_line(&content_line)?;

        // Restore cursor position
        write!(io::stdout(), "\x1b[u")?;
        io::stdout().flush()?;

        Ok(())
    }

    pub fn update_state<F>(&self, updater: F) -> io::Result<()>
    where
        F: FnOnce(&mut StatusBarState),
    {
        {
            let mut state = self.state.write().unwrap();
            updater(&mut state);
        }
        self.render()
    }

    pub fn set_processing(&self, processing: bool) -> io::Result<()> {
        self.update_state(|s| s.is_processing = processing)
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}

// --- Rendering helpers ---

#[derive(Clone)]
struct StyledSpan {
    text: String,
    fg: Option<Color>,
    bold: bool,
    dim: bool,
}

impl StyledSpan {
    fn plain(text: &str) -> Self {
        Self {
            text: text.to_string(),
            fg: None,
            bold: false,
            dim: false,
        }
    }

    fn colored(text: &str, fg: Color) -> Self {
        Self {
            text: text.to_string(),
            fg: Some(fg),
            bold: false,
            dim: false,
        }
    }

    fn dim(text: &str) -> Self {
        Self {
            text: text.to_string(),
            fg: None,
            bold: false,
            dim: true,
        }
    }

    fn bold_colored(text: &str, fg: Color) -> Self {
        Self {
            text: text.to_string(),
            fg: Some(fg),
            bold: true,
            dim: false,
        }
    }

    fn visible_len(&self) -> usize {
        self.text.len()
    }
}

fn build_segments(state: &StatusBarState, _width: usize) -> Vec<StyledSpan> {
    let mut segments: Vec<StyledSpan> = Vec::new();

    // Model name
    if !state.model_name.is_empty() {
        segments.push(StyledSpan::bold_colored(&state.model_name, Color::Cyan));
    }

    // Token usage bar
    if state.context_limit > 0 {
        let percentage = ((state.total_tokens as f64 / state.context_limit as f64) * 100.0)
            .round() as usize;
        let percentage = percentage.min(100);

        let bar_width = 15;
        let filled = ((percentage as f64 / 100.0) * bar_width as f64).round() as usize;
        let empty = bar_width - filled.min(bar_width);

        let bar = format!("{}{}", "━".repeat(filled), "╌".repeat(empty));
        let bar_color = if percentage < 50 {
            Color::Green
        } else if percentage < 85 {
            Color::Yellow
        } else {
            Color::Red
        };

        segments.push(StyledSpan::colored(&bar, bar_color));
        segments.push(StyledSpan::dim(&format!(
            " {}% {}/{}",
            percentage,
            format_tokens(state.total_tokens),
            format_tokens(state.context_limit)
        )));
    }

    // Cost
    if let Some(cost) = state.cost_usd {
        segments.push(StyledSpan::colored(
            &format!("${:.2}", cost),
            Color::Yellow,
        ));
    }

    // Extension count
    if state.extension_count > 0 {
        segments.push(StyledSpan::dim(&format!("{} ext", state.extension_count)));
    }

    // Mode
    let mode_color = match state.goose_mode.as_str() {
        "auto" => Color::Green,
        "approve" | "smart_approve" => Color::Yellow,
        "chat" => Color::Blue,
        _ => Color::White,
    };
    segments.push(StyledSpan::colored(&state.goose_mode, mode_color));

    // Processing indicator
    if state.is_processing {
        segments.push(StyledSpan::colored("⟳", Color::Yellow));
    }

    segments
}

fn build_border_line(width: usize) -> Vec<StyledSpan> {
    let inner = "─".repeat(width.saturating_sub(2));
    vec![StyledSpan::dim(&format!("╭{}╮", inner))]
}

fn build_content_line(segments: &[StyledSpan], width: usize) -> Vec<StyledSpan> {
    let mut result: Vec<StyledSpan> = Vec::new();
    result.push(StyledSpan::dim("│ "));

    let separator = " │ ";
    let mut content_width: usize = 2; // "│ " prefix

    for (i, seg) in segments.iter().enumerate() {
        let sep_len = if i > 0 { separator.len() } else { 0 };
        let needed = sep_len + seg.visible_len();

        // Reserve 2 chars for " │" suffix
        if content_width + needed + 2 > width {
            break;
        }

        if i > 0 {
            result.push(StyledSpan::dim(separator));
            content_width += sep_len;
        }
        result.push(seg.clone());
        content_width += seg.visible_len();
    }

    // Pad to width and close border
    let remaining = width.saturating_sub(content_width + 2);
    if remaining > 0 {
        result.push(StyledSpan::plain(&" ".repeat(remaining)));
    }
    result.push(StyledSpan::dim(" │"));

    result
}

fn write_styled_line(spans: &[StyledSpan]) -> io::Result<()> {
    use crossterm::style::{Attribute, Print, ResetColor, SetAttribute, SetForegroundColor};

    let mut stdout = io::stdout();
    for span in spans {
        if span.dim {
            execute!(stdout, SetAttribute(Attribute::Dim))?;
        }
        if span.bold {
            execute!(stdout, SetAttribute(Attribute::Bold))?;
        }
        if let Some(fg) = span.fg {
            execute!(stdout, SetForegroundColor(fg))?;
        }
        execute!(stdout, Print(&span.text))?;
        execute!(stdout, ResetColor)?;
        execute!(stdout, SetAttribute(Attribute::Reset))?;
    }
    Ok(())
}

fn format_tokens(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.0}k", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_tokens() {
        assert_eq!(format_tokens(500), "500");
        assert_eq!(format_tokens(1_000), "1k");
        assert_eq!(format_tokens(42_000), "42k");
        assert_eq!(format_tokens(128_000), "128k");
        assert_eq!(format_tokens(1_000_000), "1.0M");
        assert_eq!(format_tokens(1_500_000), "1.5M");
    }

    #[test]
    fn test_status_bar_state_default() {
        let state = StatusBarState::default();
        assert_eq!(state.total_tokens, 0);
        assert_eq!(state.context_limit, 0);
        assert!(!state.is_processing);
        assert_eq!(state.goose_mode, "auto");
    }

    #[test]
    fn test_build_segments_empty_model() {
        let state = StatusBarState::default();
        let segments = build_segments(&state, 80);
        // Should have at least the mode segment
        assert!(!segments.is_empty());
    }

    #[test]
    fn test_build_segments_with_data() {
        let state = StatusBarState {
            model_name: "gpt-4o".to_string(),
            provider_name: "openai".to_string(),
            total_tokens: 42_000,
            context_limit: 128_000,
            cost_usd: Some(0.12),
            extension_count: 3,
            goose_mode: "auto".to_string(),
            is_processing: false,
            ..Default::default()
        };
        let segments = build_segments(&state, 80);
        // Model, bar, bar info, cost, extensions, mode
        assert!(segments.len() >= 5);
    }

    #[test]
    fn test_build_border_line() {
        let line = build_border_line(20);
        assert_eq!(line.len(), 1);
        assert!(line[0].text.starts_with('╭'));
        assert!(line[0].text.ends_with('╮'));
    }

    #[test]
    fn test_build_content_line() {
        let segments = vec![StyledSpan::plain("hello"), StyledSpan::plain("world")];
        let line = build_content_line(&segments, 30);
        // Should have prefix, segments, separator, padding, suffix
        assert!(line.len() >= 4);
    }
}
