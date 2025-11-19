use crate::app::App;
use crate::ui::theme::Theme;
use goose::conversation::message::Message;
use rmcp::model::CallToolRequestParam;

pub enum CommandResult {
    Continue,
    Quit,
    Reply(String),
    ExecuteTool(Message),
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
                // Construct Tool Request
                let param = CallToolRequestParam {
                    name: cmd.tool.clone().into(),
                    arguments: Some(cmd.args.as_object().cloned().unwrap_or_default()),
                };

                let id = format!("call_{}", uuid::Uuid::new_v4());

                // Inject Assistant Tool Request
                let msg = Message::assistant().with_tool_request(id, Ok(param));

                CommandResult::ExecuteTool(msg)
            } else {
                CommandResult::Reply(input.to_string())
            }
        }
    }
}
