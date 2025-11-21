use crate::app::App;
use crate::ui::theme::Theme;

pub enum CommandResult {
    Continue,
    Quit,
    Reply(String),
    OpenBuilder,
}

pub fn dispatch(app: &mut App, input: &str) -> CommandResult {
    let parts: Vec<&str> = input.trim_start_matches('/').split_whitespace().collect();
    if parts.is_empty() {
        return CommandResult::Continue;
    }

    let command = parts[0];
    let args = &parts[1..];

    match command {
        "exit" | "quit" => CommandResult::Quit,
        "clear" => {
            app.messages.clear();
            CommandResult::Continue
        }
        "help" => {
            app.showing_help_popup = !app.showing_help_popup;
            CommandResult::Continue
        }
        "about" => {
            app.showing_about_popup = !app.showing_about_popup;
            CommandResult::Continue
        }
        "theme" => {
            if let Some(name) = args.first() {
                app.config.theme = Theme::from_name(name);
            }
            CommandResult::Continue
        }
        "alias" | "create-command" => CommandResult::OpenBuilder,
        _ => {
            // Check custom commands
            if let Some(cmd) = app
                .config
                .custom_commands
                .iter()
                .find(|c| c.name == command)
            {
                // Construct a prompt that asks Goose to run this tool
                let args_str = cmd
                    .args
                    .as_object()
                    .map(|obj| {
                        obj.iter()
                            .map(|(k, v)| format!("{}: {}", k, v))
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default();

                let prompt = format!(
                    "Please run the tool '{}' with these arguments: {{{}}}",
                    cmd.tool, args_str
                );

                CommandResult::Reply(prompt)
            } else {
                CommandResult::Reply(input.to_string())
            }
        }
    }
}
