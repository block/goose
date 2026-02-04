use anyhow::Result;
use nostr_sdk::prelude::*;
use nostr_sdk::{
    Client, Event, EventBuilder, EventId, Filter, Keys, Kind, Tag, TagKind, Timestamp,
};
use std::time::Duration;

use crate::config::ModelConfig;

pub const LLM_SERVICE_KIND: u16 = 31990;

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

    pub async fn publish_model(&self, model: &ModelConfig, ttl_seconds: u64) -> Result<EventId> {
        let content = serde_json::json!({
            "name": model.display_name.as_ref().unwrap_or(&model.name),
            "model": model.name,
            "endpoint": model.endpoint,
            "api_type": "openai_compatible",
            "description": model.description,
        });

        let expiration = Timestamp::now().as_secs() + ttl_seconds;

        let mut tags = vec![
            Tag::custom(
                TagKind::Custom("d".into()),
                vec![format!("llm-{}", model.name)],
            ),
            Tag::custom(
                TagKind::Custom("k".into()),
                vec!["llm-openai-compatible".to_string()],
            ),
            Tag::custom(TagKind::Custom("model".into()), vec![model.name.clone()]),
            Tag::custom(
                TagKind::Custom("endpoint".into()),
                vec![model.endpoint.clone()],
            ),
            Tag::custom(
                TagKind::Custom("context".into()),
                vec![model.context_size.unwrap_or(32000).to_string()],
            ),
            Tag::custom(
                TagKind::Custom("expiration".into()),
                vec![expiration.to_string()],
            ),
        ];

        if let Some(cost) = model.cost {
            tags.push(Tag::custom(
                TagKind::Custom("cost".into()),
                vec![cost.to_string()],
            ));
        }
        if let Some(ref geo) = model.geo {
            tags.push(Tag::custom(TagKind::Custom("geo".into()), vec![geo.clone()]));
        }

        let builder =
            EventBuilder::new(Kind::Custom(LLM_SERVICE_KIND), content.to_string()).tags(tags);

        let output = self.client.send_event_builder(builder).await?;
        Ok(*output.id())
    }

    pub async fn publish_all(&self, models: &[ModelConfig], ttl_seconds: u64) -> Result<Vec<EventId>> {
        let mut ids = Vec::new();
        for model in models {
            let id = self.publish_model(model, ttl_seconds).await?;
            ids.push(id);
        }
        Ok(ids)
    }

    pub async fn list_own_models(&self) -> Result<Vec<Event>> {
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

    pub async fn clear_all(&self) -> Result<()> {
        let events = self.list_own_models().await?;
        for event in events {
            let _ = self.unpublish(&event.id).await;
        }
        Ok(())
    }

    pub async fn unpublish(&self, event_id: &EventId) -> Result<EventId> {
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
    pub event_id: String,
    pub expires_at: Option<u64>,
}

impl DiscoveredModel {
    fn from_event(event: &Event) -> Result<Self> {
        let mut model_name = String::new();
        let mut endpoint = String::new();
        let mut context_size = None;
        let mut cost = None;
        let mut geo = None;
        let mut expires_at = None;

        for tag in event.tags.iter() {
            let tag_vec: Vec<&str> = tag.as_slice().iter().map(|s| s.as_str()).collect();
            if tag_vec.len() >= 2 {
                match tag_vec[0] {
                    "model" => model_name = tag_vec[1].to_string(),
                    "endpoint" => endpoint = tag_vec[1].to_string(),
                    "context" => context_size = tag_vec[1].parse().ok(),
                    "cost" => cost = tag_vec[1].parse().ok(),
                    "geo" => geo = Some(tag_vec[1].to_string()),
                    "expiration" => expires_at = tag_vec[1].parse().ok(),
                    _ => {}
                }
            }
        }

        let content: serde_json::Value = serde_json::from_str(&event.content).unwrap_or_default();
        let display_name = content
            .get("name")
            .and_then(|v| v.as_str())
            .map(String::from);
        let description = content
            .get("description")
            .and_then(|v| v.as_str())
            .map(String::from);

        Ok(Self {
            model_name,
            display_name,
            endpoint,
            description,
            context_size,
            cost,
            geo,
            publisher_npub: event.pubkey.to_bech32().unwrap_or_default(),
            event_id: event.id.to_hex(),
            expires_at,
        })
    }

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

        // Filter to models published in the last 30 minutes and not expired
        let thirty_mins_ago = Timestamp::now().as_secs() - 1800;
        let models: Vec<DiscoveredModel> = events
            .iter()
            .filter(|e| e.created_at.as_secs() > thirty_mins_ago)
            .filter_map(|e| DiscoveredModel::from_event(e).ok())
            .filter(|m| !m.is_expired())
            .collect();

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
            event_id: "abc123".to_string(),
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
