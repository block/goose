use anyhow::Result;
use rustyline::Editor;

#[derive(Debug)]
pub enum InputResult {
    Message(String),
    Exit,
    AddExtension(String),
    AddBuiltin(String),
    ToggleTheme,
    Retry,
}

pub fn get_input(
    editor: &mut Editor<(), rustyline::history::DefaultHistory>,
) -> Result<InputResult> {
    // Ensure Ctrl-J binding is set for newlines
    editor.bind_sequence(
        rustyline::KeyEvent(rustyline::KeyCode::Char('j'), rustyline::Modifiers::CTRL),
        rustyline::EventHandler::Simple(rustyline::Cmd::Newline),
    );

    let prompt = format!("{} ", console::style("( O)>").cyan().bold());
    let input = match editor.readline(&prompt) {
        Ok(text) => text,
        Err(e) => match e {
            rustyline::error::ReadlineError::Interrupted => return Ok(InputResult::Exit),
            _ => return Err(e.into()),
        },
    };

    // Add valid input to history
    if !input.trim().is_empty() {
        editor.add_history_entry(input.as_str())?;
    }

    // Handle non-slash commands first
    if !input.starts_with('/') {
        if input.eq_ignore_ascii_case("exit") || input.eq_ignore_ascii_case("quit") {
            return Ok(InputResult::Exit);
        }
        return Ok(InputResult::Message(input.trim().to_string()));
    }

    // Handle slash commands
    match handle_slash_command(&input) {
        Some(result) => Ok(result),
        None => Ok(InputResult::Message(input.trim().to_string())),
    }
}

fn handle_slash_command(input: &str) -> Option<InputResult> {
    let input = input.trim();

    match input {
        "/exit" | "/quit" => Some(InputResult::Exit),
        "/?" | "/help" => {
            print_help();
            Some(InputResult::Retry)
        }
        "/t" => Some(InputResult::ToggleTheme),
        s if s.starts_with("/extension ") => Some(InputResult::AddExtension(s[11..].to_string())),
        s if s.starts_with("/builtin ") => Some(InputResult::AddBuiltin(s[9..].to_string())),
        _ => None,
    }
}

fn print_help() {
    println!(
        "Available commands:
/exit or /quit - Exit the session
/t - Toggle Light/Dark/Ansi theme
/extension <command> - Add a stdio extension (format: ENV1=val1 command args...)
/builtin <names> - Add builtin extensions by name (comma-separated)
/? or /help - Display this help message

Navigation:
Ctrl+C - Interrupt goose (resets the interaction to before the interrupted user request)
Ctrl+J - Add a newline
Up/Down arrows - Navigate through command history"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_slash_command() {
        // Test exit commands
        assert!(matches!(
            handle_slash_command("/exit"),
            Some(InputResult::Exit)
        ));
        assert!(matches!(
            handle_slash_command("/quit"),
            Some(InputResult::Exit)
        ));

        // Test help commands
        assert!(matches!(
            handle_slash_command("/help"),
            Some(InputResult::Retry)
        ));
        assert!(matches!(
            handle_slash_command("/?"),
            Some(InputResult::Retry)
        ));

        // Test theme toggle
        assert!(matches!(
            handle_slash_command("/t"),
            Some(InputResult::ToggleTheme)
        ));

        // Test extension command
        if let Some(InputResult::AddExtension(cmd)) = handle_slash_command("/extension foo bar") {
            assert_eq!(cmd, "foo bar");
        } else {
            panic!("Expected AddExtension");
        }

        // Test builtin command
        if let Some(InputResult::AddBuiltin(names)) = handle_slash_command("/builtin dev,git") {
            assert_eq!(names, "dev,git");
        } else {
            panic!("Expected AddBuiltin");
        }

        // Test unknown commands
        assert!(handle_slash_command("/unknown").is_none());
    }

    // Test whitespace handling
    #[test]
    fn test_whitespace_handling() {
        // Leading/trailing whitespace in extension command
        if let Some(InputResult::AddExtension(cmd)) = handle_slash_command("  /extension foo bar  ")
        {
            assert_eq!(cmd, "foo bar");
        } else {
            panic!("Expected AddExtension");
        }

        // Leading/trailing whitespace in builtin command
        if let Some(InputResult::AddBuiltin(names)) = handle_slash_command("  /builtin dev,git  ") {
            assert_eq!(names, "dev,git");
        } else {
            panic!("Expected AddBuiltin");
        }
    }
}
