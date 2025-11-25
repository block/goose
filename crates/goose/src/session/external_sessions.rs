use crate::config::Config;
use crate::session::extension_data::ExtensionData;
use crate::session::session_manager::{Session, SessionType};

fn is_enabled() -> bool {
    #[cfg(test)]
    return false;

    #[cfg(not(test))]
    Config::global()
        .get_goose_enable_external_sessions()
        .ok()
        .unwrap_or(false)
}

pub fn get_external_session(id: &str, include_messages: bool) -> Option<Session> {
    if !is_enabled() {
        return None;
    }

    if let Some(session) = get_claude_code_session(id, include_messages) {
        return Some(session);
    }

    get_codex_session(id, include_messages)
}

fn get_claude_code_session(id: &str, include_messages: bool) -> Option<Session> {
    let sessions = crate::session::claude_code::list_claude_code_sessions().ok()?;

    for (session_id, working_dir, updated_at) in sessions {
        if session_id == id {
            let conversation = if include_messages {
                crate::session::claude_code::load_claude_code_session(id).ok()
            } else {
                None
            };

            let message_count = conversation
                .as_ref()
                .map(|c| c.messages().len())
                .unwrap_or(0);

            return Some(Session {
                id: session_id.clone(),
                working_dir,
                name: format!(
                    "Claude Code Session {}",
                    session_id.chars().take(8).collect::<String>()
                ),
                user_set_name: false,
                session_type: SessionType::User,
                created_at: updated_at,
                updated_at,
                extension_data: ExtensionData::default(),
                total_tokens: None,
                input_tokens: None,
                output_tokens: None,
                accumulated_total_tokens: None,
                accumulated_input_tokens: None,
                accumulated_output_tokens: None,
                schedule_id: None,
                recipe: None,
                user_recipe_values: None,
                conversation,
                message_count,
                provider_name: None,
                model_config: None,
            });
        }
    }
    None
}

fn get_codex_session(id: &str, include_messages: bool) -> Option<Session> {
    let sessions = crate::session::codex::list_codex_sessions().ok()?;

    for (session_id, working_dir, updated_at) in sessions {
        if session_id == id {
            let conversation = if include_messages {
                crate::session::codex::load_codex_session(id).ok()
            } else {
                None
            };

            let message_count = conversation
                .as_ref()
                .map(|c| c.messages().len())
                .unwrap_or(0);

            return Some(Session {
                id: session_id.clone(),
                working_dir,
                name: format!(
                    "Codex Session {}",
                    session_id.chars().take(8).collect::<String>()
                ),
                user_set_name: false,
                session_type: SessionType::User,
                created_at: updated_at,
                updated_at,
                extension_data: ExtensionData::default(),
                total_tokens: None,
                input_tokens: None,
                output_tokens: None,
                accumulated_total_tokens: None,
                accumulated_input_tokens: None,
                accumulated_output_tokens: None,
                schedule_id: None,
                recipe: None,
                user_recipe_values: None,
                conversation,
                message_count,
                provider_name: None,
                model_config: None,
            });
        }
    }
    None
}

pub fn get_external_sessions_for_list() -> Vec<Session> {
    let enabled = is_enabled();
    tracing::debug!("External sessions enabled: {}", enabled);

    if !enabled {
        return Vec::new();
    }

    let mut sessions = Vec::new();

    match crate::session::claude_code::list_claude_code_sessions() {
        Ok(claude_sessions) => {
            tracing::debug!("Found {} Claude Code sessions", claude_sessions.len());
            for (session_id, working_dir, updated_at) in claude_sessions {
                sessions.push(Session {
                    id: session_id.clone(),
                    working_dir,
                    name: format!(
                        "Claude Code Session {}",
                        session_id.chars().take(8).collect::<String>()
                    ),
                    user_set_name: false,
                    session_type: SessionType::User,
                    created_at: updated_at,
                    updated_at,
                    extension_data: ExtensionData::default(),
                    total_tokens: None,
                    input_tokens: None,
                    output_tokens: None,
                    accumulated_total_tokens: None,
                    accumulated_input_tokens: None,
                    accumulated_output_tokens: None,
                    schedule_id: None,
                    recipe: None,
                    user_recipe_values: None,
                    conversation: None,
                    message_count: 0,
                    provider_name: None,
                    model_config: None,
                });
            }
        }
        Err(e) => {
            tracing::debug!("Failed to list Claude Code sessions: {}", e);
        }
    }

    match crate::session::codex::list_codex_sessions() {
        Ok(codex_sessions) => {
            tracing::debug!("Found {} Codex sessions", codex_sessions.len());
            for (session_id, working_dir, updated_at) in codex_sessions {
                sessions.push(Session {
                    id: session_id.clone(),
                    working_dir,
                    name: format!(
                        "Codex Session {}",
                        session_id.chars().take(8).collect::<String>()
                    ),
                    user_set_name: false,
                    session_type: SessionType::User,
                    created_at: updated_at,
                    updated_at,
                    extension_data: ExtensionData::default(),
                    total_tokens: None,
                    input_tokens: None,
                    output_tokens: None,
                    accumulated_total_tokens: None,
                    accumulated_input_tokens: None,
                    accumulated_output_tokens: None,
                    schedule_id: None,
                    recipe: None,
                    user_recipe_values: None,
                    conversation: None,
                    message_count: 0,
                    provider_name: None,
                    model_config: None,
                });
            }
        }
        Err(e) => {
            tracing::debug!("Failed to list Codex sessions: {}", e);
        }
    }

    tracing::debug!("Returning {} external sessions", sessions.len());
    sessions
}
