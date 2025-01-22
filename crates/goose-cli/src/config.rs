use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use goose::agents::ExtensionConfig;

const DEFAULT_EXTENSION: &str = "developer";

/// Core configuration for Goose CLI
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub default_provider: String,
    pub default_model: String,
    pub extensions: HashMap<String, ExtensionEntry>,
}

/// An extension configuration entry with an enabled flag and configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ExtensionEntry {
    pub enabled: bool,
    #[serde(flatten)]
    pub config: ExtensionConfig,
}

impl Config {
    /// Get the configuration file path
    pub fn config_path() -> Result<PathBuf> {
        let home_dir = dirs::home_dir().context("Could not determine home directory")?;
        let config_dir = home_dir.join(".config").join("goose");
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }
        Ok(config_dir.join("config.yaml"))
    }

    /// Load the configuration from disk
    pub fn load() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Err(anyhow::anyhow!("Config has not yet been created"));
        }
        let content = fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&content)?)
    }

    /// Save the configuration to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        let content = serde_yaml::to_string(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Get the extension configuration if enabled
    pub fn get_extension_config(&self, name: &str) -> Option<ExtensionConfig> {
        let entry = self.extensions.get(name)?;
        if entry.enabled {
            Some(entry.config.clone())
        } else {
            None
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_provider: "".to_string(),
            default_model: "".to_string(),
            extensions: HashMap::from([(
                DEFAULT_EXTENSION.to_string(),
                ExtensionEntry {
                    enabled: true,
                    config: ExtensionConfig::Builtin {
                        name: DEFAULT_EXTENSION.to_string(),
                    },
                },
            )]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// This test provides a comprehensive example of all possible configuration options
    /// and validates that they are correctly parsed
    #[test]
    fn test_comprehensive_config() {
        let yaml = r#"
# Core settings for the default provider and model
default_provider: openai
default_model: gpt-4

# Extension configurations showing all possible variants
extensions:
  # Built-in extension that just needs to be enabled
  developer:
    enabled: true
    type: builtin
    name: developer

  # Built-in extension that is disabled
  unused:
    enabled: false
    type: builtin
    name: unused

  # Full stdio extension configuration with all options
  python:
    enabled: true
    type: stdio
    cmd: python3
    args:
      - "-m"
      - "goose.extensions.python"
    envs:
      PYTHONPATH: /path/to/python
      DEBUG: "true"

  # Full SSE extension configuration
  remote:
    enabled: true
    type: sse
    uri: http://localhost:8000/events
    envs:
      API_KEY: secret
      DEBUG: "true"

  # Disabled full extension configuration
  disabled_extension:
    enabled: false
    type: stdio
    cmd: test
    args: []
    envs: {}
"#;
        let config: Config = serde_yaml::from_str(yaml).unwrap();

        // Check core settings
        assert_eq!(config.default_provider, "openai");
        assert_eq!(config.default_model, "gpt-4");

        // Check builtin enabled extension
        match &config.extensions.get("developer").unwrap().config {
            ExtensionConfig::Builtin { name } => assert_eq!(name, "developer"),
            _ => panic!("Expected builtin extension config"),
        }
        assert!(config.extensions.get("developer").unwrap().enabled);

        // Check builtin disabled extension
        match &config.extensions.get("unused").unwrap().config {
            ExtensionConfig::Builtin { name } => assert_eq!(name, "unused"),
            _ => panic!("Expected builtin extension config"),
        }
        assert!(!config.extensions.get("unused").unwrap().enabled);

        // Check full stdio extension
        let python = config.extensions.get("python").unwrap();
        assert!(python.enabled);
        match &python.config {
            ExtensionConfig::Stdio { cmd, args, envs } => {
                assert_eq!(cmd, "python3");
                assert_eq!(
                    args,
                    &vec!["-m".to_string(), "goose.extensions.python".to_string()]
                );
                let env = envs.get_env();
                assert_eq!(env.get("PYTHONPATH").unwrap(), "/path/to/python");
                assert_eq!(env.get("DEBUG").unwrap(), "true");
            }
            _ => panic!("Expected stdio extension config"),
        }

        // Check full SSE extension
        let remote = config.extensions.get("remote").unwrap();
        assert!(remote.enabled);
        match &remote.config {
            ExtensionConfig::Sse { uri, envs } => {
                assert_eq!(uri, "http://localhost:8000/events");
                let env = envs.get_env();
                assert_eq!(env.get("API_KEY").unwrap(), "secret");
                assert_eq!(env.get("DEBUG").unwrap(), "true");
            }
            _ => panic!("Expected sse extension config"),
        }

        // Check disabled full extension
        assert!(!config.extensions.get("disabled_extension").unwrap().enabled);

        // Test the get_extension_config helper
        assert!(config.get_extension_config("developer").is_some());
        assert!(config.get_extension_config("unused").is_none());
        assert!(config.get_extension_config("python").is_some());
        assert!(config.get_extension_config("remote").is_some());
        assert!(config.get_extension_config("disabled_extension").is_none());
        assert!(config.get_extension_config("nonexistent").is_none());
    }
}
