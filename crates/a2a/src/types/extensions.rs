//! Extension types mapped from a2a.proto.

use serde::{Deserialize, Serialize};

/// Agent extension declaration (proto `AgentExtension`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentExtension {
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// Extension URI type alias.
pub type ExtensionUri = String;

/// Parse a comma-separated extension service parameter into URIs.
pub fn parse_extensions_parameter(value: &str) -> Vec<ExtensionUri> {
    let mut seen = std::collections::HashSet::new();
    value
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && seen.insert(s.clone()))
        .collect()
}

/// Join extension URIs into a comma-separated service parameter.
pub fn extensions_to_parameter(extensions: &[ExtensionUri]) -> String {
    extensions.join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_extensions_parameter() {
        let uris = parse_extensions_parameter("ext1,ext2,ext3");
        assert_eq!(uris, vec!["ext1", "ext2", "ext3"]);
    }

    #[test]
    fn test_parse_extensions_deduplication() {
        let uris = parse_extensions_parameter("ext1,ext2,ext1");
        assert_eq!(uris, vec!["ext1", "ext2"]);
    }

    #[test]
    fn test_parse_extensions_trim() {
        let uris = parse_extensions_parameter(" ext1 , ext2 ");
        assert_eq!(uris, vec!["ext1", "ext2"]);
    }

    #[test]
    fn test_extensions_to_parameter() {
        let param = extensions_to_parameter(&["ext1".to_string(), "ext2".to_string()]);
        assert_eq!(param, "ext1,ext2");
    }

    #[test]
    fn test_agent_extension_serde() {
        let ext = AgentExtension {
            uri: "https://example.com/ext/v1".to_string(),
            description: Some("Test extension".to_string()),
            required: true,
            params: Some(serde_json::json!({"key": "value"})),
        };
        let json = serde_json::to_value(&ext).unwrap();
        assert_eq!(json["uri"], "https://example.com/ext/v1");
        assert_eq!(json["required"], true);
    }
}
