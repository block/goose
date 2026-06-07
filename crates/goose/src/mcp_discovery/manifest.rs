use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::error::DiscoveryError;

/// Trust class declared by an MCP server manifest.
///
/// Per draft-serra-mcp-discovery-uri a missing declaration defaults to the most
/// restrictive safe value, `Public`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum TrustClass {
    #[default]
    Public,
    Sandbox,
    Enterprise,
    Regulated,
}

impl TrustClass {
    /// Whether the spec mandates an `auth` declaration for this trust class.
    pub fn requires_auth_declaration(self) -> bool {
        matches!(self, TrustClass::Enterprise | TrustClass::Regulated)
    }
}

/// Authentication requirements advertised by a manifest.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct AuthRequirements {
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub methods: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apikey_header: Option<String>,
    #[serde(default)]
    pub scopes: Vec<String>,
}

/// Detached JWS signature object carried in a manifest.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ManifestSignature {
    pub alg: String,
    pub kid: String,
    /// Base64url-encoded detached JWS signature over the canonical JSON
    /// serialization of the manifest with the `signature` field removed.
    pub value: String,
}

/// The `.well-known/mcp-server` manifest as defined by the draft.
///
/// Only the fields goose acts on are modelled explicitly; the remaining
/// optional fields are accepted and ignored to stay forward-compatible.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct McpServerManifest {
    pub mcp_version: String,
    pub name: String,
    pub endpoint: String,
    pub transport: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthRequirements>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trust_class: Option<TrustClass>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<ManifestSignature>,
}

impl McpServerManifest {
    /// Trust class, defaulting to the most restrictive value when absent.
    pub fn effective_trust_class(&self) -> TrustClass {
        self.trust_class.unwrap_or_default()
    }

    /// Validate the manifest's internal consistency against the spec's MUST
    /// rules. Does not perform host-match or signature checks — those need the
    /// discovery host and DNS keys respectively.
    pub fn validate(&self, url: &str) -> Result<(), DiscoveryError> {
        if self.transport != "http" {
            return Err(DiscoveryError::UnsupportedTransport(self.transport.clone()));
        }
        if self.endpoint.trim().is_empty() {
            return Err(DiscoveryError::MalformedManifest {
                url: url.to_string(),
                reason: "endpoint is empty".to_string(),
            });
        }
        if self.effective_trust_class().requires_auth_declaration() && self.auth.is_none() {
            return Err(DiscoveryError::MalformedManifest {
                url: url.to_string(),
                reason: format!(
                    "trust_class {:?} requires an auth declaration",
                    self.effective_trust_class()
                ),
            });
        }
        Ok(())
    }
}

/// Returns true when `endpoint_host` equals `discovery_host` or is a subdomain
/// of it. Comparison is case-insensitive and a trailing dot is ignored.
pub fn host_matches(discovery_host: &str, endpoint_host: &str) -> bool {
    let normalize = |h: &str| h.trim_end_matches('.').to_ascii_lowercase();
    let discovery = normalize(discovery_host);
    let endpoint = normalize(endpoint_host);
    if discovery.is_empty() || endpoint.is_empty() {
        return false;
    }
    endpoint == discovery || endpoint.ends_with(&format!(".{discovery}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_match_accepts_exact_and_subdomain() {
        assert!(host_matches("example.com", "example.com"));
        assert!(host_matches("example.com", "api.example.com"));
        assert!(host_matches("example.com", "a.b.example.com"));
        assert!(host_matches("Example.com", "API.example.com."));
    }

    #[test]
    fn host_match_rejects_unrelated_and_suffix_tricks() {
        assert!(!host_matches("example.com", "evil.com"));
        assert!(!host_matches("example.com", "notexample.com"));
        assert!(!host_matches("example.com", "example.com.evil.com"));
        assert!(!host_matches("example.com", ""));
    }

    #[test]
    fn validate_rejects_non_http_transport() {
        let m = McpServerManifest {
            mcp_version: "2025-06-18".into(),
            name: "x".into(),
            endpoint: "https://example.com/mcp".into(),
            transport: "stdio".into(),
            description: None,
            auth: None,
            capabilities: vec![],
            trust_class: None,
            signature: None,
        };
        assert!(matches!(
            m.validate("u"),
            Err(DiscoveryError::UnsupportedTransport(_))
        ));
    }

    #[test]
    fn validate_requires_auth_for_enterprise() {
        let m = McpServerManifest {
            mcp_version: "2025-06-18".into(),
            name: "x".into(),
            endpoint: "https://example.com/mcp".into(),
            transport: "http".into(),
            description: None,
            auth: None,
            capabilities: vec![],
            trust_class: Some(TrustClass::Enterprise),
            signature: None,
        };
        assert!(matches!(
            m.validate("u"),
            Err(DiscoveryError::MalformedManifest { .. })
        ));
    }

    #[test]
    fn trust_class_defaults_to_public() {
        let m = McpServerManifest {
            mcp_version: "2025-06-18".into(),
            name: "x".into(),
            endpoint: "https://example.com/mcp".into(),
            transport: "http".into(),
            description: None,
            auth: None,
            capabilities: vec![],
            trust_class: None,
            signature: None,
        };
        assert_eq!(m.effective_trust_class(), TrustClass::Public);
    }
}
