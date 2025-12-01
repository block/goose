#[cfg(not(test))]
use crate::config::Config;
use crate::conversation::Conversation;
use crate::session::session_manager::{Session, SessionType};
use chrono::{DateTime, Utc};
use std::path::PathBuf;

fn is_enabled() -> bool {
    #[cfg(test)]
    return false;

    #[cfg(not(test))]
    Config::global()
        .get_goose_enable_external_sessions()
        .ok()
        .unwrap_or(false)
}

enum ExternalSessionSource {
    ClaudeCode,
    Codex,
}

fn create_external_session(
    source: ExternalSessionSource,
    session_id: String,
    working_dir: PathBuf,
    updated_at: DateTime<Utc>,
    conversation: Option<Conversation>,
) -> Session {
    let name_prefix = match source {
        ExternalSessionSource::ClaudeCode => "Claude Code Session",
        ExternalSessionSource::Codex => "Codex Session",
    };
    let short_id: String = session_id.chars().take(8).collect();
    let message_count = conversation
        .as_ref()
        .map(|c| c.messages().len())
        .unwrap_or(0);

    Session {
        id: session_id,
        working_dir,
        name: format!("{} {}", name_prefix, short_id),
        user_set_name: false,
        session_type: SessionType::User,
        created_at: updated_at,
        updated_at,
        conversation,
        message_count,
        ..Default::default()
    }
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

    sessions
        .into_iter()
        .find(|(session_id, _, _)| session_id == id)
        .map(|(session_id, working_dir, updated_at)| {
            let conversation = if include_messages {
                crate::session::claude_code::load_claude_code_session(id).ok()
            } else {
                None
            };
            create_external_session(
                ExternalSessionSource::ClaudeCode,
                session_id,
                working_dir,
                updated_at,
                conversation,
            )
        })
}

fn get_codex_session(id: &str, include_messages: bool) -> Option<Session> {
    let sessions = crate::session::codex::list_codex_sessions().ok()?;

    sessions
        .into_iter()
        .find(|(session_id, _, _)| session_id == id)
        .map(|(session_id, working_dir, updated_at)| {
            let conversation = if include_messages {
                crate::session::codex::load_codex_session(id).ok()
            } else {
                None
            };
            create_external_session(
                ExternalSessionSource::Codex,
                session_id,
                working_dir,
                updated_at,
                conversation,
            )
        })
}

pub fn get_external_sessions_for_list() -> Vec<Session> {
    if !is_enabled() {
        return Vec::new();
    }

    let mut sessions = Vec::new();

    if let Ok(claude_sessions) = crate::session::claude_code::list_claude_code_sessions() {
        for (session_id, working_dir, updated_at) in claude_sessions {
            sessions.push(create_external_session(
                ExternalSessionSource::ClaudeCode,
                session_id,
                working_dir,
                updated_at,
                None,
            ));
        }
    }

    if let Ok(codex_sessions) = crate::session::codex::list_codex_sessions() {
        for (session_id, working_dir, updated_at) in codex_sessions {
            sessions.push(create_external_session(
                ExternalSessionSource::Codex,
                session_id,
                working_dir,
                updated_at,
                None,
            ));
        }
    }

    sessions
}
