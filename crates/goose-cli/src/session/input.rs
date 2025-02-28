use anyhow::Result;
use rustyline::Editor;
use shlex;
use std::collections::HashMap;

use super::completion::GooseCompleter;

#[derive(Debug)]
pub enum InputResult {
    Message(String),
    Exit,
    AddExtension(String),
    AddBuiltin(String),
    ToggleTheme,
    Retry,
    ListPrompts(Option<String>),
    PromptCommand(PromptCommandOptions),
}

#[derive(Debug)]
pub struct PromptCommandOptions {
    pub name: String,
    pub info: bool,
    pub arguments: HashMap<String, String>,
}

pub fn get_input(
    editor: &mut Editor<GooseCompleter, rustyline::history::DefaultHistory>,
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

    // Add valid input to history (history saving to file is handled in the Session::interactive method)
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
        "/prompts" => Some(InputResult::ListPrompts(None)),
        s if s.starts_with("/prompts ") => {
            // Parse arguments for /prompts command
            let args = s.strip_prefix("/prompts ").unwrap_or_default();
            parse_prompts_command(args)
        }
        s if s.starts_with("/prompt") => {
            if s == "/prompt" {
                // No arguments case
                Some(InputResult::PromptCommand(PromptCommandOptions {
                    name: String::new(), // Empty name will trigger the error message in the rendering
                    info: false,
                    arguments: HashMap::new(),
                }))
            } else if let Some(stripped) = s.strip_prefix("/prompt ") {
                // Has arguments case
                parse_prompt_command(stripped)
            } else {
                // Handle invalid cases like "/promptxyz"
                None
            }
        }
        s if s.starts_with("/extension ") => Some(InputResult::AddExtension(s[11..].to_string())),
        s if s.starts_with("/builtin ") => Some(InputResult::AddBuiltin(s[9..].to_string())),
        _ => None,
    }
}

fn parse_prompts_command(args: &str) -> Option<InputResult> {
    let parts: Vec<String> = shlex::split(args).unwrap_or_default();

    // Look for --extension flag
    for i in 0..parts.len() {
        if parts[i] == "--extension" && i + 1 < parts.len() {
            // Return the extension name that follows the flag
            return Some(InputResult::ListPrompts(Some(parts[i + 1].clone())));
        }
    }

    // If we got here, there was no valid --extension flag
    Some(InputResult::ListPrompts(None))
}

fn parse_prompt_command(args: &str) -> Option<InputResult> {
    let parts: Vec<String> = shlex::split(args).unwrap_or_default();

    // set name to empty and error out in the rendering
    let mut options = PromptCommandOptions {
        name: parts.first().cloned().unwrap_or_default(),
        info: false,
        arguments: HashMap::new(),
    };

    // handle info at any point in the command
    if parts.iter().any(|part| part == "--info") {
        options.info = true;
    }

    // Parse remaining arguments
    let mut i = 1;

    while i < parts.len() {
        let part = &parts[i];

        // Skip flag arguments
        if part == "--info" {
            i += 1;
            continue;
        }

        // Process key=value pairs - removed redundant contains check
        if let Some((key, value)) = part.split_once('=') {
            options.arguments.insert(key.to_string(), value.to_string());
        }

        i += 1;
    }

    Some(InputResult::PromptCommand(options))
}

fn print_help() {
    println!(
        "Available commands:
/exit or /quit - Exit the session
/t - Toggle Light/Dark/Ansi theme
/extension <command> - Add a stdio extension (format: ENV1=val1 command args...)
/builtin <names> - Add builtin extensions by name (comma-separated)
/prompts [--extension <name>] - List all available prompts, optionally filtered by extension
/prompt <n> [--info] [key=value...] - Get prompt info or execute a prompt
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

    #[test]
    fn test_prompts_command() {
        // Test basic prompts command
        if let Some(InputResult::ListPrompts(extension)) = handle_slash_command("/prompts") {
            assert!(extension.is_none());
        } else {
            panic!("Expected ListPrompts");
        }

        // Test prompts with extension filter
        if let Some(InputResult::ListPrompts(extension)) =
            handle_slash_command("/prompts --extension test")
        {
            assert_eq!(extension, Some("test".to_string()));
        } else {
            panic!("Expected ListPrompts with extension");
        }
    }

    #[test]
    fn test_prompt_command() {
        // Test basic prompt info command
        if let Some(InputResult::PromptCommand(opts)) =
            handle_slash_command("/prompt test-prompt --info")
        {
            assert_eq!(opts.name, "test-prompt");
            assert!(opts.info);
            assert!(opts.arguments.is_empty());
        } else {
            panic!("Expected PromptCommand");
        }

        // Test prompt with arguments
        if let Some(InputResult::PromptCommand(opts)) =
            handle_slash_command("/prompt test-prompt arg1=val1 arg2=val2")
        {
            assert_eq!(opts.name, "test-prompt");
            assert!(!opts.info);
            assert_eq!(opts.arguments.len(), 2);
            assert_eq!(opts.arguments.get("arg1"), Some(&"val1".to_string()));
            assert_eq!(opts.arguments.get("arg2"), Some(&"val2".to_string()));
        } else {
            panic!("Expected PromptCommand");
        }
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

    // Test prompt with no arguments
    #[test]
    fn test_prompt_no_args() {
        // Test just "/prompt" with no arguments
        if let Some(InputResult::PromptCommand(opts)) = handle_slash_command("/prompt") {
            assert_eq!(opts.name, "");
            assert!(!opts.info);
            assert!(opts.arguments.is_empty());
        } else {
            panic!("Expected PromptCommand");
        }

        // Test invalid prompt command
        assert!(handle_slash_command("/promptxyz").is_none());
    }

    // Test quoted arguments
    #[test]
    fn test_quoted_arguments() {
        // Test prompt with quoted arguments
        if let Some(InputResult::PromptCommand(opts)) = handle_slash_command(
            r#"/prompt test-prompt arg1="value with spaces" arg2="another value""#,
        ) {
            assert_eq!(opts.name, "test-prompt");
            assert_eq!(opts.arguments.len(), 2);
            assert_eq!(
                opts.arguments.get("arg1"),
                Some(&"value with spaces".to_string())
            );
            assert_eq!(
                opts.arguments.get("arg2"),
                Some(&"another value".to_string())
            );
        } else {
            panic!("Expected PromptCommand");
        }

        // Test prompt with mixed quoted and unquoted arguments
        if let Some(InputResult::PromptCommand(opts)) = handle_slash_command(
            r#"/prompt test-prompt simple=value quoted="value with \"nested\" quotes""#,
        ) {
            assert_eq!(opts.name, "test-prompt");
            assert_eq!(opts.arguments.len(), 2);
            assert_eq!(opts.arguments.get("simple"), Some(&"value".to_string()));
            assert_eq!(
                opts.arguments.get("quoted"),
                Some(&r#"value with "nested" quotes"#.to_string())
            );
        } else {
            panic!("Expected PromptCommand");
        }
    }

    // Test invalid arguments
    #[test]
    fn test_invalid_arguments() {
        // Test prompt with invalid arguments
        if let Some(InputResult::PromptCommand(opts)) =
            handle_slash_command(r#"/prompt test-prompt valid=value invalid_arg another_invalid"#)
        {
            assert_eq!(opts.name, "test-prompt");
            assert_eq!(opts.arguments.len(), 1);
            assert_eq!(opts.arguments.get("valid"), Some(&"value".to_string()));
            // Invalid arguments are ignored but logged
        } else {
            panic!("Expected PromptCommand");
        }
    }
}
