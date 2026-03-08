use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;

use crate::registry::manifest::{
    AgentDetail, RecipeDetail, RegistryEntry, RegistryEntryDetail, RegistryEntryKind, SkillDetail,
    ToolDetail, ToolTransport,
};
use crate::registry::source::RegistrySource;

/// Discovers registry entries from an HTTP index endpoint.
///
/// Supports two discovery modes:
/// 1. Direct index URL: fetches a JSON array of entry descriptors
/// 2. Well-known discovery: fetches `/.well-known/agent.json` from a domain
///
/// The index format follows a simplified ACP-inspired schema where each entry
/// declares its kind, name, and metadata.
pub struct HttpRegistrySource {
    base_url: String,
    client: Client,
}

#[derive(Debug, Deserialize)]
struct IndexEntry {
    name: String,
    kind: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    metadata: serde_json::Value,
}

impl HttpRegistrySource {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: Client::new(),
        }
    }

    pub fn well_known(domain: &str) -> Self {
        let scheme = if domain.starts_with("http") {
            String::new()
        } else {
            "https://".to_string()
        };
        Self::new(&format!("{}{}/.well-known/agent.json", scheme, domain))
    }

    async fn fetch_index(&self) -> Result<Vec<IndexEntry>> {
        let resp = self
            .client
            .get(&self.base_url)
            .header("Accept", "application/json")
            .send()
            .await?;

        if !resp.status().is_success() {
            anyhow::bail!(
                "HTTP registry at {} returned status {}",
                self.base_url,
                resp.status()
            );
        }

        let entries: Vec<IndexEntry> = resp.json().await?;
        Ok(entries)
    }

    fn index_entry_to_registry_entry(&self, idx: &IndexEntry) -> Option<RegistryEntry> {
        let kind = match idx.kind.as_str() {
            "tool" => RegistryEntryKind::Tool,
            "skill" => RegistryEntryKind::Skill,
            "agent" => RegistryEntryKind::Agent,
            "recipe" => RegistryEntryKind::Recipe,
            _ => return None,
        };

        let detail = match kind {
            RegistryEntryKind::Tool => {
                let transport = match idx.metadata.get("transport").and_then(|v| v.as_str()) {
                    Some("streamable_http") => {
                        let uri = idx
                            .metadata
                            .get("uri")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();
                        ToolTransport::StreamableHttp { uri }
                    }
                    _ => {
                        let cmd = idx
                            .metadata
                            .get("cmd")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_string();
                        let args: Vec<String> = idx
                            .metadata
                            .get("args")
                            .and_then(|v| v.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default();
                        ToolTransport::Stdio { cmd, args }
                    }
                };

                let capabilities: Vec<String> = idx
                    .metadata
                    .get("capabilities")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                let env_keys: Vec<String> = idx
                    .metadata
                    .get("env_keys")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();

                RegistryEntryDetail::Tool(ToolDetail {
                    transport,
                    capabilities,
                    env_keys,
                })
            }
            RegistryEntryKind::Skill => {
                let content = idx
                    .metadata
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                RegistryEntryDetail::Skill(SkillDetail {
                    content,
                    builtin: false,
                })
            }
            RegistryEntryKind::Agent => {
                let instructions = idx
                    .metadata
                    .get("instructions")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();
                let model = idx
                    .metadata
                    .get("model")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                RegistryEntryDetail::Agent(Box::new(AgentDetail {
                    instructions,
                    model,
                    recommended_models: Vec::new(),
                    capabilities: Vec::new(),
                    domains: Vec::new(),
                    input_content_types: Vec::new(),
                    output_content_types: Vec::new(),
                    required_extensions: Vec::new(),
                    dependencies: Vec::new(),
                    ..Default::default()
                }))
            }
            RegistryEntryKind::Recipe => {
                let prompt = idx
                    .metadata
                    .get("prompt")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let instructions = idx
                    .metadata
                    .get("instructions")
                    .and_then(|v| v.as_str())
                    .map(String::from);

                RegistryEntryDetail::Recipe(RecipeDetail {
                    instructions,
                    prompt,
                    extension_names: Vec::new(),
                    parameters: Vec::new(),
                })
            }
        };

        Some(RegistryEntry {
            name: idx.name.clone(),
            kind,
            description: idx.description.clone(),
            version: idx.version.clone(),
            source_uri: idx
                .url
                .clone()
                .or_else(|| Some(format!("{}/{}", self.base_url, idx.name))),
            tags: idx.tags.clone(),
            detail,
            ..Default::default()
        })
    }
}

#[async_trait]
impl RegistrySource for HttpRegistrySource {
    fn name(&self) -> &str {
        "http"
    }

    async fn search(
        &self,
        query: Option<&str>,
        kind: Option<RegistryEntryKind>,
    ) -> Result<Vec<RegistryEntry>> {
        let index = self.fetch_index().await?;

        let mut entries: Vec<RegistryEntry> = index
            .iter()
            .filter_map(|idx| self.index_entry_to_registry_entry(idx))
            .collect();

        if let Some(k) = kind {
            entries.retain(|e| e.kind == k);
        }

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
        let entries = self.search(Some(name), kind).await?;
        Ok(entries.into_iter().find(|e| e.name == name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_well_known_url() {
        let source = HttpRegistrySource::well_known("example.com");
        assert_eq!(
            source.base_url,
            "https://example.com/.well-known/agent.json"
        );
    }

    #[test]
    fn test_well_known_with_scheme() {
        let source = HttpRegistrySource::well_known("http://localhost:8080");
        assert_eq!(
            source.base_url,
            "http://localhost:8080/.well-known/agent.json"
        );
    }

    #[test]
    fn test_index_entry_to_registry_entry_tool() {
        let source = HttpRegistrySource::new("https://registry.example.com/api/v1");
        let idx = IndexEntry {
            name: "developer".to_string(),
            kind: "tool".to_string(),
            description: "Developer tools".to_string(),
            version: Some("1.0.0".to_string()),
            url: Some("https://example.com/developer".to_string()),
            tags: vec!["coding".to_string()],
            metadata: serde_json::json!({
                "transport": "stdio",
                "capabilities": ["text_editor", "shell"]
            }),
        };

        let entry = source.index_entry_to_registry_entry(&idx).unwrap();
        assert_eq!(entry.name, "developer");
        assert_eq!(entry.kind, RegistryEntryKind::Tool);
        if let RegistryEntryDetail::Tool(ref detail) = entry.detail {
            assert!(matches!(detail.transport, ToolTransport::Stdio { .. }));
            assert_eq!(detail.capabilities.len(), 2);
        } else {
            panic!("expected Tool detail");
        }
    }

    #[test]
    fn test_index_entry_to_registry_entry_agent() {
        let source = HttpRegistrySource::new("https://registry.example.com");
        let idx = IndexEntry {
            name: "code-reviewer".to_string(),
            kind: "agent".to_string(),
            description: "Reviews code".to_string(),
            version: None,
            url: None,
            tags: Vec::new(),
            metadata: serde_json::json!({
                "instructions": "You are a code reviewer.",
                "model": "claude-sonnet-4"
            }),
        };

        let entry = source.index_entry_to_registry_entry(&idx).unwrap();
        assert_eq!(entry.kind, RegistryEntryKind::Agent);
        if let RegistryEntryDetail::Agent(ref detail) = entry.detail {
            assert_eq!(detail.instructions, "You are a code reviewer.");
            assert_eq!(detail.model, Some("claude-sonnet-4".to_string()));
        } else {
            panic!("expected Agent detail");
        }
    }

    #[test]
    fn test_unknown_kind_filtered() {
        let source = HttpRegistrySource::new("https://registry.example.com");
        let idx = IndexEntry {
            name: "unknown".to_string(),
            kind: "workflow".to_string(),
            description: "Unknown type".to_string(),
            version: None,
            url: None,
            tags: Vec::new(),
            metadata: serde_json::Value::Null,
        };

        assert!(source.index_entry_to_registry_entry(&idx).is_none());
    }
}
