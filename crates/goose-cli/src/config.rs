use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use goose::agents::SystemConfig;

const DEFAULT_SYSTEM: &str = "developer";

/// Core configuration for Goose CLI
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub default_provider: String,
    pub default_model: String,
    pub systems: HashMap<String, SystemEntry>,
}

/// A system configuration entry with an enabled flag and configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SystemEntry {
    pub enabled: bool,
    #[serde(flatten)]
    pub config: SystemConfig,
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

    /// Get the system configuration if enabled
    pub fn get_system_config(&self, name: &str) -> Option<SystemConfig> {
        let entry = self.systems.get(name)?;
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
            systems: HashMap::from([(
                DEFAULT_SYSTEM.to_string(),
                SystemEntry {
                    enabled: true,
                    config: SystemConfig::Builtin {
                        name: DEFAULT_SYSTEM.to_string(),
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

# System configurations showing all possible variants
systems:
  # Built-in system that just needs to be enabled
  developer:
    enabled: true
    type: builtin
    name: developer

  # Built-in system that is disabled
  unused:
    enabled: false
    type: builtin
    name: unused

  # Full stdio system configuration with all options
  python:
    enabled: true
    type: stdio
    cmd: python3
    args:
      - "-m"
      - "goose.systems.python"
    envs:
      PYTHONPATH: /path/to/python
      DEBUG: "true"

  # Full SSE system configuration
  remote:
    enabled: true
    type: sse
    uri: http://localhost:8000/events
    envs:
      API_KEY: secret
      DEBUG: "true"

  # Disabled full system configuration
  disabled_system:
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

        // Check builtin enabled system
        match &config.systems.get("developer").unwrap().config {
            SystemConfig::Builtin { name } => assert_eq!(name, "developer"),
            _ => panic!("Expected builtin system config"),
        }
        assert!(config.systems.get("developer").unwrap().enabled);

        // Check builtin disabled system
        match &config.systems.get("unused").unwrap().config {
            SystemConfig::Builtin { name } => assert_eq!(name, "unused"),
            _ => panic!("Expected builtin system config"),
        }
        assert!(!config.systems.get("unused").unwrap().enabled);

        // Check full stdio system
        let python = config.systems.get("python").unwrap();
        assert!(python.enabled);
        match &python.config {
            SystemConfig::Stdio { cmd, args, envs } => {
                assert_eq!(cmd, "python3");
                assert_eq!(
                    args,
                    &vec!["-m".to_string(), "goose.systems.python".to_string()]
                );
                let env = envs.get_env();
                assert_eq!(env.get("PYTHONPATH").unwrap(), "/path/to/python");
                assert_eq!(env.get("DEBUG").unwrap(), "true");
            }
            _ => panic!("Expected stdio system config"),
        }

        // Check full SSE system
        let remote = config.systems.get("remote").unwrap();
        assert!(remote.enabled);
        match &remote.config {
            SystemConfig::Sse { uri, envs } => {
                assert_eq!(uri, "http://localhost:8000/events");
                let env = envs.get_env();
                assert_eq!(env.get("API_KEY").unwrap(), "secret");
                assert_eq!(env.get("DEBUG").unwrap(), "true");
            }
            _ => panic!("Expected sse system config"),
        }

        // Check disabled full system
        assert!(!config.systems.get("disabled_system").unwrap().enabled);

        // Test the get_system_config helper
        assert!(config.get_system_config("developer").is_some());
        assert!(config.get_system_config("unused").is_none());
        assert!(config.get_system_config("python").is_some());
        assert!(config.get_system_config("remote").is_some());
        assert!(config.get_system_config("disabled_system").is_none());
        assert!(config.get_system_config("nonexistent").is_none());
    }
}
