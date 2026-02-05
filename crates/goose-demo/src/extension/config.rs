//! Extension configuration
//!
//! Defines the structure for extension configs loaded from file.

use serde::{Deserialize, Serialize};

/// Configuration for a single extension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionConfig {
    /// Unique name for this extension
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// How to instantiate this extension
    #[serde(flatten)]
    pub kind: ExtensionKind,
}

/// How an extension is implemented/instantiated
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum ExtensionKind {
    /// Native Rust implementation
    Native,

    /// MCP server process
    Mcp {
        /// Command to run (e.g., "npx", "uvx", "/path/to/binary")
        command: String,

        /// Arguments to the command
        #[serde(default)]
        args: Vec<String>,

        /// Environment variables to set
        #[serde(default)]
        env: std::collections::HashMap<String, String>,
    },
}

/// Root config file structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionsConfig {
    /// Map of extension name -> config
    #[serde(default)]
    pub extensions: std::collections::HashMap<String, ExtensionConfigEntry>,
}

/// Entry in the config file (name comes from the key)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionConfigEntry {
    /// Human-readable description
    pub description: String,

    /// How to instantiate this extension
    #[serde(flatten)]
    pub kind: ExtensionKind,
}

impl ExtensionConfigEntry {
    /// Convert to full ExtensionConfig with name
    pub fn into_config(self, name: String) -> ExtensionConfig {
        ExtensionConfig {
            name,
            description: self.description,
            kind: self.kind,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_native_extension() {
        let toml = r#"
[extensions.develop]
kind = "native"
description = "Shell and file editing tools"
"#;

        let config: ExtensionsConfig = toml::from_str(toml).unwrap();
        assert!(config.extensions.contains_key("develop"));

        let develop = &config.extensions["develop"];
        assert!(matches!(develop.kind, ExtensionKind::Native));
    }

    #[test]
    fn test_parse_mcp_extension() {
        let toml = r#"
[extensions.browser]
kind = "mcp"
command = "npx"
args = ["-y", "@anthropic/mcp-browser"]
description = "Web browsing tools"

[extensions.memory]
kind = "mcp"
command = "uvx"
args = ["mcp-memory"]
description = "Persistent memory"
"#;

        let config: ExtensionsConfig = toml::from_str(toml).unwrap();

        let browser = &config.extensions["browser"];
        match &browser.kind {
            ExtensionKind::Mcp { command, args, .. } => {
                assert_eq!(command, "npx");
                assert_eq!(args, &["-y", "@anthropic/mcp-browser"]);
            }
            _ => panic!("Expected MCP extension"),
        }
    }

    #[test]
    fn test_parse_mcp_with_env() {
        let toml = r#"
[extensions.github]
kind = "mcp"
command = "npx"
args = ["-y", "@anthropic/mcp-github"]
description = "GitHub tools"

[extensions.github.env]
GITHUB_TOKEN = "from-env"
"#;

        let config: ExtensionsConfig = toml::from_str(toml).unwrap();
        let github = &config.extensions["github"];

        match &github.kind {
            ExtensionKind::Mcp { env, .. } => {
                assert_eq!(env.get("GITHUB_TOKEN"), Some(&"from-env".to_string()));
            }
            _ => panic!("Expected MCP extension"),
        }
    }
}
