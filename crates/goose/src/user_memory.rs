use crate::config::paths::Paths;
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::providers::base::Provider;
use crate::utils::safe_truncate;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

const MEMORY_FILE: &str = "user_memory.json";
const MAX_FACTS_WORDS: usize = 200;
const MIN_MESSAGES_FOR_EXTRACTION: usize = 4;
const MAX_MESSAGE_CHARS: usize = 500;

fn truncate_message(text: &str) -> String {
    safe_truncate(text, MAX_MESSAGE_CHARS)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserMemory {
    pub facts: String,
    pub updated_at: Option<String>,
}

fn memory_path() -> PathBuf {
    Paths::in_state_dir(MEMORY_FILE)
}

pub fn load_user_memory() -> Option<UserMemory> {
    let path = memory_path();
    let data = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&data).ok()
}

fn save_user_memory(memory: &UserMemory) -> Result<()> {
    let path = memory_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(memory)?;
    std::fs::write(path, data)?;
    Ok(())
}

fn sample_conversation_text(conversation: &Conversation) -> String {
    let messages = conversation.messages();
    if messages.len() < MIN_MESSAGES_FOR_EXTRACTION {
        return String::new();
    }

    // Take first few and last few messages to capture the topic and conclusion
    let first_n = 4.min(messages.len());
    let last_n = 4.min(messages.len());
    let skip_last = messages.len().saturating_sub(last_n);

    let mut parts = Vec::new();

    for msg in messages.iter().take(first_n) {
        let role = match msg.role {
            rmcp::model::Role::User => "User",
            rmcp::model::Role::Assistant => "Assistant",
        };
        let text = msg.as_concat_text();
        if !text.is_empty() {
            let truncated = truncate_message(&text);
            parts.push(format!("{}: {}", role, truncated));
        }
    }

    if skip_last > first_n {
        parts.push("[... middle of conversation omitted ...]".to_string());
        for msg in messages.iter().skip(skip_last) {
            let role = match msg.role {
                rmcp::model::Role::User => "User",
                rmcp::model::Role::Assistant => "Assistant",
            };
            let text = msg.as_concat_text();
            if !text.is_empty() {
                let truncated = truncate_message(&text);
                parts.push(format!("{}: {}", role, truncated));
            }
        }
    }

    parts.join("\n\n")
}

pub async fn extract_and_update_memory(
    provider: Arc<dyn Provider>,
    conversation: Conversation,
) -> Result<()> {
    let sample = sample_conversation_text(&conversation);
    if sample.is_empty() {
        return Ok(());
    }

    let existing = load_user_memory().map(|m| m.facts).unwrap_or_default();

    let system = "You maintain a concise user profile from conversations. \
        Output ONLY the updated facts, nothing else. \
        Keep it under 200 words. Use short bullet points. \
        Categories: identity/role, projects/tech stack, preferences/style, key topics. \
        Merge new info with existing facts. Drop outdated or trivial info. \
        If the conversation reveals nothing personal or useful, return the existing facts unchanged.";

    let user_text = if existing.is_empty() {
        format!(
            "Extract key facts about this user from the conversation below.\n\n\
            ---CONVERSATION---\n{}\n---END---",
            sample
        )
    } else {
        format!(
            "Update these existing user facts with any new info from the conversation.\n\n\
            ---EXISTING FACTS---\n{}\n---END EXISTING---\n\n\
            ---CONVERSATION---\n{}\n---END---",
            existing, sample
        )
    };

    let message = Message::user().with_text(&user_text);
    let result = provider
        .complete_fast("user-memory", system, &[message], &[])
        .await;

    match result {
        Ok((response, _usage)) => {
            let facts: String = response
                .content
                .iter()
                .filter_map(|c| c.as_text())
                .collect();

            let facts = truncate_to_word_limit(&facts, MAX_FACTS_WORDS);

            if !facts.trim().is_empty() {
                let memory = UserMemory {
                    facts,
                    updated_at: Some(chrono::Utc::now().to_rfc3339()),
                };
                save_user_memory(&memory)?;
            }
        }
        Err(e) => {
            tracing::warn!("Failed to extract user memory: {}", e);
        }
    }

    Ok(())
}

fn truncate_to_word_limit(text: &str, max_words: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.len() <= max_words {
        text.to_string()
    } else {
        words[..max_words].join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_to_word_limit() {
        assert_eq!(truncate_to_word_limit("hello world", 10), "hello world");
        assert_eq!(truncate_to_word_limit("a b c d e", 3), "a b c");
        assert_eq!(truncate_to_word_limit("", 5), "");
    }

    #[test]
    fn test_sample_conversation_text_too_short() {
        let conv = Conversation::new_unvalidated(vec![
            Message::user().with_text("hi"),
            Message::assistant().with_text("hello"),
        ]);
        assert!(sample_conversation_text(&conv).is_empty());
    }

    #[test]
    fn test_sample_conversation_text_enough_messages() {
        let conv = Conversation::new_unvalidated(vec![
            Message::user().with_text("I work on rust projects"),
            Message::assistant().with_text("Great!"),
            Message::user().with_text("Help me with cargo"),
            Message::assistant().with_text("Sure thing"),
        ]);
        let sample = sample_conversation_text(&conv);
        assert!(sample.contains("rust projects"));
        assert!(sample.contains("cargo"));
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("user_memory.json");

        let memory = UserMemory {
            facts: "User is a Rust developer".to_string(),
            updated_at: Some("2025-01-01T00:00:00Z".to_string()),
        };

        let data = serde_json::to_string_pretty(&memory).unwrap();
        std::fs::write(&path, data).unwrap();

        let loaded: UserMemory =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(loaded.facts, "User is a Rust developer");
        assert_eq!(loaded.updated_at.as_deref(), Some("2025-01-01T00:00:00Z"));
    }

    #[test]
    fn test_user_memory_serialization() {
        let memory = UserMemory {
            facts: "- Works on goose\n- Prefers Rust".to_string(),
            updated_at: None,
        };
        let json = serde_json::to_string(&memory).unwrap();
        let parsed: UserMemory = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.facts, memory.facts);
        assert!(parsed.updated_at.is_none());
    }
}
