//! Slash command parsing and dispatch.

use crossterm::{cursor, execute, terminal};

use crate::commands::{self, CommandDef};
use crate::display;

pub(crate) const BUILT_IN_COMMANDS: &[(&str, &str)] = &[
    ("commands", "List all commands"),
    ("help", "Show help"),
    ("clear", "Clear screen"),
    ("theme", "List or set color theme"),
    ("alias", "Create, list, or remove user commands"),
    ("show", "Show full output of a tool call"),
    ("quit", "Exit (or Ctrl+D)"),
    ("exit", "Exit (alias for /quit)"),
];

/// Result of handling a slash command.
pub(crate) enum SlashResult {
    /// Command handled, continue the REPL loop
    Handled { reload_commands: bool },
    /// Not a known slash command, fall through to agent
    NotHandled,
    /// User wants to quit
    Quit,
    /// User command that needs to be sent as a prompt
    SendPrompt(String),
    /// User command that needs a tool call
    ToolCall {
        tool_name: String,
        arg_key: String,
        body: String,
    },
    /// /show command — caller handles with access to tool outputs
    Show(display::ShowRequest),
}

/// Handle built-in slash commands. Pure — no async, no session access.
pub(crate) fn handle_slash_command(trimmed: &str, user_commands: &[CommandDef]) -> SlashResult {
    if trimmed == "/quit" || trimmed == "/exit" {
        return SlashResult::Quit;
    }

    if trimmed == "/help" {
        display::print_help(BUILT_IN_COMMANDS);
        return SlashResult::Handled {
            reload_commands: false,
        };
    }

    if trimmed == "/clear" {
        execute!(
            std::io::stderr(),
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, 0)
        )
        .ok();
        return SlashResult::Handled {
            reload_commands: false,
        };
    }

    if trimmed == "/theme" || trimmed.starts_with("/theme ") {
        let arg = trimmed.strip_prefix("/theme").unwrap_or("").trim();
        handle_theme_command(arg);
        return SlashResult::Handled {
            reload_commands: false,
        };
    }

    if trimmed == "/alias" || trimmed.starts_with("/alias ") {
        let args = trimmed.strip_prefix("/alias").unwrap_or("").trim();
        let mutated = handle_alias_command(args);
        return SlashResult::Handled {
            reload_commands: mutated,
        };
    }

    if trimmed == "/show" || trimmed.starts_with("/show ") {
        let arg = trimmed.strip_prefix("/show").unwrap_or("").trim();
        if arg.is_empty() {
            return SlashResult::Show(display::ShowRequest::List);
        } else if arg == "last" {
            return SlashResult::Show(display::ShowRequest::Last);
        } else if let Ok(n) = arg.parse::<usize>() {
            if n == 0 {
                display::print_hint("Invalid number. Use /show to see available outputs.");
                return SlashResult::Handled {
                    reload_commands: false,
                };
            }
            return SlashResult::Show(display::ShowRequest::ByNumber(n));
        } else {
            display::print_hint("Usage: /show [number|last]");
            return SlashResult::Handled {
                reload_commands: false,
            };
        }
    }

    let Some(rest) = trimmed.strip_prefix('/') else {
        return SlashResult::NotHandled;
    };

    let (cmd_name, cmd_args) = rest.split_once(' ').unwrap_or((rest, ""));

    if cmd_name == "commands" {
        for (name, desc) in BUILT_IN_COMMANDS {
            eprintln!("  /{:<12} {}", name, display::style::dim(desc));
        }
        if !user_commands.is_empty() {
            eprintln!();
        }
        for cmd in user_commands {
            let name = display::sanitize_control_chars(&cmd.name);
            let desc = display::sanitize_control_chars(cmd.description.as_deref().unwrap_or(""));
            eprintln!("  /{:<12} {}", name, display::style::dim(&desc));
        }
        return SlashResult::Handled {
            reload_commands: false,
        };
    }

    if let Some(cmd) = user_commands.iter().find(|c| c.name == cmd_name) {
        let body = commands::substitute_args(&cmd.body, cmd_args);

        if let Some(tool_name) = cmd.tool.clone() {
            let arg_key = cmd
                .argument_name
                .as_deref()
                .unwrap_or("command")
                .to_string();
            return SlashResult::ToolCall {
                tool_name,
                arg_key,
                body,
            };
        } else {
            return SlashResult::SendPrompt(body);
        }
    }

    SlashResult::NotHandled
}

/// Handle the /theme command.
fn handle_theme_command(arg: &str) {
    if arg.is_empty() {
        let current = crate::display::theme::active_theme().name.clone();
        eprintln!("Available themes:");
        for &name in crate::display::theme::BUILT_IN_THEMES {
            let resolved = crate::display::theme::resolve_theme(name);
            if resolved.name == current {
                eprintln!(
                    "  {} {}",
                    display::style::success("●"),
                    display::style::success(&format!("{name} (active)"))
                );
            } else {
                eprintln!("    {}", display::style::dim(name));
            }
        }
    } else if !crate::display::theme::is_known_theme(arg) {
        display::print_hint(&format!(
            "Unknown theme '{arg}'. Use /theme to see available themes."
        ));
    } else {
        let resolved = crate::display::theme::resolve_theme(arg);
        crate::display::theme::set_active_theme(resolved.clone());
        match crate::display::theme::save_theme_preference(arg) {
            Ok(()) => {
                eprintln!("Theme set to {}", display::style::success(&resolved.name));
            }
            Err(e) => {
                eprintln!("Theme applied. Note: failed to save preference: {e}");
            }
        }
    }
}

/// Handle the /alias command.
/// Returns true if the alias set was mutated (create or remove succeeded).
fn handle_alias_command(args: &str) -> bool {
    if args.is_empty() || args == "--list" {
        let cmds = commands::load_commands();
        if cmds.is_empty() {
            display::print_hint("No user commands defined. Create one with /alias <name> <body>");
        } else {
            eprintln!("User commands:");
            for cmd in &cmds {
                let name = display::sanitize_control_chars(&cmd.name);
                let desc =
                    display::sanitize_control_chars(cmd.description.as_deref().unwrap_or(""));
                eprintln!("  /{:<12} {}", name, display::style::dim(&desc));
            }
        }
        false
    } else if let Some(name) = args
        .strip_prefix("--remove ")
        .or_else(|| args.strip_prefix("-r "))
    {
        let name = name.trim().trim_start_matches('/');
        match commands::remove_command(name) {
            Ok(true) => {
                let clean = display::sanitize_control_chars(name);
                eprintln!("Removed /{}", display::style::success(&clean));
                true
            }
            Ok(false) => {
                display::print_hint(&format!("No command /{name} found"));
                false
            }
            Err(e) => {
                display::print_hint(&format!("Failed to remove: {e}"));
                false
            }
        }
    } else {
        let (name, body) = args.split_once(' ').unwrap_or((args, ""));
        let name = name.trim_start_matches('/');
        if body.is_empty() {
            display::print_hint("Usage: /alias <name> <prompt body>");
            false
        } else {
            match commands::create_command(name, body) {
                Ok(()) => {
                    let clean_name = display::sanitize_control_chars(name);
                    let clean_body = display::sanitize_control_chars(body);
                    eprintln!(
                        "Created {} → \"{}\"",
                        display::style::success(&format!("/{clean_name}")),
                        display::style::dim(&clean_body)
                    );
                    true
                }
                Err(e) => {
                    display::print_hint(&format!("Failed to create: {e}"));
                    false
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_commands() -> Vec<CommandDef> {
        vec![]
    }

    fn sample_commands() -> Vec<CommandDef> {
        vec![
            CommandDef {
                name: "deploy".to_string(),
                description: Some("Deploy to prod".to_string()),
                tool: None,
                argument_name: None,
                argument_hint: None,
                body: "deploy $ARGUMENTS".to_string(),
                source: std::path::PathBuf::new(),
            },
            CommandDef {
                name: "run-shell".to_string(),
                description: None,
                tool: Some("developer__shell".to_string()),
                argument_name: Some("command".to_string()),
                argument_hint: None,
                body: "ls $ARGUMENTS".to_string(),
                source: std::path::PathBuf::new(),
            },
        ]
    }

    #[test]
    fn quit_commands() {
        assert!(matches!(
            handle_slash_command("/quit", &no_commands()),
            SlashResult::Quit
        ));
        assert!(matches!(
            handle_slash_command("/exit", &no_commands()),
            SlashResult::Quit
        ));
    }

    #[test]
    fn show_variants() {
        assert!(matches!(
            handle_slash_command("/show", &no_commands()),
            SlashResult::Show(display::ShowRequest::List)
        ));
        assert!(matches!(
            handle_slash_command("/show last", &no_commands()),
            SlashResult::Show(display::ShowRequest::Last)
        ));
        assert!(matches!(
            handle_slash_command("/show 3", &no_commands()),
            SlashResult::Show(display::ShowRequest::ByNumber(3))
        ));
        // /show 0 is invalid — handled inline
        assert!(matches!(
            handle_slash_command("/show 0", &no_commands()),
            SlashResult::Handled {
                reload_commands: false
            }
        ));
    }

    #[test]
    fn user_command_sends_prompt() {
        let cmds = sample_commands();
        match handle_slash_command("/deploy staging", &cmds) {
            SlashResult::SendPrompt(body) => {
                assert_eq!(body, "deploy staging");
            }
            other => panic!("expected SendPrompt, got {other:?}"),
        }
    }

    #[test]
    fn user_command_tool_call() {
        let cmds = sample_commands();
        match handle_slash_command("/run-shell -la", &cmds) {
            SlashResult::ToolCall {
                tool_name,
                arg_key,
                body,
            } => {
                assert_eq!(tool_name, "developer__shell");
                assert_eq!(arg_key, "command");
                assert_eq!(body, "ls -la");
            }
            other => panic!("expected ToolCall, got {other:?}"),
        }
    }

    #[test]
    fn unknown_slash_falls_through() {
        assert!(matches!(
            handle_slash_command("/nonexistent", &no_commands()),
            SlashResult::NotHandled
        ));
    }

    // Debug impl needed for panic messages in tests
    impl std::fmt::Debug for SlashResult {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Handled { reload_commands } => {
                    write!(f, "Handled {{ reload_commands: {reload_commands} }}")
                }
                Self::NotHandled => write!(f, "NotHandled"),
                Self::Quit => write!(f, "Quit"),
                Self::SendPrompt(s) => write!(f, "SendPrompt({s:?})"),
                Self::ToolCall {
                    tool_name,
                    arg_key,
                    body,
                } => write!(f, "ToolCall({tool_name}, {arg_key}, {body:?})"),
                Self::Show(_) => write!(f, "Show(..)"),
            }
        }
    }
}
