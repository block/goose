use anyhow::Result;
use nostr_sdk::prelude::*;
use nostr_sdk::{
    Client, Event, EventBuilder, EventId, Filter, Keys, Kind, Tag, TagKind, Timestamp,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

use crate::config::ModelConfig;

pub const LLM_SERVICE_KIND: u16 = 31990;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PublishedModel {
    name: String,
    endpoint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    context_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cost: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    geo: Option<String>,
}

impl From<&ModelConfig> for PublishedModel {
    fn from(m: &ModelConfig) -> Self {
        Self {
            name: m.name.clone(),
            endpoint: m.endpoint.clone(),
            display_name: m.display_name.clone(),
            description: m.description.clone(),
            context_size: m.context_size,
            cost: m.cost,
            geo: m.geo.clone(),
        }
    }
}

pub struct ModelPublisher {
    client: Client,
    keys: Keys,
}

impl ModelPublisher {
    pub async fn new(keys: Keys, relays: Vec<String>) -> Result<Self> {
        let client = Client::new(keys.clone());
        for relay in &relays {
            client.add_relay(relay).await?;
        }
        Ok(Self { client, keys })
    }

    pub async fn connect(&self) {
        self.client.connect().await;
    }

    pub fn npub(&self) -> String {
        self.keys.public_key().to_bech32().unwrap_or_default()
    }

    pub async fn publish(&self, models: &[ModelConfig], ttl_seconds: u64) -> Result<EventId> {
        let expiration = Timestamp::now().as_secs() + ttl_seconds;

        let published_models: Vec<PublishedModel> = models.iter().map(|m| m.into()).collect();
        let content = serde_json::json!({
            "models": published_models,
            "api_type": "openai_compatible",
        });

        let tags = vec![
            Tag::custom(TagKind::Custom("d".into()), vec!["llm-offerings".to_string()]),
            Tag::custom(
                TagKind::Custom("k".into()),
                vec!["llm-openai-compatible".to_string()],
            ),
            Tag::custom(
                TagKind::Custom("expiration".into()),
                vec![expiration.to_string()],
            ),
        ];

        let builder =
            EventBuilder::new(Kind::Custom(LLM_SERVICE_KIND), content.to_string()).tags(tags);

        let output = self.client.send_event_builder(builder).await?;
        Ok(*output.id())
    }

    pub async fn list_own_events(&self) -> Result<Vec<Event>> {
        let filter = Filter::new()
            .kind(Kind::Custom(LLM_SERVICE_KIND))
            .author(self.keys.public_key())
            .limit(50);
        let events = self
            .client
            .fetch_events(filter, Duration::from_secs(10))
            .await?;
        Ok(events.into_iter().collect())
    }

    pub async fn clear_old_events(&self) -> Result<usize> {
        let events = self.list_own_events().await?;
        let mut deleted = 0;
        for event in events {
            // Delete old per-model events (have "model" tag) 
            let has_model_tag = event
                .tags
                .iter()
                .any(|t| t.as_slice().first().map(|s| s.as_str()) == Some("model"));
            if has_model_tag {
                let _ = self.delete_event(&event.id).await;
                deleted += 1;
            }
        }
        Ok(deleted)
    }

    pub async fn delete_event(&self, event_id: &EventId) -> Result<EventId> {
        let request = EventDeletionRequest::new().id(*event_id);
        let delete_builder = EventBuilder::delete(request);
        let output = self.client.send_event_builder(delete_builder).await?;
        Ok(*output.id())
    }
}

#[derive(Debug, Clone)]
pub struct DiscoveredModel {
    pub model_name: String,
    pub display_name: Option<String>,
    pub endpoint: String,
    pub description: Option<String>,
    pub context_size: Option<u32>,
    pub cost: Option<f64>,
    pub geo: Option<String>,
    pub publisher_npub: String,
    pub expires_at: Option<u64>,
}

impl DiscoveredModel {
    pub fn is_expired(&self) -> bool {
        if let Some(exp) = self.expires_at {
            exp < Timestamp::now().as_secs()
        } else {
            false
        }
    }
}

pub struct ModelDiscovery {
    client: Client,
}

impl ModelDiscovery {
    pub async fn new(relays: Vec<String>) -> Result<Self> {
        let keys = Keys::generate();
        let client = Client::new(keys);
        for relay in &relays {
            client.add_relay(relay).await?;
        }
        Ok(Self { client })
    }

    pub async fn connect(&self) {
        self.client.connect().await;
    }

    pub async fn discover(&self) -> Result<Vec<DiscoveredModel>> {
        let filter = Filter::new()
            .kind(Kind::Custom(LLM_SERVICE_KIND))
            .custom_tag(
                SingleLetterTag::lowercase(Alphabet::K),
                "llm-openai-compatible".to_string(),
            )
            .limit(100);
        let events = self
            .client
            .fetch_events(filter, Duration::from_secs(10))
            .await?;

        // Dedupe by publisher+d_tag (keep latest per publisher)
        let mut latest: HashMap<(String, String), (u64, &Event)> = HashMap::new();
        for event in events.iter() {
            let d_tag = event
                .tags
                .iter()
                .find(|t| t.as_slice().first().map(|s| s.as_str()) == Some("d"))
                .and_then(|t| t.as_slice().get(1))
                .map(|s| s.to_string())
                .unwrap_or_default();

            let pubkey = event.pubkey.to_hex();
            let key = (pubkey, d_tag);
            let created = event.created_at.as_secs();

            if let Some((existing_ts, _)) = latest.get(&key) {
                if created > *existing_ts {
                    latest.insert(key, (created, event));
                }
            } else {
                latest.insert(key, (created, event));
            }
        }

        // Parse models from each event's content
        let mut models = Vec::new();
        for (_, event) in latest.values() {
            let expires_at = event
                .tags
                .iter()
                .find(|t| t.as_slice().first().map(|s| s.as_str()) == Some("expiration"))
                .and_then(|t| t.as_slice().get(1))
                .and_then(|s| s.parse::<u64>().ok());

            // Skip expired events
            if let Some(exp) = expires_at {
                if exp < Timestamp::now().as_secs() {
                    continue;
                }
            }

            let publisher_npub = event.pubkey.to_bech32().unwrap_or_default();

            if let Ok(content) = serde_json::from_str::<serde_json::Value>(&event.content) {
                if let Some(model_array) = content.get("models").and_then(|m| m.as_array()) {
                    for m in model_array {
                        let model = DiscoveredModel {
                            model_name: m
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            display_name: m
                                .get("display_name")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            endpoint: m
                                .get("endpoint")
                                .and_then(|v| v.as_str())
                                .unwrap_or_default()
                                .to_string(),
                            description: m
                                .get("description")
                                .and_then(|v| v.as_str())
                                .map(String::from),
                            context_size: m
                                .get("context_size")
                                .and_then(|v| v.as_u64())
                                .map(|v| v as u32),
                            cost: m.get("cost").and_then(|v| v.as_f64()),
                            geo: m.get("geo").and_then(|v| v.as_str()).map(String::from),
                            publisher_npub: publisher_npub.clone(),
                            expires_at,
                        };
                        if !model.model_name.is_empty() && !model.endpoint.is_empty() {
                            models.push(model);
                        }
                    }
                }
            }
        }

        Ok(models)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_kind() {
        assert_eq!(LLM_SERVICE_KIND, 31990);
    }

    #[test]
    fn test_expiration_check() {
        let model = DiscoveredModel {
            model_name: "test".to_string(),
            display_name: None,
            endpoint: "http://localhost:11434".to_string(),
            description: None,
            context_size: None,
            cost: None,
            geo: None,
            publisher_npub: "npub1test".to_string(),
            expires_at: Some(1), // Expired (timestamp 1 is way in the past)
        };
        assert!(model.is_expired());

        let model_future = DiscoveredModel {
            expires_at: Some(Timestamp::now().as_secs() + 3600),
            ..model.clone()
        };
        assert!(!model_future.is_expired());
    }
}
