use std::time::Duration;

use crate::display;
use crate::display::ToolOutputMode;

/// How the CLI behaves. Determines rendering, cancellation, and permission handling.
/// Three variants, zero boolean flags in the stream loop.
pub(crate) enum SessionMode {
    /// Full TUI: markdown streaming, raw-key cancel, interactive permissions, spinner.
    Rich {
        streamer: Box<display::markdown::MarkdownStreamer>,
        tool_output_mode: ToolOutputMode,
        spinner_frame: usize,
    },
    /// Interactive but plain text (--plain-stream, NO_COLOR, TERM=dumb).
    Plain { tool_output_mode: ToolOutputMode },
    /// Non-interactive: piped stdin, one-shot, recipe.
    Pipe,
}

impl SessionMode {
    pub fn rich() -> Self {
        Self::Rich {
            streamer: Box::new(display::markdown::MarkdownStreamer::new()),
            tool_output_mode: ToolOutputMode::from_env(),
            spinner_frame: 0,
        }
    }

    pub fn plain() -> Self {
        Self::Plain {
            tool_output_mode: ToolOutputMode::from_env(),
        }
    }

    pub fn pipe() -> Self {
        Self::Pipe
    }

    // --- Behavioral ---

    pub fn is_interactive(&self) -> bool {
        !matches!(self, Self::Pipe)
    }

    pub fn uses_raw_mode(&self) -> bool {
        matches!(self, Self::Rich { .. })
    }

    // --- Rendering ---

    pub fn render_agent_text(&mut self, text: &str) {
        let clean = display::sanitize(text);
        match self {
            Self::Rich { streamer, .. } => streamer.push(&clean),
            Self::Plain { .. } | Self::Pipe => display::print_plain_text(&clean),
        }
    }

    pub fn render_thinking(&mut self, text: &str) {
        if let Self::Rich { streamer, .. } = self {
            streamer.finish_if_active();
            display::clear_spinner();
            let clean = display::sanitize(text);
            display::print_thinking(&clean);
        }
    }

    pub fn render_tool_start(&mut self, title: &str, input: Option<&serde_json::Value>) {
        if self.uses_raw_mode() {
            self.finish_if_active();
            display::clear_spinner();
        }
        display::print_tool_start(title, input, self.uses_raw_mode());
    }

    pub fn render_tool_complete(
        &mut self,
        title: &str,
        elapsed: Duration,
        args: Option<&str>,
        number: Option<usize>,
    ) {
        if self.uses_raw_mode() {
            display::clear_spinner();
        }
        display::print_tool_complete(title, elapsed, args, number, self.is_interactive());
    }

    pub fn render_tool_failed(&mut self, title: &str, elapsed: Duration, number: Option<usize>) {
        if self.uses_raw_mode() {
            display::clear_spinner();
        }
        display::print_tool_failed(title, elapsed, number, self.is_interactive());
    }

    pub fn render_tool_output(&mut self, content: &str) {
        let (mode, rich) = match self {
            Self::Rich {
                tool_output_mode, ..
            } => (*tool_output_mode, true),
            Self::Plain { tool_output_mode } => (*tool_output_mode, false),
            Self::Pipe => return,
        };
        display::print_tool_output(content, mode, rich);
    }

    /// Finalize the markdown streamer (Rich only). No-op for other modes.
    pub fn finish(&mut self) {
        if let Self::Rich { streamer, .. } = self {
            streamer.finish();
        }
    }

    /// Finalize only if the streamer is currently active. No-op for other modes.
    pub fn finish_if_active(&mut self) {
        if let Self::Rich { streamer, .. } = self {
            streamer.finish_if_active();
        }
    }

    pub fn update_spinner(&mut self) {
        if let Self::Rich { spinner_frame, .. } = self {
            display::update_spinner(*spinner_frame);
            *spinner_frame += 1;
        }
    }

    pub fn clear_spinner_and_finish(&mut self) {
        if let Self::Rich { .. } = self {
            display::clear_spinner();
            self.finish();
        }
    }

    /// Clear spinner and reset frame counter. Called when all active tools complete.
    pub fn on_tools_empty(&mut self) {
        if let Self::Rich { spinner_frame, .. } = self {
            display::clear_spinner();
            *spinner_frame = 0;
        }
    }

    // --- Permission ---

    pub fn render_permission_prompt(&mut self, title: &str, input: Option<&serde_json::Value>) {
        // Only Rich mode renders an interactive permission prompt.
        if matches!(self, Self::Rich { .. }) {
            crate::permissions::render_permission_prompt(title, input);
        }
    }
}
