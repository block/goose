//! Multi-format parsing and generation for agent manifests.
//!
//! Supports:
//! - **ACP Client Protocol** `agent.json` (parse + generate)
//! - **A2A Protocol** `agent-card.json` (generate from RegistryEntry)
//! - **Goose native** `agent.yaml` (already handled by manifest.rs)

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::registry::manifest::{
    AgentDetail, AgentDistribution, AgentSkill, AuthorInfo, BinaryTarget, PackageDistribution,
    RegistryEntry, RegistryEntryDetail, RegistryEntryKind, SecurityScheme,
};

// ──────────────────────────────────────────────────────────────────────────────
// ACP Client Protocol agent.json
// ──────────────────────────────────────────────────────────────────────────────

/// ACP Client Protocol agent.json format.
///
/// Spec: <https://agentclientprotocol.com/rfds/acp-agent-registry>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpClientAgentJson {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub distribution: AcpDistribution,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AcpDistribution {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub binary: HashMap<String, AcpBinaryTarget>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub npx: Option<AcpPackageTarget>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub uvx: Option<AcpPackageTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpBinaryTarget {
    pub archive: String,
    pub cmd: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcpPackageTarget {
    pub package: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
}

/// Parse an ACP Client agent.json into a RegistryEntry.
pub fn parse_acp_client_agent_json(json_str: &str) -> anyhow::Result<RegistryEntry> {
    let acp: AcpClientAgentJson = serde_json::from_str(json_str)?;

    let author = if !acp.authors.is_empty() {
        Some(AuthorInfo {
            name: Some(acp.authors.join(", ")),
            ..Default::default()
        })
    } else {
        None
    };

    // Convert ACP binary targets to Goose BinaryTarget HashMap
    let binary: HashMap<String, BinaryTarget> = acp
        .distribution
        .binary
        .into_iter()
        .map(|(platform, t)| {
            (
                platform,
                BinaryTarget {
                    archive: t.archive,
                    cmd: t.cmd,
                    args: t.args,
                    env: t.env,
                },
            )
        })
        .collect();

    let distribution = Some(AgentDistribution {
        binary,
        npx: acp.distribution.npx.map(|p| PackageDistribution {
            package: p.package,
            args: Some(p.args).filter(|a| !a.is_empty()),
            env: p.env,
        }),
        uvx: acp.distribution.uvx.map(|p| PackageDistribution {
            package: p.package,
            args: Some(p.args).filter(|a| !a.is_empty()),
            env: p.env,
        }),
        cargo: None,
        docker: None,
    });

    Ok(RegistryEntry {
        name: acp.name,
        kind: RegistryEntryKind::Agent,
        description: acp.description,
        version: Some(acp.version),
        author,
        license: acp.license,
        icon: acp.icon,
        repository: acp.repository,
        detail: RegistryEntryDetail::Agent(Box::new(AgentDetail {
            distribution,
            ..Default::default()
        })),
        metadata: {
            let mut m = HashMap::new();
            m.insert("acp_client_id".to_string(), acp.id);
            m.insert("format".to_string(), "acp-client".to_string());
            m
        },
        ..Default::default()
    })
}

/// Generate an ACP Client agent.json from a RegistryEntry.
pub fn generate_acp_client_agent_json(entry: &RegistryEntry) -> anyhow::Result<String> {
    let id = entry
        .metadata
        .get("acp_client_id")
        .cloned()
        .unwrap_or_else(|| slug_from_name(&entry.name));

    let authors: Vec<String> = entry
        .author
        .as_ref()
        .and_then(|a| a.name.clone())
        .map(|n| vec![n])
        .unwrap_or_default();

    let distribution = if let RegistryEntryDetail::Agent(ref detail) = entry.detail {
        if let Some(ref dist) = detail.distribution {
            AcpDistribution {
                binary: dist
                    .binary
                    .iter()
                    .map(|(platform, t)| {
                        (
                            platform.clone(),
                            AcpBinaryTarget {
                                archive: t.archive.clone(),
                                cmd: t.cmd.clone(),
                                args: t.args.clone(),
                                env: t.env.clone(),
                            },
                        )
                    })
                    .collect(),
                npx: dist.npx.as_ref().map(|p| AcpPackageTarget {
                    package: p.package.clone(),
                    args: p.args.clone().unwrap_or_default(),
                    env: p.env.clone(),
                }),
                uvx: dist.uvx.as_ref().map(|p| AcpPackageTarget {
                    package: p.package.clone(),
                    args: p.args.clone().unwrap_or_default(),
                    env: p.env.clone(),
                }),
            }
        } else {
            AcpDistribution::default()
        }
    } else {
        AcpDistribution::default()
    };

    let acp = AcpClientAgentJson {
        id,
        name: entry.name.clone(),
        version: entry.version.clone().unwrap_or_else(|| "0.1.0".to_string()),
        description: entry.description.clone(),
        distribution,
        repository: entry.repository.clone(),
        authors,
        license: entry.license.clone(),
        icon: entry.icon.clone(),
    };

    Ok(serde_json::to_string_pretty(&acp)?)
}

fn slug_from_name(name: &str) -> String {
    name.to_lowercase()
        .replace(' ', "-")
        .chars()
        .filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
        .collect()
}

// ──────────────────────────────────────────────────────────────────────────────
// A2A Protocol agent-card.json
// ──────────────────────────────────────────────────────────────────────────────

/// A2A Protocol Agent Card.
///
/// Spec: <https://a2a-protocol.org/latest/specification/#5-agent-card>
/// Discovery: `/.well-known/agent-card.json`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct A2aAgentCard {
    pub name: String,
    pub description: String,
    pub version: String,
    pub supported_interfaces: Vec<A2aAgentInterface>,
    pub default_input_modes: Vec<String>,
    pub default_output_modes: Vec<String>,
    pub skills: Vec<A2aAgentSkill>,
    pub capabilities: A2aAgentCapabilities,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<A2aAgentProvider>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,

    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub security_schemes: HashMap<String, A2aSecurityScheme>,

    /// A2A extensions declared by this agent (MCP extensions mapped to A2A AgentExtension).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extensions: Vec<A2aAgentExtension>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct A2aAgentInterface {
    pub url: String,
    pub protocol_binding: String,
    pub protocol_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aAgentProvider {
    pub organization: String,
    pub url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct A2aAgentCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub push_notifications: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct A2aAgentSkill {
    pub id: String,
    pub name: String,
    pub description: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<String>,
}

/// A2A AgentExtension — declares an extension (MCP tool provider) available to this agent.
/// Maps to A2A spec §4.4.4 AgentExtension.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct A2aAgentExtension {
    /// URI identifying the extension (e.g., "mcp://developer", "mcp://memory").
    pub uri: String,

    /// Human-readable description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Whether this extension is required for the agent to function.
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct A2aSecurityScheme {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_security_scheme: Option<A2aApiKeySecurity>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_auth_security_scheme: Option<A2aHttpAuthSecurity>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth2_security_scheme: Option<A2aOAuth2Security>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aApiKeySecurity {
    pub location: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct A2aHttpAuthSecurity {
    pub scheme: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bearer_format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct A2aOAuth2Security {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flows: Option<serde_json::Value>,
}

/// Generate an A2A agent-card.json from a RegistryEntry.
pub fn generate_a2a_agent_card(entry: &RegistryEntry, agent_url: &str) -> anyhow::Result<String> {
    let (skills, input_types, output_types, security_schemes) =
        if let RegistryEntryDetail::Agent(ref detail) = entry.detail {
            let skills: Vec<A2aAgentSkill> = if detail.skills.is_empty() {
                // Fall back to capabilities as skills
                detail
                    .capabilities
                    .iter()
                    .enumerate()
                    .map(|(i, cap)| A2aAgentSkill {
                        id: format!("cap-{i}"),
                        name: cap.clone(),
                        description: cap.clone(),
                        tags: detail.domains.clone(),
                        examples: Vec::new(),
                    })
                    .collect()
            } else {
                detail
                    .skills
                    .iter()
                    .map(|s| A2aAgentSkill {
                        id: s.id.clone(),
                        name: s.name.clone(),
                        description: s.description.clone().unwrap_or_default(),
                        tags: s.tags.clone(),
                        examples: s.examples.clone(),
                    })
                    .collect()
            };

            let input_types = if detail.input_content_types.is_empty() {
                vec!["text/plain".to_string()]
            } else {
                detail.input_content_types.clone()
            };

            let output_types = if detail.output_content_types.is_empty() {
                vec!["text/plain".to_string()]
            } else {
                detail.output_content_types.clone()
            };

            let mut sec_schemes = HashMap::new();
            for scheme in &detail.security {
                match scheme {
                    SecurityScheme::ApiKey {
                        header,
                        query_param,
                    } => {
                        let (location, name) = if let Some(h) = header {
                            ("header".to_string(), h.clone())
                        } else if let Some(q) = query_param {
                            ("query".to_string(), q.clone())
                        } else {
                            continue;
                        };
                        sec_schemes.insert(
                            "apiKey".to_string(),
                            A2aSecurityScheme {
                                api_key_security_scheme: Some(A2aApiKeySecurity { location, name }),
                                http_auth_security_scheme: None,
                                oauth2_security_scheme: None,
                            },
                        );
                    }
                    SecurityScheme::Http { scheme: s } => {
                        sec_schemes.insert(
                            "http".to_string(),
                            A2aSecurityScheme {
                                api_key_security_scheme: None,
                                http_auth_security_scheme: Some(A2aHttpAuthSecurity {
                                    scheme: s.clone(),
                                    bearer_format: None,
                                }),
                                oauth2_security_scheme: None,
                            },
                        );
                    }
                    SecurityScheme::OAuth2 {
                        authorization_url,
                        token_url,
                        scopes,
                    } => {
                        let mut scope_map = serde_json::Map::new();
                        for s in scopes {
                            scope_map.insert(s.clone(), serde_json::Value::String(s.clone()));
                        }
                        let flows = serde_json::json!({
                            "authorizationCode": {
                                "authorizationUrl": authorization_url,
                                "tokenUrl": token_url,
                                "scopes": scope_map
                            }
                        });
                        sec_schemes.insert(
                            "oauth2".to_string(),
                            A2aSecurityScheme {
                                api_key_security_scheme: None,
                                http_auth_security_scheme: None,
                                oauth2_security_scheme: Some(A2aOAuth2Security {
                                    flows: Some(flows),
                                }),
                            },
                        );
                    }
                }
            }

            (skills, input_types, output_types, sec_schemes)
        } else {
            (
                Vec::new(),
                vec!["text/plain".to_string()],
                vec!["text/plain".to_string()],
                HashMap::new(),
            )
        };

    let provider = entry.author.as_ref().and_then(|a| {
        a.name.as_ref().map(|name| A2aAgentProvider {
            organization: name.clone(),
            url: a.url.clone().unwrap_or_default(),
        })
    });

    let card = A2aAgentCard {
        name: entry.name.clone(),
        description: entry.description.clone(),
        version: entry.version.clone().unwrap_or_else(|| "0.1.0".to_string()),
        supported_interfaces: vec![A2aAgentInterface {
            url: agent_url.to_string(),
            protocol_binding: "HTTP+JSON".to_string(),
            protocol_version: "1.0".to_string(),
        }],
        default_input_modes: input_types,
        default_output_modes: output_types,
        skills,
        capabilities: A2aAgentCapabilities {
            streaming: Some(true),
            push_notifications: None,
        },
        provider,
        documentation_url: entry.repository.clone(),
        icon_url: entry.icon.clone(),
        security_schemes,
        extensions: Vec::new(),
    };

    Ok(serde_json::to_string_pretty(&card)?)
}

/// Parse an A2A agent-card.json into a RegistryEntry.
pub fn parse_a2a_agent_card(json_str: &str) -> anyhow::Result<RegistryEntry> {
    let card: A2aAgentCard = serde_json::from_str(json_str)?;

    let author = card.provider.as_ref().map(|p| AuthorInfo {
        name: Some(p.organization.clone()),
        url: Some(p.url.clone()),
        ..Default::default()
    });

    let skills: Vec<AgentSkill> = card
        .skills
        .iter()
        .map(|s| AgentSkill {
            id: s.id.clone(),
            name: s.name.clone(),
            description: Some(s.description.clone()),
            tags: s.tags.clone(),
            examples: s.examples.clone(),
        })
        .collect();

    let capabilities: Vec<String> = card.skills.iter().map(|s| s.name.clone()).collect();
    let domains: Vec<String> = {
        let mut tags: Vec<String> = card.skills.iter().flat_map(|s| s.tags.clone()).collect();
        tags.sort();
        tags.dedup();
        tags
    };

    let mut security = Vec::new();
    for scheme in card.security_schemes.values() {
        if let Some(ref api_key) = scheme.api_key_security_scheme {
            security.push(SecurityScheme::ApiKey {
                header: if api_key.location == "header" {
                    Some(api_key.name.clone())
                } else {
                    None
                },
                query_param: if api_key.location == "query" {
                    Some(api_key.name.clone())
                } else {
                    None
                },
            });
        }
        if let Some(ref http) = scheme.http_auth_security_scheme {
            security.push(SecurityScheme::Http {
                scheme: http.scheme.clone(),
            });
        }
        if let Some(ref oauth) = scheme.oauth2_security_scheme {
            let (auth_url, token_url, scopes) = oauth
                .flows
                .as_ref()
                .and_then(|f| {
                    f.get("authorizationCode").map(|ac| {
                        let auth = ac
                            .get("authorizationUrl")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let token = ac
                            .get("tokenUrl")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let scopes: Vec<String> = ac
                            .get("scopes")
                            .and_then(|v| v.as_object())
                            .map(|m| m.keys().cloned().collect())
                            .unwrap_or_default();
                        (auth, token, scopes)
                    })
                })
                .unwrap_or_default();
            security.push(SecurityScheme::OAuth2 {
                authorization_url: auth_url,
                token_url,
                scopes,
            });
        }
    }

    let source_uri = card.supported_interfaces.first().map(|i| i.url.clone());

    Ok(RegistryEntry {
        name: card.name,
        kind: RegistryEntryKind::Agent,
        description: card.description,
        version: Some(card.version),
        author,
        icon: card.icon_url,
        repository: card.documentation_url,
        source_uri,
        detail: RegistryEntryDetail::Agent(Box::new(AgentDetail {
            capabilities,
            domains,
            skills,
            security,
            input_content_types: card.default_input_modes,
            output_content_types: card.default_output_modes,
            ..Default::default()
        })),
        metadata: {
            let mut m = HashMap::new();
            m.insert("format".to_string(), "a2a".to_string());
            m
        },
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_acp_client_binary_agent() {
        let json = r#"{
            "id": "goose-dev",
            "name": "Goose Developer",
            "version": "1.0.0",
            "description": "AI coding agent",
            "repository": "https://github.com/block/goose",
            "authors": ["Block"],
            "license": "Apache-2.0",
            "icon": "icon.svg",
            "distribution": {
                "binary": {
                    "linux-x86_64": {
                        "archive": "https://example.com/goose-linux.tar.gz",
                        "cmd": "./goose",
                        "args": ["acp"]
                    }
                }
            }
        }"#;

        let entry = parse_acp_client_agent_json(json).unwrap();
        assert_eq!(entry.name, "Goose Developer");
        assert_eq!(entry.kind, RegistryEntryKind::Agent);
        assert_eq!(entry.version, Some("1.0.0".to_string()));
        assert_eq!(entry.license, Some("Apache-2.0".to_string()));
        assert_eq!(
            entry.metadata.get("acp_client_id"),
            Some(&"goose-dev".to_string())
        );

        if let RegistryEntryDetail::Agent(ref detail) = entry.detail {
            assert!(detail.distribution.is_some());
            let dist = detail.distribution.as_ref().unwrap();
            assert_eq!(dist.binary.len(), 1);
        } else {
            panic!("Expected Agent detail");
        }
    }

    #[test]
    fn parse_acp_client_npx_agent() {
        let json = r#"{
            "id": "node-agent",
            "name": "Node Agent",
            "version": "2.1.0",
            "description": "A Node.js agent",
            "distribution": {
                "npx": {
                    "package": "node-agent@latest",
                    "args": ["--stdio"]
                }
            }
        }"#;

        let entry = parse_acp_client_agent_json(json).unwrap();
        assert_eq!(entry.name, "Node Agent");

        if let RegistryEntryDetail::Agent(ref detail) = entry.detail {
            let dist = detail.distribution.as_ref().unwrap();
            assert!(dist.npx.is_some());
            assert_eq!(dist.npx.as_ref().unwrap().package, "node-agent@latest");
        } else {
            panic!("Expected Agent detail");
        }
    }

    #[test]
    fn acp_client_roundtrip() {
        let json = r#"{
            "id": "test-agent",
            "name": "Test Agent",
            "version": "1.0.0",
            "description": "A test agent",
            "authors": ["Test Corp"],
            "license": "MIT",
            "distribution": {
                "uvx": {
                    "package": "test-agent@1.0.0",
                    "args": ["--mode", "acp"]
                }
            }
        }"#;

        let entry = parse_acp_client_agent_json(json).unwrap();
        let regenerated = generate_acp_client_agent_json(&entry).unwrap();
        let reparsed = parse_acp_client_agent_json(&regenerated).unwrap();

        assert_eq!(entry.name, reparsed.name);
        assert_eq!(entry.version, reparsed.version);
        assert_eq!(entry.license, reparsed.license);
    }

    #[test]
    fn generate_a2a_card_from_agent() {
        let entry = RegistryEntry {
            name: "Goose Developer".to_string(),
            kind: RegistryEntryKind::Agent,
            description: "AI coding agent".to_string(),
            version: Some("1.0.0".to_string()),
            author: Some(AuthorInfo {
                name: Some("Block".to_string()),
                url: Some("https://block.xyz".to_string()),
                ..Default::default()
            }),
            detail: RegistryEntryDetail::Agent(Box::new(AgentDetail {
                capabilities: vec!["Code Generation".to_string(), "Code Review".to_string()],
                domains: vec!["software-development".to_string()],
                input_content_types: vec!["text/plain".to_string()],
                output_content_types: vec![
                    "text/plain".to_string(),
                    "application/json".to_string(),
                ],
                security: vec![SecurityScheme::ApiKey {
                    header: Some("X-Agent-Key".to_string()),
                    query_param: None,
                }],
                ..Default::default()
            })),
            ..Default::default()
        };

        let json = generate_a2a_agent_card(&entry, "https://goose.example.com/a2a").unwrap();
        assert!(json.contains("Goose Developer"));
        assert!(json.contains("supportedInterfaces"));
        assert!(json.contains("https://goose.example.com/a2a"));
        assert!(json.contains("Code Generation"));
        assert!(json.contains("X-Agent-Key"));

        let card: A2aAgentCard = serde_json::from_str(&json).unwrap();
        assert_eq!(card.skills.len(), 2);
        assert_eq!(card.provider.unwrap().organization, "Block");
    }

    #[test]
    fn parse_a2a_agent_card_test() {
        let json = r#"{
            "name": "Remote Agent",
            "description": "A remote coding agent",
            "version": "2.0.0",
            "supportedInterfaces": [
                {
                    "url": "https://agent.example.com/a2a",
                    "protocolBinding": "HTTP+JSON",
                    "protocolVersion": "1.0"
                }
            ],
            "defaultInputModes": ["text/plain"],
            "defaultOutputModes": ["text/plain", "application/json"],
            "skills": [
                {
                    "id": "code-gen",
                    "name": "Code Generation",
                    "description": "Generates code",
                    "tags": ["coding", "rust"]
                }
            ],
            "capabilities": {
                "streaming": true
            },
            "provider": {
                "organization": "Example Corp",
                "url": "https://example.com"
            },
            "securitySchemes": {
                "bearer": {
                    "httpAuthSecurityScheme": {
                        "scheme": "Bearer"
                    }
                }
            }
        }"#;

        let entry = parse_a2a_agent_card(json).unwrap();
        assert_eq!(entry.name, "Remote Agent");
        assert_eq!(entry.kind, RegistryEntryKind::Agent);
        assert_eq!(entry.version, Some("2.0.0".to_string()));
        assert_eq!(
            entry.source_uri,
            Some("https://agent.example.com/a2a".to_string())
        );

        if let RegistryEntryDetail::Agent(ref detail) = entry.detail {
            assert_eq!(detail.skills.len(), 1);
            assert_eq!(detail.skills[0].id, "code-gen");
            assert_eq!(detail.security.len(), 1);
            assert!(matches!(detail.security[0], SecurityScheme::Http { .. }));
        } else {
            panic!("Expected Agent detail");
        }
    }

    #[test]
    fn a2a_roundtrip() {
        let entry = RegistryEntry {
            name: "Roundtrip Agent".to_string(),
            kind: RegistryEntryKind::Agent,
            description: "Testing roundtrip".to_string(),
            version: Some("1.0.0".to_string()),
            detail: RegistryEntryDetail::Agent(Box::new(AgentDetail {
                skills: vec![AgentSkill {
                    id: "test".to_string(),
                    name: "Testing".to_string(),
                    description: Some("A test skill".to_string()),
                    tags: vec!["test".to_string()],
                    examples: vec!["Run tests".to_string()],
                }],
                ..Default::default()
            })),
            ..Default::default()
        };

        let card_json = generate_a2a_agent_card(&entry, "https://example.com/a2a").unwrap();
        let reparsed = parse_a2a_agent_card(&card_json).unwrap();

        assert_eq!(entry.name, reparsed.name);
        assert_eq!(entry.version, reparsed.version);
    }

    #[test]
    fn slug_generation() {
        assert_eq!(slug_from_name("Goose Developer"), "goose-developer");
        assert_eq!(slug_from_name("My Agent 2.0!"), "my-agent-20");
        assert_eq!(slug_from_name("simple"), "simple");
    }
}
