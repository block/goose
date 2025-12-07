use super::action::Action;
use crate::state::{
    ActivePopup, AppState, CwdAnalysisState, InputMode, PendingToolConfirmation, TodoItem,
};
use goose::conversation::message::{Message, MessageContent, ToolConfirmationRequest};
use goose::model::ModelConfig;
use goose::providers::base::ModelInfo;
use goose_server::routes::reply::MessageEvent;
use std::sync::Arc;
use std::time::{Duration, Instant};

const FLASH_DURATION_SHORT: Duration = Duration::from_secs(2);
const FLASH_DURATION_NORMAL: Duration = Duration::from_secs(3);
const FLASH_DURATION_LONG: Duration = Duration::from_secs(5);
const FLASH_DURATION_APPROVAL: Duration = Duration::from_secs(10);

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
        Action::Resize => {
            state.needs_refresh = true;
        }
        Action::ServerMessage(msg) => handle_server_message(state, msg),
        Action::ModelsLoaded { provider, models } => handle_models_loaded(state, provider, models),
        Action::ConfigLoaded(config) => handle_config_loaded(state, config),
        Action::CwdAnalysisComplete(result) => {
            state.cwd_analysis = match result {
                Some(s) => {
                    state.flash_message = Some((
                        "✓ Project context loaded".to_string(),
                        Instant::now() + FLASH_DURATION_SHORT,
                    ));
                    CwdAnalysisState::Complete(s)
                }
                None => {
                    state.flash_message = Some((
                        "⚠ Project context unavailable".to_string(),
                        Instant::now() + FLASH_DURATION_SHORT,
                    ));
                    CwdAnalysisState::Failed
                }
            };
        }
        _ => {
            if !handle_data_loaded(state, &action)
                && !handle_chat(state, &action)
                && !handle_ui(state, &action)
                && !handle_misc(state, &action)
            {}
        }
    }
}

fn handle_data_loaded(state: &mut AppState, action: &Action) -> bool {
    match action {
        Action::SessionResumed(session) => {
            let is_new_session = state.session_id != session.id;
            state.session_id = session.id.clone();
            state.messages = session
                .conversation
                .as_ref()
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
            state.active_popup = ActivePopup::None;
            state.is_working = false;
            state.pending_confirmation = None;
            if is_new_session && !state.messages.is_empty() {
                state.cwd_analysis = CwdAnalysisState::Failed;
            }

            let pending = state.messages.last().and_then(|last_msg| {
                extract_tool_confirmation(last_msg).map(|req| PendingToolConfirmation {
                    id: req.id.clone(),
                    tool_name: req.tool_name.clone(),
                    arguments: req.arguments.clone(),
                    security_warning: req.prompt.clone(),
                    message_index: state.messages.len() - 1,
                })
            });

            if pending.is_some() {
                state.pending_confirmation = pending;
                state.is_working = false;
                state.flash_message = Some((
                    "⚠ Tool approval required - press Y to allow, N to deny".to_string(),
                    Instant::now() + FLASH_DURATION_APPROVAL,
                ));
            }

            true
        }
        Action::SessionsListLoaded(sessions) => {
            state.available_sessions = sessions.clone();
            true
        }
        Action::ToolsLoaded(tools) => {
            state.available_tools = tools.clone();
            true
        }
        Action::ProvidersLoaded(providers) => {
            state.providers = providers.clone();
            true
        }
        Action::ExtensionsLoaded(extensions) => {
            state.extensions = extensions.clone();
            true
        }
        _ => false,
    }
}

fn handle_chat(state: &mut AppState, action: &Action) -> bool {
    match action {
        Action::SendMessage(message) => {
            state.messages.push(message.clone());
            state.is_working = true;
            state.has_worked = true;
            true
        }
        Action::Interrupt => {
            state.is_working = false;
            state.flash_message = Some((
                "Interrupted".to_string(),
                Instant::now() + FLASH_DURATION_SHORT,
            ));
            true
        }
        Action::ClearChat => {
            state.messages.clear();
            state.todos.clear();
            state.token_state = goose::conversation::message::TokenState::default();
            state.has_worked = false;
            state.pending_confirmation = None;
            true
        }
        Action::CreateNewSession | Action::ResumeSession(_) | Action::ForkFromMessage(_) => {
            state.is_working = true;
            state.active_popup = ActivePopup::None;
            state.messages.clear();
            state.todos.clear();
            state.pending_confirmation = None;
            true
        }
        Action::ConfirmToolCall { id, approved } => {
            if let Some(pending) = &state.pending_confirmation {
                if pending.id == *id {
                    let tool_name = pending.tool_name.clone();
                    state.pending_confirmation = None;
                    state.is_working = true;
                    let action_str = if *approved { "allowed" } else { "denied" };
                    state.flash_message = Some((
                        format!("Tool {tool_name} {action_str}"),
                        Instant::now() + FLASH_DURATION_SHORT,
                    ));
                }
            }
            true
        }
        Action::Error(e) => {
            state.flash_message =
                Some((format!("Error: {e}"), Instant::now() + FLASH_DURATION_LONG));
            state.is_working = false;
            true
        }
        _ => false,
    }
}

fn handle_ui(state: &mut AppState, action: &Action) -> bool {
    match action {
        Action::ToggleInputMode => {
            state.input_mode = match state.input_mode {
                InputMode::Normal => InputMode::Editing,
                InputMode::Editing => InputMode::Normal,
            };
            true
        }
        Action::ToggleTodo => {
            state.active_popup = if state.active_popup == ActivePopup::Todo {
                ActivePopup::None
            } else {
                ActivePopup::Todo
            };
            true
        }
        Action::ToggleHelp => {
            state.active_popup = if state.active_popup == ActivePopup::Help {
                ActivePopup::None
            } else {
                ActivePopup::Help
            };
            true
        }
        Action::OpenSessionPicker => {
            state.active_popup = ActivePopup::SessionPicker;
            true
        }
        Action::OpenConfig => {
            state.active_popup = ActivePopup::Config;
            true
        }
        Action::ClosePopup => {
            state.active_popup = ActivePopup::None;
            true
        }
        Action::OpenThemePicker => {
            state.active_popup = ActivePopup::ThemePicker;
            true
        }
        Action::OpenMessageInfo(idx) => {
            state.active_popup = ActivePopup::MessageInfo(*idx);
            true
        }
        Action::StartCommandBuilder => {
            state.active_popup = ActivePopup::CommandBuilder;
            true
        }
        Action::ToggleCopyMode => {
            state.copy_mode = !state.copy_mode;
            state.needs_refresh = true;
            true
        }
        _ => false,
    }
}

fn handle_misc(state: &mut AppState, action: &Action) -> bool {
    match action {
        Action::Refresh => {
            state.needs_refresh = true;
            true
        }
        Action::ShowFlash(message) => {
            state.flash_message = Some((message.clone(), Instant::now() + FLASH_DURATION_NORMAL));
            true
        }
        Action::PreviewTheme(name) => {
            state.config.theme = crate::utils::styles::Theme::from_name(name);
            state.needs_refresh = true;
            true
        }
        Action::ChangeTheme(name) => {
            state.config.theme = crate::utils::styles::Theme::from_name(name);
            let _ = state.config.save_theme();
            state.active_popup = ActivePopup::None;
            state.needs_refresh = true;
            true
        }
        Action::SetInputEmpty(is_empty) => {
            state.input_text_is_empty = *is_empty;
            true
        }
        Action::DeleteCustomCommand(name) => {
            state.config.custom_commands.retain(|c| c.name != *name);
            let _ = state.config.save();
            state.flash_message = Some((
                format!("✓ Deleted /{name}"),
                Instant::now() + FLASH_DURATION_NORMAL,
            ));
            true
        }
        Action::SubmitCommandBuilder(cmd, msg) => {
            state.config.custom_commands.retain(|c| c.name != cmd.name);
            state.config.custom_commands.push(cmd.clone());
            let _ = state.config.save();
            state.active_popup = ActivePopup::None;
            state.flash_message = Some((msg.clone(), Instant::now() + FLASH_DURATION_NORMAL));
            true
        }
        Action::UpdateProvider { provider, model } => {
            state.active_provider = Some(provider.clone());
            state.active_model = Some(model.clone());
            state.active_popup = ActivePopup::None;
            true
        }
        _ => false,
    }
}

fn handle_models_loaded(state: &mut AppState, provider: String, models: Vec<String>) {
    if let Some(p) = state.providers.iter_mut().find(|p| p.name == provider) {
        let existing_map: std::collections::HashMap<String, ModelInfo> = p
            .metadata
            .known_models
            .drain(..)
            .map(|m| (m.name.clone(), m))
            .collect();

        let mut new_list = Vec::new();
        for name in models {
            if let Some(info) = existing_map.get(&name) {
                new_list.push(info.clone());
            } else {
                let limit = ModelConfig::new(&name)
                    .map(|c| c.context_limit())
                    .unwrap_or(crate::utils::DEFAULT_CONTEXT_LIMIT);

                new_list.push(ModelInfo::new(name, limit));
            }
        }
        p.metadata.known_models = new_list;
    }
}

fn handle_config_loaded(state: &mut AppState, config: serde_json::Value) {
    if let Some(obj) = config.as_object() {
        if let Some(val) = obj.get("GOOSE_PROVIDER") {
            if let Some(s) = val.as_str() {
                state.active_provider = Some(s.to_string());
            }
        }
        if let Some(val) = obj.get("GOOSE_MODEL") {
            if let Some(s) = val.as_str() {
                state.active_model = Some(s.to_string());
            }
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

            if let Some(todos) = extract_todos_from_message(message) {
                state.todos = todos;
            }

            if let Some(req) = extract_tool_confirmation(message) {
                let msg_idx = state.messages.len();
                set_pending_confirmation(state, req, msg_idx);
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
            state.flash_message = Some((
                "Context compaction complete".to_string(),
                Instant::now() + FLASH_DURATION_LONG,
            ));
        }
        MessageEvent::Error { error } => {
            state.flash_message = Some((
                format!("Server Error: {error}"),
                Instant::now() + FLASH_DURATION_LONG,
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

fn extract_tool_confirmation(message: &Message) -> Option<&ToolConfirmationRequest> {
    message.content.iter().find_map(|c| {
        if let MessageContent::ToolConfirmationRequest(req) = c {
            Some(req)
        } else {
            None
        }
    })
}

fn set_pending_confirmation(
    state: &mut AppState,
    req: &ToolConfirmationRequest,
    message_index: usize,
) {
    state.pending_confirmation = Some(PendingToolConfirmation {
        id: req.id.clone(),
        tool_name: req.tool_name.clone(),
        arguments: req.arguments.clone(),
        security_warning: req.prompt.clone(),
        message_index,
    });
    state.is_working = false;
    state.flash_message = Some((
        "⚠ Tool approval required - press Y to allow, N to deny".to_string(),
        Instant::now() + FLASH_DURATION_APPROVAL,
    ));
}

fn extract_todos_from_message(message: &Message) -> Option<Vec<TodoItem>> {
    for content in &message.content {
        let MessageContent::ToolRequest(req) = content else {
            continue;
        };
        let Ok(tool_call) = &req.tool_call else {
            continue;
        };
        if tool_call.name != "todo__todo_write" {
            continue;
        }
        let Some(args) = &tool_call.arguments else {
            continue;
        };
        let Some(content_str) = args.get("content").and_then(|v| v.as_str()) else {
            continue;
        };

        let mut todos = Vec::new();
        for line in content_str.lines() {
            let trimmed = line.trim();
            if let Some(task) = trimmed.strip_prefix("- [ ] ") {
                todos.push(TodoItem {
                    text: task.to_string(),
                    done: false,
                });
            } else if let Some(task) = trimmed.strip_prefix("- [x] ") {
                todos.push(TodoItem {
                    text: task.to_string(),
                    done: true,
                });
            }
        }

        if !todos.is_empty() {
            return Some(todos);
        }
    }
    None
}
