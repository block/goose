use super::action::Action;
use crate::state::{AppState, InputMode, TodoItem};
use goose::conversation::message::MessageContent;
use goose_server::routes::reply::MessageEvent;
use std::sync::Arc;
use std::time::Instant;

pub fn update(state: &mut AppState, action: Action) {
    match action {
        Action::Tick => {
            if let Some((_, expiry)) = state.flash_message {
                if Instant::now() > expiry {
                    state.flash_message = None;
                }
            }
        }
        Action::Quit => {}
        Action::Resize => {}
        Action::ServerMessage(msg) => handle_server_message(state, msg),
        Action::SessionResumed(session) => {
            state.session_id = session.id;
            state.messages = session
                .conversation
                .map(|c| c.messages().clone())
                .unwrap_or_default();
            state.token_state.total_tokens = session.total_tokens.unwrap_or(0);
            state.token_state.input_tokens = session.input_tokens.unwrap_or(0);
            state.token_state.output_tokens = session.output_tokens.unwrap_or(0);
            state.token_state.accumulated_total_tokens =
                session.accumulated_total_tokens.unwrap_or(0);
            state.token_state.accumulated_input_tokens =
                session.accumulated_input_tokens.unwrap_or(0);
            state.token_state.accumulated_output_tokens =
                session.accumulated_output_tokens.unwrap_or(0);
            state.todos.clear();
            state.showing_session_picker = false;
            state.is_working = false;
        }
        Action::SessionsListLoaded(sessions) => {
            state.available_sessions = sessions;
        }
        Action::ToolsLoaded(tools) => {
            state.available_tools = tools;
        }
        Action::Error(e) => {
            state.flash_message = Some((
                format!("Error: {e}"),
                Instant::now() + std::time::Duration::from_secs(5),
            ));
            state.is_working = false;
        }
        Action::SendMessage(message) => {
            state.messages.push(message);
            state.is_working = true;
            state.has_worked = true;
        }
        Action::Interrupt => {
            state.is_working = false;
            state.flash_message = Some((
                "Interrupted".to_string(),
                Instant::now() + std::time::Duration::from_secs(2),
            ));
        }
        Action::ToggleInputMode => {
            state.input_mode = match state.input_mode {
                InputMode::Normal => InputMode::Editing,
                InputMode::Editing => InputMode::Normal,
            };
        }
        Action::ToggleTodo => state.showing_todo = !state.showing_todo,
        Action::ToggleHelp => state.showing_help = !state.showing_help,
        Action::OpenSessionPicker => state.showing_session_picker = true,
        Action::ResumeSession(_) => {
            state.is_working = true;
        }
        Action::ChangeTheme(name) => {
            state.config.theme = crate::utils::styles::Theme::from_name(&name);
        }
        Action::ClearChat => {
            state.messages.clear();
            state.todos.clear();
            state.token_state = goose::conversation::message::TokenState::default();
            state.has_worked = false;
        }
        Action::OpenMessageInfo(idx) => state.showing_message_info = Some(idx),
        Action::SetInputEmpty(is_empty) => state.input_text_is_empty = is_empty,
        Action::ClosePopup => {
            state.showing_help = false;
            state.showing_todo = false;
            state.showing_session_picker = false;
            state.showing_command_builder = false;
            state.showing_message_info = None;
        }
        Action::DeleteCustomCommand(name) => {
            state.config.custom_commands.retain(|c| c.name != name);
            let _ = state.config.save();
            state.showing_command_builder = false;
        }
        Action::StartCommandBuilder => state.showing_command_builder = true,
        Action::SubmitCommandBuilder(cmd) => {
            state.config.custom_commands.push(cmd);
            let _ = state.config.save();
            state.showing_command_builder = false;
        }
    }
}

fn handle_server_message(state: &mut AppState, msg: Arc<MessageEvent>) {
    match msg.as_ref() {
        MessageEvent::Message {
            message,
            token_state,
        } => {
            state.token_state = token_state.clone();

            for content in &message.content {
                if let MessageContent::ToolRequest(req) = content {
                    if let Ok(tool_call) = &req.tool_call {
                        if tool_call.name == "todo__todo_write" {
                            if let Some(args) = &tool_call.arguments {
                                if let Some(content_val) = args.get("content") {
                                    if let Some(content_str) = content_val.as_str() {
                                        let mut new_todos = Vec::new();
                                        let mut has_todos = false;
                                        for line in content_str.lines() {
                                            let trimmed = line.trim();
                                            if let Some(task) = trimmed.strip_prefix("- [ ] ") {
                                                new_todos.push(TodoItem {
                                                    text: task.to_string(),
                                                    done: false,
                                                });
                                                has_todos = true;
                                            } else if let Some(task) =
                                                trimmed.strip_prefix("- [x] ")
                                            {
                                                new_todos.push(TodoItem {
                                                    text: task.to_string(),
                                                    done: true,
                                                });
                                                has_todos = true;
                                            }
                                        }
                                        if has_todos {
                                            state.todos = new_todos;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if let Some(last_msg) = state.messages.last_mut() {
                if last_msg.id == message.id {
                    for content in message.content.clone() {
                        if let MessageContent::Text(new_text) = &content {
                            if let Some(MessageContent::Text(last_text)) =
                                last_msg.content.last_mut()
                            {
                                last_text.text.push_str(&new_text.text);
                                continue;
                            }
                        }
                        last_msg.content.push(content);
                    }
                } else {
                    state.messages.push(message.clone());
                }
            } else {
                state.messages.push(message.clone());
            }
        }
        MessageEvent::UpdateConversation { conversation } => {
            state.messages = conversation.messages().clone();
        }
        MessageEvent::Error { error } => {
            state.flash_message = Some((
                format!("Server Error: {error}"),
                Instant::now() + std::time::Duration::from_secs(5),
            ));
            state.is_working = false;
        }
        MessageEvent::Finish { token_state, .. } => {
            state.token_state = token_state.clone();
            state.is_working = false;
        }
        _ => {}
    }
}
