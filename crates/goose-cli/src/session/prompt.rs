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

/// Returns a system prompt extension for terminal/batch mode
pub fn get_term_prompt() -> String {
    String::from(
        "# Terminal Mode (Asynchronous)

You are running in terminal/batch mode. The user has submitted a request and will review the results later - this is NOT an interactive back-and-forth conversation.

**Execution Strategy:**
1. **Gather first** - Use tools to collect all necessary information before making changes
2. **Act in batch** - Make all required changes/modifications together
3. **Summarize last** - Provide a single, concise final summary explaining:
   - What was requested
   - What actions were taken
   - Key results or outcomes
   - Any issues encountered

**Do NOT:**
- Provide running commentary during tool execution
- Break work into multiple conversational turns
- Explain what you're \"about to do\" - just do it

**Do:**
- Work autonomously to completion
- Front-load all investigation/research
- If clarification is truly needed, investigate first, then ask all questions together at the end
- Batch related changes together
- End with a clear, actionable summary"
    )
}
