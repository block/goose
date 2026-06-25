use console::Term;
use std::sync::Once;

static SIGINT_HANDLER: Once = Once::new();

/// Restores cursor visibility after cliclack/dialoguer TUI prompts.
pub fn restore_interactive_terminal() {
    let _ = Term::stdout().show_cursor();
}

/// RAII guard for interactive TUI flows such as `goose configure`.
///
/// cliclack hides the cursor during prompts; if the user presses Ctrl+C the
/// process can exit before the library restores it. This guard shows the cursor
/// on normal return and registers a SIGINT handler for interrupt exits.
pub struct InteractiveTerminalGuard {
    _private: (),
}

impl InteractiveTerminalGuard {
    pub fn new() -> Self {
        SIGINT_HANDLER.call_once(|| {
            ctrlc::set_handler(|| {
                restore_interactive_terminal();
                std::process::exit(130);
            })
            .expect("failed to set Ctrl+C handler");
        });
        Self { _private: () }
    }
}

impl Default for InteractiveTerminalGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for InteractiveTerminalGuard {
    fn drop(&mut self) {
        restore_interactive_terminal();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restore_interactive_terminal_does_not_panic() {
        restore_interactive_terminal();
    }
}
