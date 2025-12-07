use crate::utils::styles::Theme;
use anyhow::Result;
use goose::config::Config;
use serde::{Deserialize, Serialize};

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
    pub smart_context: bool,
}

impl TuiConfig {
    pub fn load() -> Result<Self> {
        let global = Config::global();

        let theme_name = global
            .get_param::<String>("tui_theme")
            .unwrap_or_else(|_| "goose".to_string());

        let custom_commands = global
            .get_param::<Vec<CustomCommand>>("tui_custom_commands")
            .unwrap_or_else(|_| Self::default_commands());

        let smart_context = global
            .get_param::<bool>("tui_smart_context")
            .unwrap_or(true);

        Ok(Self {
            theme: Theme::from_name(&theme_name),
            custom_commands,
            smart_context,
        })
    }

    pub fn save(&self) -> Result<()> {
        let global = Config::global();
        global.set_param("tui_custom_commands", &self.custom_commands)?;
        Ok(())
    }

    pub fn save_theme(&self) -> Result<()> {
        let global = Config::global();
        global.set_param("tui_theme", &self.theme.name)?;
        Ok(())
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
