use super::types::*;
use nostr_sdk::prelude::*;
use serde_json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug)]
pub enum NostrMemoryError {
    NostrError(String),
    #[allow(dead_code)]
    TimeoutError,
    InvalidData(String),
    SerializationError(String),
}

impl std::fmt::Display for NostrMemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            NostrMemoryError::NostrError(e) => write!(f, "Nostr error: {e}"),
            NostrMemoryError::TimeoutError => write!(f, "Operation timed out"),
            NostrMemoryError::InvalidData(e) => write!(f, "Invalid data: {e}"),
            NostrMemoryError::SerializationError(e) => write!(f, "Serialization error: {e}"),
        }
    }
}

impl std::error::Error for NostrMemoryError {}

#[derive(Debug, Clone)]
pub struct NostrMemoryClient {
    client: Client,
    keys: Keys,
    is_connected: Arc<AtomicBool>,
}

impl NostrMemoryClient {
    pub fn new(client: Client, keys: Keys) -> Self {
        Self {
            client,
            keys,
            is_connected: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn ensure_connected(&self) -> Result<(), NostrMemoryError> {
        if self.is_connected.load(Ordering::Relaxed) {
            return Ok(());
        }

        let relays = vec!["wss://nostr.chaima.info"];

        for relay_url in relays {
            match self.client.add_relay(relay_url).await {
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!("Failed to add relay {relay_url}: {e}");
                }
            }
        }

        self.client.connect().await;
        self.is_connected.store(true, Ordering::Relaxed);

        Ok(())
    }

    pub async fn store_memory(&self, memory: &MemoryEntry) -> Result<bool, NostrMemoryError> {
        self.ensure_connected().await?;

        let content = serde_json::to_string(memory)
            .map_err(|e| NostrMemoryError::SerializationError(e.to_string()))?;

        let our_public_key = self.keys.public_key();

        let _event_id = self
            .client
            .send_private_msg(our_public_key, content, [])
            .await
            .map_err(|e| {
                NostrMemoryError::NostrError(format!("Send private message failed: {e}"))
            })?;

        Ok(true)
    }

    pub async fn retrieve_memories(
        &self,
        filter: &RetrieveMemoryRequest,
    ) -> Result<Vec<MemoryEntry>, NostrMemoryError> {
        self.ensure_connected().await?;

        let our_public_key = self.keys.public_key();

        let mut nostr_filter = Filter::new().kind(Kind::GiftWrap).pubkey(our_public_key);

        if let Some(since_str) = &filter.since {
            if let Ok(since_dt) = chrono::DateTime::parse_from_rfc3339(since_str) {
                let timestamp = Timestamp::from(since_dt.timestamp() as u64);
                nostr_filter = nostr_filter.since(timestamp);
            }
        }

        if let Some(until_str) = &filter.until {
            if let Ok(until_dt) = chrono::DateTime::parse_from_rfc3339(until_str) {
                let timestamp = Timestamp::from(until_dt.timestamp() as u64);
                nostr_filter = nostr_filter.until(timestamp);
            }
        }

        let timeout = std::time::Duration::from_secs(10);
        let events = self
            .client
            .fetch_events(nostr_filter, timeout)
            .await
            .map_err(|e| NostrMemoryError::NostrError(e.to_string()))?;

        let mut memories = Vec::new();
        let mut deleted_memory_ids = std::collections::HashSet::new();

        for event in events {
            if event.kind == Kind::GiftWrap {
                if let Ok(unwrapped_gift) = self.client.unwrap_gift_wrap(&event).await {
                    if unwrapped_gift.rumor.kind == Kind::PrivateDirectMessage
                        && unwrapped_gift.sender == our_public_key
                    {
                        if unwrapped_gift.rumor.content.starts_with("MEMORY_DELETED:") {
                            let memory_id = unwrapped_gift
                                .rumor
                                .content
                                .strip_prefix("MEMORY_DELETED:")
                                .unwrap();
                            deleted_memory_ids.insert(memory_id.to_string());
                        } else if let Ok(memory) =
                            serde_json::from_str::<MemoryEntry>(&unwrapped_gift.rumor.content)
                        {
                            if self.matches_filter(&memory, filter) {
                                memories.push(memory);
                            }
                        }
                    }
                }
            }
        }

        memories.retain(|memory| !deleted_memory_ids.contains(&memory.id));
        memories.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(memories)
    }

    pub async fn delete_memory(&self, memory_id: &str) -> Result<bool, NostrMemoryError> {
        self.ensure_connected().await?;

        let deletion_content = format!("MEMORY_DELETED:{memory_id}");
        let our_public_key = self.keys.public_key();

        self.client
            .send_private_msg(our_public_key, deletion_content, [])
            .await
            .map_err(|e| NostrMemoryError::NostrError(e.to_string()))?;

        Ok(true)
    }

    pub async fn update_memory(
        &self,
        memory_id: &str,
        update: &UpdateMemoryRequest,
    ) -> Result<MemoryEntry, NostrMemoryError> {
        let retrieve_filter = RetrieveMemoryRequest {
            query: None,
            memory_type: None,
            category: None,
            tags: None,
            limit: Some(1000),
            since: None,
            until: None,
        };

        let memories = self.retrieve_memories(&retrieve_filter).await?;

        let mut existing_memory = memories
            .into_iter()
            .find(|m| m.id == memory_id)
            .ok_or_else(|| NostrMemoryError::InvalidData("Memory not found".to_string()))?;

        if let Some(title) = &update.title {
            existing_memory.content.title = title.clone();
        }
        if let Some(description) = &update.description {
            existing_memory.content.description = description.clone();
        }
        if let Some(tags) = &update.tags {
            existing_memory.content.metadata.tags = tags.clone();
        }
        if let Some(priority) = &update.priority {
            existing_memory.content.metadata.priority = Some(priority.clone());
        }
        if let Some(expiry_str) = &update.expiry {
            if let Ok(expiry_dt) = chrono::DateTime::parse_from_rfc3339(expiry_str) {
                existing_memory.content.metadata.expiry =
                    Some(expiry_dt.with_timezone(&chrono::Utc));
            }
        }

        existing_memory.timestamp = chrono::Utc::now();
        self.store_memory(&existing_memory).await?;

        Ok(existing_memory)
    }

    pub async fn get_memory_stats(&self) -> Result<MemoryStats, NostrMemoryError> {
        let retrieve_filter = RetrieveMemoryRequest {
            query: None,
            memory_type: None,
            category: None,
            tags: None,
            limit: Some(10000),
            since: None,
            until: None,
        };

        let memories = self.retrieve_memories(&retrieve_filter).await?;

        let mut by_type = std::collections::HashMap::new();
        let mut by_category = std::collections::HashMap::new();
        let mut oldest = None;
        let mut newest = None;

        for memory in &memories {
            *by_type.entry(memory.memory_type.clone()).or_insert(0) += 1;

            if let Some(category) = &memory.category {
                *by_category.entry(category.clone()).or_insert(0) += 1;
            }

            if oldest.is_none() || memory.timestamp < oldest.unwrap() {
                oldest = Some(memory.timestamp);
            }
            if newest.is_none() || memory.timestamp > newest.unwrap() {
                newest = Some(memory.timestamp);
            }
        }

        Ok(MemoryStats {
            total_memories: memories.len(),
            by_type,
            by_category,
            oldest,
            newest,
        })
    }

    fn matches_filter(&self, memory: &MemoryEntry, filter: &RetrieveMemoryRequest) -> bool {
        if memory.is_expired() {
            return false;
        }

        if let Some(query) = &filter.query {
            if !memory.matches_query(query) {
                return false;
            }
        }

        if let Some(filter_type) = &filter.memory_type {
            if &memory.memory_type != filter_type {
                return false;
            }
        }

        if let Some(filter_category) = &filter.category {
            match &memory.category {
                Some(memory_category) => {
                    if memory_category != filter_category {
                        return false;
                    }
                }
                None => return false,
            }
        }

        if let Some(filter_tags) = &filter.tags {
            for filter_tag in filter_tags {
                if !memory.tags.contains(filter_tag) {
                    return false;
                }
            }
        }

        true
    }
}
