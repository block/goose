use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;

use super::types::HookEventKind;

#[derive(Debug, Clone, Default)]
pub struct HookSettingsFile {
    pub hooks: HashMap<HookEventKind, Vec<HookEventConfig>>,
    pub allow_project_hooks: bool,
}

impl<'de> serde::Deserialize<'de> for HookSettingsFile {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Raw {
            #[serde(default)]
            hooks: HashMap<String, Vec<HookEventConfig>>,
            #[serde(default)]
            allow_project_hooks: bool,
        }

        let raw = Raw::deserialize(deserializer)?;
        let mut hooks = HashMap::new();

        for (key, configs) in raw.hooks {
            match HookEventKind::from_str(&key) {
                Ok(event) => {
                    hooks.insert(event, configs);
                }
                Err(_) => {
                    tracing::warn!("Unknown hook event '{}', ignoring", key);
                }
            }
        }

        Ok(Self {
            hooks,
            allow_project_hooks: raw.allow_project_hooks,
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookEventConfig {
    #[serde(default)]
    pub matcher: Option<String>,

    pub hooks: Vec<HookAction>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum HookAction {
    Command {
        command: String,

        #[serde(default = "default_timeout")]
        timeout: u64,
    },
    #[serde(rename = "mcp_tool")]
    McpTool {
        tool: String,
        #[serde(default)]
        arguments: serde_json::Map<String, serde_json::Value>,
        #[serde(default = "default_timeout")]
        timeout: u64,
    },
}

fn default_timeout() -> u64 {
    600
}

impl HookSettingsFile {
    pub fn load_merged(working_dir: &Path) -> Result<Self> {
        let global_path = crate::config::paths::Paths::in_config_dir("hooks.json");
        let goose_project_path = working_dir.join(".goose").join("settings.json");
        let claude_project_path = working_dir.join(".claude").join("settings.json");

        let global = Self::load_from_file(&global_path).unwrap_or_else(|e| {
            tracing::debug!("No global hooks config at {:?}: {}", global_path, e);
            Self::default()
        });

        // Use the allow_project_hooks setting from the global config
        let allow_project_hooks = global.allow_project_hooks;

        // If project hooks are not allowed, check if they exist and log a warning
        if !allow_project_hooks {
            let project_path = if goose_project_path.exists() {
                Some(&goose_project_path)
            } else if claude_project_path.exists() {
                Some(&claude_project_path)
            } else {
                None
            };

            if let Some(path) = project_path {
                tracing::info!(
                    "Project hooks found at {:?} but project hooks are not enabled. Set allow_project_hooks: true in ~/.config/goose/hooks.json to enable.",
                    path
                );
            }

            return Ok(global);
        }

        let project = if goose_project_path.exists() {
            if claude_project_path.exists() {
                tracing::warn!("Found hooks config in both .goose/ and .claude/; using .goose/");
            }
            Self::load_from_file(&goose_project_path).unwrap_or_else(|e| {
                tracing::warn!(
                    "Failed to parse hooks config {:?}: {}",
                    goose_project_path,
                    e
                );
                Self::default()
            })
        } else {
            Self::load_from_file(&claude_project_path).unwrap_or_else(|e| {
                if claude_project_path.exists() {
                    tracing::warn!(
                        "Failed to parse hooks config {:?}: {}",
                        claude_project_path,
                        e
                    );
                }
                Self::default()
            })
        };

        Ok(Self::merge(global, project))
    }

    fn load_from_file(path: &Path) -> Result<Self> {
        if !path.exists() {
            anyhow::bail!("Config file does not exist: {:?}", path);
        }

        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read hooks config from {:?}", path))?;

        let config: Self = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse hooks config from {:?}", path))?;

        Ok(config)
    }

    fn merge(global: Self, project: Self) -> Self {
        let mut merged_hooks: HashMap<HookEventKind, Vec<HookEventConfig>> = global.hooks;

        for (event, project_configs) in project.hooks {
            merged_hooks
                .entry(event)
                .or_default()
                .extend(project_configs);
        }

        Self {
            hooks: merged_hooks,
            allow_project_hooks: global.allow_project_hooks,
        }
    }

    pub fn get_hooks_for_event(&self, event: HookEventKind) -> &[HookEventConfig] {
        self.hooks.get(&event).map(|v| v.as_slice()).unwrap_or(&[])
    }
}
