use crate::utils::styles::Theme;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CustomCommand {
    pub name: String,
    pub description: String,
    pub tool: String,
    pub args: serde_json::Value,
}

#[derive(Clone)]
pub struct TuiConfig {
    pub theme: Theme,
    pub custom_commands: Vec<CustomCommand>,
}

impl TuiConfig {
    pub fn load() -> Result<Self> {
        let theme_name = "gemini";
        let config_path = Self::get_config_path();
        let custom_commands = if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            serde_json::from_str(&content).unwrap_or_else(|_| Self::default_commands())
        } else {
            Self::default_commands()
        };

        Ok(Self {
            theme: Theme::from_name(theme_name),
            custom_commands,
        })
    }

    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path();
        let content = serde_json::to_string_pretty(&self.custom_commands)?;
        fs::write(config_path, content)?;
        Ok(())
    }

    fn get_config_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".goose_tui_commands.json")
    }

    fn default_commands() -> Vec<CustomCommand> {
        vec![
            CustomCommand {
                name: "ls".to_string(),
                description: "List files".to_string(),
                tool: "developer__shell".to_string(),
                args: serde_json::json!({ "command": "ls -la" }),
            },
            CustomCommand {
                name: "status".to_string(),
                description: "Git Status".to_string(),
                tool: "developer__shell".to_string(),
                args: serde_json::json!({ "command": "git status" }),
            },
        ]
    }
}
