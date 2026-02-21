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
    pub project_name: String,
    pub git_branch: Option<String>,
    pub git_dirty: bool,
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
            project_name: String::new(),
            git_branch: None,
            git_dirty: false,
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

        // Build the two flat status lines
        let line1 = build_line_1(&state, width);
        let line2 = build_line_2(&state, width);

        // Save cursor position
        write!(io::stdout(), "\x1b[s")?;

        // Move to the status bar area and write
        execute!(io::stdout(), cursor::MoveTo(0, bar_start))?;
        write!(io::stdout(), "\x1b[2K")?; // clear line
        write_styled_line(&line1)?;

        execute!(io::stdout(), cursor::MoveTo(0, bar_start + 1))?;
        write!(io::stdout(), "\x1b[2K")?; // clear line
        write_styled_line(&line2)?;

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
}

/// Build line 1: 🪿 model | 📁 project | 🌿 branch ● | ⚡ pct% · tokens
fn build_line_1(state: &StatusBarState, _width: usize) -> Vec<StyledSpan> {
    let mut spans: Vec<StyledSpan> = Vec::new();
    spans.push(StyledSpan::plain("  "));

    // Model
    if !state.model_name.is_empty() {
        spans.push(StyledSpan::dim("\u{1fabf} "));
        spans.push(StyledSpan::bold_colored(&state.model_name, Color::Cyan));
    }

    // Project name
    if !state.project_name.is_empty() {
        spans.push(StyledSpan::dim(" | "));
        spans.push(StyledSpan::dim("\u{1f4c1} "));
        spans.push(StyledSpan::dim(&state.project_name));
    }

    // Git branch
    if let Some(ref branch) = state.git_branch {
        spans.push(StyledSpan::dim(" | "));
        spans.push(StyledSpan::dim("\u{1f33f} "));
        spans.push(StyledSpan::colored(branch, Color::Green));
        if state.git_dirty {
            spans.push(StyledSpan::colored(" \u{25cf}", Color::Yellow));
        }
    }

    // Token usage
    if state.context_limit > 0 {
        let pct = ((state.total_tokens as f64 / state.context_limit as f64) * 100.0).round()
            as usize;
        let pct = pct.min(100);
        let token_color = if pct < 50 {
            Color::Green
        } else if pct < 85 {
            Color::Yellow
        } else {
            Color::Red
        };
        spans.push(StyledSpan::dim(" | "));
        spans.push(StyledSpan::colored(
            &format!(
                "\u{26a1} {}% \u{00b7} {}/{} tokens",
                pct,
                format_tokens(state.total_tokens),
                format_tokens(state.context_limit)
            ),
            token_color,
        ));
    }

    spans
}

/// Build line 2: ⏵ mode · N extensions [· $cost] [⟳]
fn build_line_2(state: &StatusBarState, _width: usize) -> Vec<StyledSpan> {
    let mut spans: Vec<StyledSpan> = Vec::new();
    spans.push(StyledSpan::plain("  "));

    // Mode indicator
    let mode_color = match state.goose_mode.as_str() {
        "auto" => Color::Green,
        "approve" | "smart_approve" => Color::Yellow,
        "chat" => Color::Blue,
        _ => Color::White,
    };
    spans.push(StyledSpan::colored("\u{23f5} ", mode_color));
    spans.push(StyledSpan::colored(
        &format!("{} mode", state.goose_mode),
        mode_color,
    ));

    // Extension count
    if state.extension_count > 0 {
        let ext_label = if state.extension_count == 1 {
            "extension"
        } else {
            "extensions"
        };
        spans.push(StyledSpan::dim(&format!(
            " \u{00b7} {} {}",
            state.extension_count, ext_label
        )));
    }

    // Cost
    if let Some(cost) = state.cost_usd {
        spans.push(StyledSpan::dim(" \u{00b7} "));
        spans.push(StyledSpan::colored(&format!("${:.2}", cost), Color::Yellow));
    }

    // Processing indicator
    if state.is_processing {
        spans.push(StyledSpan::plain(" "));
        spans.push(StyledSpan::colored("\u{27f3}", Color::Yellow));
    }

    spans
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
        assert!(state.git_branch.is_none());
        assert!(!state.git_dirty);
        assert!(state.project_name.is_empty());
    }

    #[test]
    fn test_build_line_1_with_model() {
        let state = StatusBarState {
            model_name: "gpt-4o".to_string(),
            project_name: "goose".to_string(),
            git_branch: Some("main".to_string()),
            git_dirty: true,
            total_tokens: 42_000,
            context_limit: 128_000,
            ..Default::default()
        };
        let line = build_line_1(&state, 120);
        // Should have padding + model + project + git + tokens spans
        assert!(line.len() >= 5);
    }

    #[test]
    fn test_build_line_2_with_mode() {
        let state = StatusBarState {
            goose_mode: "auto".to_string(),
            extension_count: 3,
            cost_usd: Some(0.12),
            is_processing: true,
            ..Default::default()
        };
        let line = build_line_2(&state, 80);
        // Should have padding + mode icon + mode text + extensions + cost + processing
        assert!(line.len() >= 4);
    }

    #[test]
    fn test_build_line_1_empty_state() {
        let state = StatusBarState::default();
        let line = build_line_1(&state, 80);
        // Should at least have the padding span
        assert!(!line.is_empty());
    }
}
