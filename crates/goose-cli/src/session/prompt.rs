/// Returns a system prompt extension that explains CLI-specific functionality
pub fn get_cli_prompt() -> String {
    let newline_key = super::input::get_newline_key().to_ascii_uppercase();
    format!(
        "You are being accessed through a command-line interface. The following slash commands are available
- you can let the user know about them if they need help:

- /exit or /quit - Exit the session
- /t - Toggle between Light/Dark/Ansi themes
- /? or /help - Display help message

Additional keyboard shortcuts:
- Ctrl+C - Interrupt the current interaction (resets to before the interrupted request)
- Ctrl+{newline_key} - Add a newline
- Up/Down arrows - Navigate command history"
    )
}
