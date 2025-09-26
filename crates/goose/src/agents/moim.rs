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

/// Find a safe insertion point for MOIM.
///
/// We want to insert as close to the end as possible, but we must avoid
/// breaking tool call/response pairs. We check if inserting at a position
/// would separate a tool call from its response.
#[doc(hidden)]
pub fn find_safe_insertion_point(messages: &[Message]) -> usize {
    if messages.is_empty() {
        return 0;
    }

    // Default to inserting before the last message
    let last_pos = messages.len() - 1;

    // Check if inserting at last_pos would break a tool pair
    if last_pos > 0 {
        let prev_msg = &messages[last_pos - 1];
        let curr_msg = &messages[last_pos];

        // If previous message has tool calls and current has matching responses,
        // we can't insert between them
        if prev_msg.is_tool_call() && curr_msg.is_tool_response() {
            // Find the next best position (before the tool call)
            return last_pos.saturating_sub(1);
        }
    }

    last_pos
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
            // Find a safe position that won't break tool call/response pairs
            let insert_pos = find_safe_insertion_point(&msgs);
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
