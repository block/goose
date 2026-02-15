use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;

use crate::registry::formats::parse_a2a_agent_card;
use crate::registry::manifest::{RegistryEntry, RegistryEntryKind};
use crate::registry::source::RegistrySource;

/// Discovers agents from remote A2A endpoints via `/.well-known/agent-card.json`.
///
/// Per the A2A protocol specification, agents advertise their capabilities by
/// serving an Agent Card at a well-known URL. This source fetches those cards
/// and converts them into `RegistryEntry` items for unified discovery.
pub struct A2aRegistrySource {
    endpoints: Vec<String>,
    client: Client,
}

impl A2aRegistrySource {
    pub fn new(endpoints: Vec<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_default();
        Self { endpoints, client }
    }

    pub fn from_single(endpoint: &str) -> Self {
        Self::new(vec![endpoint.to_string()])
    }

    async fn fetch_agent_card(&self, base_url: &str) -> Result<RegistryEntry> {
        let url = format!(
            "{}/.well-known/agent-card.json",
            base_url.trim_end_matches('/')
        );

        let resp = self
            .client
            .get(&url)
            .header("Accept", "application/json")
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!("A2A agent at {} returned status {}", url, resp.status());
        }

        let body = resp.text().await?;
        let mut entry = parse_a2a_agent_card(&body)?;

        if entry.source_uri.is_none() {
            entry.source_uri = Some(base_url.to_string());
        }

        Ok(entry)
    }

    async fn fetch_all(&self) -> Vec<RegistryEntry> {
        let mut entries = Vec::new();
        for endpoint in &self.endpoints {
            match self.fetch_agent_card(endpoint).await {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    tracing::debug!(endpoint = %endpoint, error = %e, "failed to fetch A2A agent card");
                }
            }
        }
        entries
    }
}

#[async_trait]
impl RegistrySource for A2aRegistrySource {
    fn name(&self) -> &str {
        "a2a"
    }

    async fn search(
        &self,
        query: Option<&str>,
        kind: Option<RegistryEntryKind>,
    ) -> Result<Vec<RegistryEntry>> {
        if let Some(k) = kind {
            if k != RegistryEntryKind::Agent {
                return Ok(Vec::new());
            }
        }

        let mut entries = self.fetch_all().await;

        if let Some(q) = query {
            let q_lower = q.to_lowercase();
            entries.retain(|e| {
                e.name.to_lowercase().contains(&q_lower)
                    || e.description.to_lowercase().contains(&q_lower)
                    || e.tags.iter().any(|t| t.to_lowercase().contains(&q_lower))
            });
        }

        Ok(entries)
    }

    async fn get(
        &self,
        name: &str,
        kind: Option<RegistryEntryKind>,
    ) -> Result<Option<RegistryEntry>> {
        if let Some(k) = kind {
            if k != RegistryEntryKind::Agent {
                return Ok(None);
            }
        }

        let entries = self.fetch_all().await;
        Ok(entries.into_iter().find(|e| e.name == name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_endpoint() {
        let source = A2aRegistrySource::from_single("https://agent.example.com");
        assert_eq!(source.endpoints, vec!["https://agent.example.com"]);
    }

    #[test]
    fn test_multiple_endpoints() {
        let source = A2aRegistrySource::new(vec![
            "https://a.example.com".to_string(),
            "https://b.example.com".to_string(),
        ]);
        assert_eq!(source.endpoints.len(), 2);
    }

    #[tokio::test]
    async fn test_search_non_agent_kind_returns_empty() {
        let source = A2aRegistrySource::new(vec![]);
        let results = source
            .search(None, Some(RegistryEntryKind::Tool))
            .await
            .unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_get_non_agent_kind_returns_none() {
        let source = A2aRegistrySource::new(vec![]);
        let result = source
            .get("test", Some(RegistryEntryKind::Skill))
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_empty_endpoints_returns_empty() {
        let source = A2aRegistrySource::new(vec![]);
        let results = source
            .search(None, Some(RegistryEntryKind::Agent))
            .await
            .unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_fetch_unreachable_endpoint_is_skipped() {
        let source = A2aRegistrySource::from_single("http://192.0.2.1:1");
        let results = source
            .search(None, Some(RegistryEntryKind::Agent))
            .await
            .unwrap();
        assert!(results.is_empty());
    }
}
