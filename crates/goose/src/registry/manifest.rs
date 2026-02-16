use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The four kinds of artifacts in the registry.
///
/// Aligns with the existing `SourceKind` enum in summon_extension.rs
/// but adds Tool (which is managed separately by ExtensionManager today).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum RegistryEntryKind {
    #[default]
    Tool,
    Skill,
    Agent,
    Recipe,
}

impl std::fmt::Display for RegistryEntryKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tool => write!(f, "tool"),
            Self::Skill => write!(f, "skill"),
            Self::Agent => write!(f, "agent"),
            Self::Recipe => write!(f, "recipe"),
        }
    }
}

/// A unified registry entry that can represent any of the 4 artifact types.
///
/// Designed to be the common currency across all registry sources (local, GitHub, HTTP).
/// Schema is a superset of:
/// - ACP Client Protocol agent.json (distribution, version, id)
/// - ACP Communication Protocol manifest (metadata, dependencies, content types)
/// - A2A Agent Card (skills, security, discovery)
/// - Kilo Code modes (behavioral configurations)
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct RegistryEntry {
    pub name: String,
    pub kind: RegistryEntryKind,
    pub description: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<AuthorInfo>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    /// Where this entry was resolved from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_uri: Option<String>,

    /// Local path if available (e.g. from filesystem scan).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    pub local_path: Option<PathBuf>,

    /// Tags for search and categorization.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    /// Kind-specific payload.
    #[serde(flatten)]
    pub detail: RegistryEntryDetail,

    /// Additional metadata from external registries.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

impl RegistryEntry {
    /// Merge metadata from another entry with the same name+kind.
    pub fn merge_metadata(&mut self, other: &RegistryEntry) {
        for (k, v) in &other.metadata {
            self.metadata.entry(k.clone()).or_insert_with(|| v.clone());
        }
        if self.version.is_none() {
            self.version.clone_from(&other.version);
        }
        if self.author.is_none() {
            self.author.clone_from(&other.author);
        }
        if self.license.is_none() {
            self.license.clone_from(&other.license);
        }
        if self.repository.is_none() {
            self.repository.clone_from(&other.repository);
        }
    }

    /// Check if this entry has enough metadata to be published to a registry.
    pub fn validate_for_publish(&self) -> Vec<String> {
        let mut issues = Vec::new();

        if self.name.is_empty() {
            issues.push("name is required".into());
        }
        if self.description.is_empty() {
            issues.push("description is required".into());
        }
        if self.version.is_none() {
            issues.push("version is required for publishing".into());
        }
        if self.author.is_none() {
            issues.push("author is recommended for publishing".into());
        }
        if self.license.is_none() {
            issues.push("license is recommended for publishing".into());
        }

        if let RegistryEntryDetail::Agent(ref agent) = self.detail {
            if agent.instructions.is_empty() {
                issues.push("agent instructions are required".into());
            }
            if agent.capabilities.is_empty() {
                issues.push("at least one capability is recommended".into());
            }
        }

        issues
    }
}

/// Kind-specific details for each registry entry type.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "detail_type")]
pub enum RegistryEntryDetail {
    #[serde(rename = "tool")]
    Tool(ToolDetail),
    #[serde(rename = "skill")]
    Skill(SkillDetail),
    #[serde(rename = "agent")]
    Agent(Box<AgentDetail>),
    #[serde(rename = "recipe")]
    Recipe(RecipeDetail),
}

impl Default for RegistryEntryDetail {
    fn default() -> Self {
        Self::Tool(ToolDetail::default())
    }
}

// ──────────────────────────────────────────────────────────
// Tool types
// ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ToolDetail {
    pub transport: ToolTransport,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env_keys: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ToolTransport {
    Stdio {
        cmd: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        args: Vec<String>,
    },
    StreamableHttp {
        uri: String,
    },
    #[default]
    Builtin,
}

// ──────────────────────────────────────────────────────────
// Skill types
// ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SkillDetail {
    pub content: String,
    pub builtin: bool,
}

// ──────────────────────────────────────────────────────────
// Agent types (Kilo Code modes + ACP + A2A)
// ──────────────────────────────────────────────────────────

/// A dependency required by an agent or recipe.
///
/// Inspired by ACP Agent Manifest `dependencies` field.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentDependency {
    #[serde(rename = "type")]
    pub dep_type: RegistryEntryKind,

    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    #[serde(default = "default_true")]
    pub required: bool,
}

fn default_true() -> bool {
    true
}

/// A behavioral mode for an agent (Kilo Code-inspired).
///
/// Modes allow one agent to have multiple behavioral configurations.
/// Each mode can restrict available tools and override the system prompt.
/// Orthogonal to GooseMode (Auto/Approve/Chat) which controls permissions.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentMode {
    /// Unique identifier for this mode (e.g., "code", "review", "architect").
    pub slug: String,

    /// Display name (e.g., "💻 Code", "🔍 Review").
    pub name: String,

    pub description: String,

    /// Inline instructions that override or augment the agent's base instructions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,

    /// Path to a .md file containing mode-specific instructions.
    /// Relative to the agent's directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions_file: Option<String>,

    /// Tool groups available in this mode.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_groups: Vec<ToolGroupAccess>,

    /// Hint for when this mode should be auto-selected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when_to_use: Option<String>,

    /// Internal modes are used by orchestration only, not exposed via ACP/A2A discovery.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_internal: bool,

    /// If set, this mode is deprecated. The message explains what to use instead.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<String>,
}

/// Access control for a tool group within a mode.
///
/// Either full access to all files, or restricted to files matching a regex.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum ToolGroupAccess {
    /// Full access to the tool group (e.g., "read", "edit", "command", "mcp").
    Full(String),

    /// Restricted access: only files matching file_regex can be accessed.
    Restricted { group: String, file_regex: String },
}

/// A structured skill declaration (A2A-inspired).
///
/// Skills describe what an agent can do, for discovery and matching.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AgentSkill {
    pub id: String,
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub examples: Vec<String>,
}

/// Detail for an Agent definition.
///
/// Schema is a superset of:
/// - ACP Communication Protocol manifest (capabilities, domains, content types, deps)
/// - A2A Agent Card (skills, security)
/// - Kilo Code modes (behavioral modes with tool group access)
/// - ACP Client Protocol (distribution, framework, programming language)
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct AgentDetail {
    pub instructions: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recommended_models: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub domains: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_content_types: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub output_content_types: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_extensions: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<AgentDependency>,

    // ── Modes (Kilo Code-inspired) ──
    /// Default mode slug. If None, agent has no mode concept.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_mode: Option<String>,

    /// Available behavioral modes for this agent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modes: Vec<AgentMode>,

    // ── Skills (A2A-inspired) ──
    /// Structured skill declarations for discovery and matching.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<AgentSkill>,

    // ── Distribution (ACP Client-inspired) ──
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distribution: Option<AgentDistribution>,

    // ── Security (A2A-inspired) ──
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub security: Vec<SecurityScheme>,

    // ── Runtime metadata (ACP Comm-inspired) ──
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<RuntimeStatus>,

    // ── Additional metadata ──
    #[serde(skip_serializing_if = "Option::is_none")]
    pub framework: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub programming_language: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub natural_languages: Vec<String>,
}

// ──────────────────────────────────────────────────────────
// Distribution types (ACP Client Protocol-inspired)
// ──────────────────────────────────────────────────────────

/// How an agent can be distributed and installed.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct AgentDistribution {
    /// Platform-specific binary downloads.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub binary: HashMap<String, BinaryTarget>,

    /// Install via npx (Node.js package).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub npx: Option<PackageDistribution>,

    /// Install via uvx (Python package).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uvx: Option<PackageDistribution>,

    /// Install via cargo (Rust crate).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cargo: Option<PackageDistribution>,

    /// Install via Docker image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docker: Option<DockerDistribution>,
}

/// A binary download target for a specific platform.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BinaryTarget {
    pub archive: String,
    pub cmd: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
}

/// A package distribution (npm, pip, cargo).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PackageDistribution {
    pub package: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
}

/// Docker-based distribution.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DockerDistribution {
    pub image: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
}

// ──────────────────────────────────────────────────────────
// Security types (A2A-inspired, simplified from OpenAPI 3.2)
// ──────────────────────────────────────────────────────────

/// A security scheme for authenticating with a remote agent.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SecurityScheme {
    ApiKey {
        #[serde(skip_serializing_if = "Option::is_none")]
        header: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        query_param: Option<String>,
    },
    Http {
        scheme: String, // "bearer", "basic"
    },
    #[serde(rename = "oauth2")]
    OAuth2 {
        authorization_url: String,
        token_url: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        scopes: Vec<String>,
    },
}

// ──────────────────────────────────────────────────────────
// Runtime status (ACP Communication Protocol-inspired)
// ──────────────────────────────────────────────────────────

/// Runtime performance statistics for a deployed agent.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RuntimeStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_run_tokens: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_run_time_seconds: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub success_rate: Option<f64>,
}

// ──────────────────────────────────────────────────────────
// Recipe types
// ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RecipeDetail {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extension_names: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<String>,
}

// ──────────────────────────────────────────────────────────
// Author types
// ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct AuthorInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

// ──────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_tool_entry() {
        let entry = RegistryEntry {
            name: "developer".into(),
            kind: RegistryEntryKind::Tool,
            description: "Developer tools for code editing and shell".into(),
            version: Some("1.0.0".into()),
            license: Some("Apache-2.0".into()),
            tags: vec!["coding".into(), "shell".into()],
            detail: RegistryEntryDetail::Tool(ToolDetail {
                transport: ToolTransport::Builtin,
                capabilities: vec!["text_editor".into(), "shell".into()],
                env_keys: vec![],
            }),
            ..Default::default()
        };

        let json = serde_json::to_string_pretty(&entry).unwrap();
        assert!(json.contains("developer"));
        assert!(json.contains("tool"));
        assert!(json.contains("Apache-2.0"));

        let roundtrip: RegistryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.name, "developer");
        assert_eq!(roundtrip.kind, RegistryEntryKind::Tool);
        assert_eq!(roundtrip.license, Some("Apache-2.0".into()));
    }

    #[test]
    fn serialize_skill_entry() {
        let entry = RegistryEntry {
            name: "goose-doc-guide".into(),
            kind: RegistryEntryKind::Skill,
            description: "Guide for fetching goose documentation".into(),
            local_path: Some(PathBuf::from(
                "/home/user/.config/goose/skills/doc-guide/SKILL.md",
            )),
            tags: vec!["documentation".into()],
            detail: RegistryEntryDetail::Skill(SkillDetail {
                content: "When the user asks about goose...".into(),
                builtin: true,
            }),
            ..Default::default()
        };

        let json = serde_json::to_string_pretty(&entry).unwrap();
        assert!(json.contains("goose-doc-guide"));
        assert!(json.contains("skill"));
    }

    #[test]
    fn serialize_agent_with_modes() {
        let entry = RegistryEntry {
            name: "goose-developer".into(),
            kind: RegistryEntryKind::Agent,
            description: "Full-stack development agent with multiple modes".into(),
            version: Some("1.0.0".into()),
            license: Some("Apache-2.0".into()),
            repository: Some("https://github.com/block/goose".into()),
            author: Some(AuthorInfo {
                name: Some("Block".into()),
                contact: None,
                url: Some("https://block.xyz".into()),
            }),
            tags: vec!["coding".into(), "developer".into()],
            detail: RegistryEntryDetail::Agent(Box::new(AgentDetail {
                instructions: "You are a full-stack developer...".into(),
                model: Some("claude-sonnet-4".into()),
                recommended_models: vec!["claude-sonnet-4".into(), "gpt-4o".into()],
                capabilities: vec!["code-generation".into(), "code-review".into()],
                domains: vec!["software-development".into()],
                input_content_types: vec!["text/plain".into()],
                output_content_types: vec!["text/markdown".into()],
                required_extensions: vec!["developer".into(), "memory".into()],
                dependencies: vec![AgentDependency {
                    dep_type: RegistryEntryKind::Tool,
                    name: "developer".into(),
                    version: None,
                    required: true,
                }],
                default_mode: Some("code".into()),
                modes: vec![
                    AgentMode {
                        slug: "code".into(),
                        name: "💻 Code".into(),
                        description: "Full coding agent with all tools".into(),
                        instructions: None,
                        instructions_file: Some("modes/code.md".into()),
                        tool_groups: vec![
                            ToolGroupAccess::Full("read".into()),
                            ToolGroupAccess::Full("edit".into()),
                            ToolGroupAccess::Full("command".into()),
                            ToolGroupAccess::Full("mcp".into()),
                        ],
                        when_to_use: Some("When the user wants to write or modify code".into()),
                        is_internal: false,
                        deprecated: None,
                    },
                    AgentMode {
                        slug: "review".into(),
                        name: "🔍 Review".into(),
                        description: "Code review mode (read-only)".into(),
                        instructions: Some("You are a code reviewer. Do NOT modify files.".into()),
                        instructions_file: None,
                        tool_groups: vec![
                            ToolGroupAccess::Full("read".into()),
                            ToolGroupAccess::Full("mcp".into()),
                        ],
                        when_to_use: Some("When the user wants a code review".into()),
                        is_internal: false,
                        deprecated: None,
                    },
                    AgentMode {
                        slug: "architect".into(),
                        name: "📐 Architect".into(),
                        description: "System design mode (markdown only)".into(),
                        instructions: None,
                        instructions_file: Some("modes/architect.md".into()),
                        tool_groups: vec![
                            ToolGroupAccess::Full("read".into()),
                            ToolGroupAccess::Full("mcp".into()),
                            ToolGroupAccess::Restricted {
                                group: "edit".into(),
                                file_regex: r"\.(md|mdx)$".into(),
                            },
                        ],
                        when_to_use: Some("When discussing architecture or design".into()),
                        is_internal: false,
                        deprecated: None,
                    },
                ],
                skills: vec![AgentSkill {
                    id: "code-gen".into(),
                    name: "Code Generation".into(),
                    description: Some("Generate and modify source code".into()),
                    tags: vec!["rust".into(), "typescript".into()],
                    examples: vec!["Create a REST API endpoint".into()],
                }],
                distribution: None,
                security: vec![],
                status: None,
                framework: Some("goose".into()),
                programming_language: Some("rust".into()),
                natural_languages: vec!["en".into()],
            })),
            ..Default::default()
        };

        let json = serde_json::to_string_pretty(&entry).unwrap();
        assert!(json.contains("goose-developer"));
        assert!(json.contains("modes"));
        assert!(json.contains("code"));
        assert!(json.contains("review"));
        assert!(json.contains("architect"));
        assert!(json.contains("tool_groups"));
        assert!(json.contains("skills"));
        assert!(json.contains("code-gen"));
        assert!(json.contains("framework"));

        // Roundtrip
        let roundtrip: RegistryEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.name, "goose-developer");
        if let RegistryEntryDetail::Agent(ref detail) = roundtrip.detail {
            assert_eq!(detail.modes.len(), 3);
            assert_eq!(detail.default_mode, Some("code".into()));
            assert_eq!(detail.modes[0].slug, "code");
            assert_eq!(detail.modes[1].slug, "review");
            assert_eq!(detail.modes[2].tool_groups.len(), 3);
            assert_eq!(detail.skills.len(), 1);
            assert_eq!(detail.framework, Some("goose".into()));
            // Check restricted tool group
            if let ToolGroupAccess::Restricted {
                ref group,
                ref file_regex,
            } = detail.modes[2].tool_groups[2]
            {
                assert_eq!(group, "edit");
                assert!(file_regex.contains("md"));
            } else {
                panic!("Expected Restricted tool group");
            }
        } else {
            panic!("Expected AgentDetail");
        }
    }

    #[test]
    fn serialize_agent_with_distribution() {
        let entry = RegistryEntry {
            name: "remote-agent".into(),
            kind: RegistryEntryKind::Agent,
            description: "An agent with distribution info".into(),
            version: Some("2.0.0".into()),
            detail: RegistryEntryDetail::Agent(Box::new(AgentDetail {
                instructions: "You are a remote agent".into(),
                distribution: Some(AgentDistribution {
                    binary: {
                        let mut m = HashMap::new();
                        m.insert(
                            "darwin-aarch64".into(),
                            BinaryTarget {
                                archive: "https://example.com/agent-darwin.tar.gz".into(),
                                cmd: "my-agent".into(),
                                args: vec![],
                                env: HashMap::new(),
                            },
                        );
                        m
                    },
                    npx: Some(PackageDistribution {
                        package: "@block/my-agent".into(),
                        args: None,
                        env: HashMap::new(),
                    }),
                    uvx: None,
                    cargo: Some(PackageDistribution {
                        package: "my-agent".into(),
                        args: Some(vec!["--features".into(), "full".into()]),
                        env: HashMap::new(),
                    }),
                    docker: Some(DockerDistribution {
                        image: "ghcr.io/block/my-agent".into(),
                        tag: Some("latest".into()),
                        ports: vec!["8080:8080".into()],
                        env: HashMap::new(),
                    }),
                }),
                security: vec![
                    SecurityScheme::ApiKey {
                        header: Some("X-Agent-Key".into()),
                        query_param: None,
                    },
                    SecurityScheme::OAuth2 {
                        authorization_url: "https://auth.example.com/authorize".into(),
                        token_url: "https://auth.example.com/token".into(),
                        scopes: vec!["agent:run".into()],
                    },
                ],
                status: Some(RuntimeStatus {
                    avg_run_tokens: Some(1500.0),
                    avg_run_time_seconds: Some(12.5),
                    success_rate: Some(0.95),
                }),
                ..Default::default()
            })),
            ..Default::default()
        };

        let json = serde_json::to_string_pretty(&entry).unwrap();
        assert!(json.contains("distribution"));
        assert!(json.contains("darwin-aarch64"));
        assert!(json.contains("npx"));
        assert!(json.contains("docker"));
        assert!(json.contains("security"));
        assert!(json.contains("api_key"));
        assert!(json.contains("oauth2"));
        assert!(json.contains("status"));
        assert!(json.contains("1500"));

        let roundtrip: RegistryEntry = serde_json::from_str(&json).unwrap();
        if let RegistryEntryDetail::Agent(ref detail) = roundtrip.detail {
            assert!(detail.distribution.is_some());
            let dist = detail.distribution.as_ref().unwrap();
            assert!(dist.binary.contains_key("darwin-aarch64"));
            assert!(dist.npx.is_some());
            assert!(dist.docker.is_some());
            assert_eq!(detail.security.len(), 2);
            assert!(detail.status.is_some());
        } else {
            panic!("Expected AgentDetail");
        }
    }

    #[test]
    fn serialize_recipe_entry() {
        let entry = RegistryEntry {
            name: "analyze-pr".into(),
            kind: RegistryEntryKind::Recipe,
            description: "Analyze a pull request".into(),
            version: Some("1.0.0".into()),
            author: Some(AuthorInfo {
                name: Some("Goose Team".into()),
                contact: None,
                url: None,
            }),
            source_uri: Some("https://github.com/block/goose/recipes/analyze-pr.yaml".into()),
            tags: vec!["github".into(), "code-review".into()],
            detail: RegistryEntryDetail::Recipe(RecipeDetail {
                instructions: Some("Analyze the given PR...".into()),
                prompt: Some("Please analyze PR #{{pr_number}}".into()),
                extension_names: vec!["developer".into(), "memory".into()],
                parameters: vec!["pr_number".into(), "repo".into()],
            }),
            ..Default::default()
        };

        let json = serde_json::to_string_pretty(&entry).unwrap();
        assert!(json.contains("analyze-pr"));
        assert!(json.contains("recipe"));
    }

    #[test]
    fn merge_metadata_combines_entries() {
        let mut entry1 = RegistryEntry {
            name: "test".into(),
            kind: RegistryEntryKind::Tool,
            description: "test tool".into(),
            detail: RegistryEntryDetail::Tool(ToolDetail {
                transport: ToolTransport::Builtin,
                capabilities: vec![],
                env_keys: vec![],
            }),
            ..Default::default()
        };

        let entry2 = RegistryEntry {
            name: "test".into(),
            kind: RegistryEntryKind::Tool,
            description: "test tool from remote".into(),
            version: Some("2.0.0".into()),
            license: Some("MIT".into()),
            repository: Some("https://github.com/example/test".into()),
            author: Some(AuthorInfo {
                name: Some("Remote".into()),
                contact: None,
                url: None,
            }),
            source_uri: Some("https://example.com".into()),
            detail: RegistryEntryDetail::Tool(ToolDetail {
                transport: ToolTransport::Builtin,
                capabilities: vec![],
                env_keys: vec![],
            }),
            metadata: {
                let mut m = HashMap::new();
                m.insert("rating".into(), "A".into());
                m
            },
            ..Default::default()
        };

        entry1.merge_metadata(&entry2);
        assert_eq!(entry1.version, Some("2.0.0".into()));
        assert_eq!(entry1.license, Some("MIT".into()));
        assert_eq!(
            entry1.repository,
            Some("https://github.com/example/test".into())
        );
        assert_eq!(entry1.author.unwrap().name, Some("Remote".into()));
        assert_eq!(entry1.metadata.get("rating"), Some(&"A".into()));
    }

    #[test]
    fn entry_kind_display() {
        assert_eq!(RegistryEntryKind::Tool.to_string(), "tool");
        assert_eq!(RegistryEntryKind::Skill.to_string(), "skill");
        assert_eq!(RegistryEntryKind::Agent.to_string(), "agent");
        assert_eq!(RegistryEntryKind::Recipe.to_string(), "recipe");
    }

    #[test]
    fn validate_for_publish_complete_agent() {
        let entry = RegistryEntry {
            name: "my-agent".into(),
            kind: RegistryEntryKind::Agent,
            description: "A useful agent".into(),
            version: Some("1.0.0".into()),
            license: Some("Apache-2.0".into()),
            author: Some(AuthorInfo {
                name: Some("Test".into()),
                contact: None,
                url: None,
            }),
            detail: RegistryEntryDetail::Agent(Box::new(AgentDetail {
                instructions: "You are a helpful agent.".into(),
                capabilities: vec!["general".into()],
                ..Default::default()
            })),
            ..Default::default()
        };

        let issues = entry.validate_for_publish();
        assert!(issues.is_empty(), "Expected no issues, got: {:?}", issues);
    }

    #[test]
    fn validate_for_publish_incomplete() {
        let entry = RegistryEntry {
            name: "".into(),
            kind: RegistryEntryKind::Agent,
            description: "".into(),
            detail: RegistryEntryDetail::Agent(Box::default()),
            ..Default::default()
        };

        let issues = entry.validate_for_publish();
        assert!(issues.iter().any(|i| i.contains("name")));
        assert!(issues.iter().any(|i| i.contains("description")));
        assert!(issues.iter().any(|i| i.contains("version")));
        assert!(issues.iter().any(|i| i.contains("instructions")));
    }

    #[test]
    fn tool_group_access_roundtrip() {
        let groups = vec![
            ToolGroupAccess::Full("read".into()),
            ToolGroupAccess::Full("mcp".into()),
            ToolGroupAccess::Restricted {
                group: "edit".into(),
                file_regex: r"\.(md|mdx)$".into(),
            },
        ];

        let json = serde_json::to_string(&groups).unwrap();
        let roundtrip: Vec<ToolGroupAccess> = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.len(), 3);
        match &roundtrip[0] {
            ToolGroupAccess::Full(g) => assert_eq!(g, "read"),
            _ => panic!("Expected Full"),
        }
        match &roundtrip[2] {
            ToolGroupAccess::Restricted { group, file_regex } => {
                assert_eq!(group, "edit");
                assert!(file_regex.contains("md"));
            }
            _ => panic!("Expected Restricted"),
        }
    }

    #[test]
    fn security_scheme_roundtrip() {
        let schemes = vec![
            SecurityScheme::ApiKey {
                header: Some("X-Key".into()),
                query_param: None,
            },
            SecurityScheme::Http {
                scheme: "bearer".into(),
            },
            SecurityScheme::OAuth2 {
                authorization_url: "https://auth.example.com/authorize".into(),
                token_url: "https://auth.example.com/token".into(),
                scopes: vec!["agent:run".into()],
            },
        ];

        let json = serde_json::to_string_pretty(&schemes).unwrap();
        assert!(json.contains("api_key"));
        assert!(json.contains("http"));
        assert!(json.contains("oauth2"));

        let roundtrip: Vec<SecurityScheme> = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip.len(), 3);
    }
}
