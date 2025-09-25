use crate::agents::types::SessionConfig;
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::session::extension_data::ExtensionState;
use crate::session::{self, TodoState};
use chrono::Local;

async fn build_moim_content(session: &Option<SessionConfig>) -> Option<String> {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let mut content = format!("Current date and time: {}\n", timestamp);

    if let Some(todo_content) = get_todo_context(session).await {
        content.push_str("\nCurrent tasks and notes:\n");
        content.push_str(&todo_content);
        content.push('\n');
    }

    Some(content)
}

pub async fn inject_moim_if_enabled(
    messages_for_provider: Conversation,
    session: &Option<SessionConfig>,
) -> Conversation {
    // Check if MOIM is enabled
    let moim_enabled = crate::config::Config::global()
        .get_param::<bool>("goose_moim_enabled")
        .unwrap_or(true);

    if !moim_enabled {
        return messages_for_provider;
    }

    if let Some(moim_content) = build_moim_content(session).await {
        let mut msgs = messages_for_provider.messages().to_vec();
        let moim_msg = Message::user().with_text(moim_content);

        if msgs.is_empty() {
            // If conversation is empty, just add the MOIM
            msgs.push(moim_msg);
        } else {
            // Insert at position -1 (before last message)
            // This ensures MOIM appears just before the latest user query
            let insert_pos = msgs.len().saturating_sub(1);
            msgs.insert(insert_pos, moim_msg);
        }

        Conversation::new_unvalidated(msgs)
    } else {
        messages_for_provider
    }
}

async fn get_todo_context(session: &Option<SessionConfig>) -> Option<String> {
    let session_config = session.as_ref()?;

    match session::storage::get_path(session_config.id.clone()) {
        Ok(path) => match session::storage::read_metadata(&path) {
            Ok(metadata) => TodoState::from_extension_data(&metadata.extension_data)
                .map(|state| state.content)
                .filter(|content| !content.trim().is_empty()),
            Err(e) => {
                tracing::debug!("Could not read session metadata for MOIM: {}", e);
                None
            }
        },
        Err(e) => {
            tracing::debug!("Could not get session path for MOIM: {}", e);
            None
        }
    }
}
