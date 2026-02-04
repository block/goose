//! Slash Commands Module - Built-in and custom command handling
//!
//! Provides:
//! - Built-in commands (/help, /clear, /compact, /status, /config, /memory, /plan)
//! - Custom recipe-based commands
//! - Command parsing and argument extraction
//! - Command execution results

use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::config::Config;
use crate::recipe::Recipe;

const SLASH_COMMANDS_CONFIG_KEY: &str = "slash_commands";

/// Built-in slash commands
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuiltinCommand {
    Help,
    Clear,
    Compact,
    Status,
    Config,
    Memory,
    Plan,
    Resume,
    Stop,
    Undo,
    Bug,
    Cost,
    Permissions,
    Theme,
    Doctor,
    Init,
    Login,
    Logout,
    Vim,
    Terminal,
}

impl BuiltinCommand {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "help" | "?" => Some(BuiltinCommand::Help),
            "clear" => Some(BuiltinCommand::Clear),
            "compact" => Some(BuiltinCommand::Compact),
            "status" => Some(BuiltinCommand::Status),
            "config" | "configuration" => Some(BuiltinCommand::Config),
            "memory" | "memories" => Some(BuiltinCommand::Memory),
            "plan" => Some(BuiltinCommand::Plan),
            "resume" | "continue" => Some(BuiltinCommand::Resume),
            "stop" | "cancel" => Some(BuiltinCommand::Stop),
            "undo" => Some(BuiltinCommand::Undo),
            "bug" | "report" => Some(BuiltinCommand::Bug),
            "cost" | "costs" | "usage" => Some(BuiltinCommand::Cost),
            "permissions" | "perms" => Some(BuiltinCommand::Permissions),
            "theme" => Some(BuiltinCommand::Theme),
            "doctor" => Some(BuiltinCommand::Doctor),
            "init" => Some(BuiltinCommand::Init),
            "login" => Some(BuiltinCommand::Login),
            "logout" => Some(BuiltinCommand::Logout),
            "vim" => Some(BuiltinCommand::Vim),
            "terminal" | "term" => Some(BuiltinCommand::Terminal),
            _ => None,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            BuiltinCommand::Help => "Show available commands and usage",
            BuiltinCommand::Clear => "Clear the current conversation",
            BuiltinCommand::Compact => "Compact the conversation to reduce context",
            BuiltinCommand::Status => "Show current session status",
            BuiltinCommand::Config => "View or modify configuration",
            BuiltinCommand::Memory => "View or manage conversation memories",
            BuiltinCommand::Plan => "View or modify the current plan",
            BuiltinCommand::Resume => "Resume a previous session",
            BuiltinCommand::Stop => "Stop the current operation",
            BuiltinCommand::Undo => "Undo the last action",
            BuiltinCommand::Bug => "Report a bug or issue",
            BuiltinCommand::Cost => "Show token usage and cost",
            BuiltinCommand::Permissions => "Manage tool permissions",
            BuiltinCommand::Theme => "Change the UI theme",
            BuiltinCommand::Doctor => "Diagnose configuration issues",
            BuiltinCommand::Init => "Initialize a new project",
            BuiltinCommand::Login => "Authenticate with the service",
            BuiltinCommand::Logout => "Sign out of the service",
            BuiltinCommand::Vim => "Toggle vim mode",
            BuiltinCommand::Terminal => "Open terminal",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            BuiltinCommand::Help,
            BuiltinCommand::Clear,
            BuiltinCommand::Compact,
            BuiltinCommand::Status,
            BuiltinCommand::Config,
            BuiltinCommand::Memory,
            BuiltinCommand::Plan,
            BuiltinCommand::Resume,
            BuiltinCommand::Stop,
            BuiltinCommand::Undo,
            BuiltinCommand::Bug,
            BuiltinCommand::Cost,
            BuiltinCommand::Permissions,
            BuiltinCommand::Theme,
            BuiltinCommand::Doctor,
            BuiltinCommand::Init,
            BuiltinCommand::Login,
            BuiltinCommand::Logout,
            BuiltinCommand::Vim,
            BuiltinCommand::Terminal,
        ]
    }
}

impl std::fmt::Display for BuiltinCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            BuiltinCommand::Help => "help",
            BuiltinCommand::Clear => "clear",
            BuiltinCommand::Compact => "compact",
            BuiltinCommand::Status => "status",
            BuiltinCommand::Config => "config",
            BuiltinCommand::Memory => "memory",
            BuiltinCommand::Plan => "plan",
            BuiltinCommand::Resume => "resume",
            BuiltinCommand::Stop => "stop",
            BuiltinCommand::Undo => "undo",
            BuiltinCommand::Bug => "bug",
            BuiltinCommand::Cost => "cost",
            BuiltinCommand::Permissions => "permissions",
            BuiltinCommand::Theme => "theme",
            BuiltinCommand::Doctor => "doctor",
            BuiltinCommand::Init => "init",
            BuiltinCommand::Login => "login",
            BuiltinCommand::Logout => "logout",
            BuiltinCommand::Vim => "vim",
            BuiltinCommand::Terminal => "terminal",
        };
        write!(f, "/{}", name)
    }
}

/// Parsed slash command
#[derive(Debug, Clone)]
pub enum ParsedCommand {
    Builtin {
        command: BuiltinCommand,
        args: Vec<String>,
    },
    Recipe {
        path: PathBuf,
        recipe: Box<Recipe>,
        args: Vec<String>,
    },
    Unknown {
        command: String,
        args: Vec<String>,
    },
}

impl ParsedCommand {
    pub fn is_builtin(&self) -> bool {
        matches!(self, ParsedCommand::Builtin { .. })
    }

    pub fn is_recipe(&self) -> bool {
        matches!(self, ParsedCommand::Recipe { .. })
    }
}

/// Result of executing a slash command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandResult {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl CommandResult {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
}

/// Parse a slash command from input
pub fn parse_command(input: &str) -> Option<ParsedCommand> {
    let trimmed = input.trim();
    if !trimmed.starts_with('/') {
        return None;
    }

    let parts: Vec<&str> = trimmed
        .strip_prefix('/')
        .unwrap_or(trimmed)
        .split_whitespace()
        .collect();
    if parts.is_empty() {
        return None;
    }

    let command_name = parts[0];
    let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

    // Check built-in commands first
    if let Some(builtin) = BuiltinCommand::from_str(command_name) {
        return Some(ParsedCommand::Builtin {
            command: builtin,
            args,
        });
    }

    // Check custom recipe commands
    if let Some(recipe) = resolve_slash_command(command_name) {
        if let Some(path) = get_recipe_for_command(command_name) {
            return Some(ParsedCommand::Recipe {
                path,
                recipe: Box::new(recipe),
                args,
            });
        }
    }

    // Unknown command
    Some(ParsedCommand::Unknown {
        command: command_name.to_string(),
        args,
    })
}

/// Generate help text for all commands
pub fn generate_help() -> String {
    let mut help = String::from("# Available Commands\n\n");

    help.push_str("## Built-in Commands\n\n");
    for cmd in BuiltinCommand::all() {
        help.push_str(&format!("- **{}** - {}\n", cmd, cmd.description()));
    }

    let custom_commands = list_commands();
    if !custom_commands.is_empty() {
        help.push_str("\n## Custom Commands\n\n");
        for mapping in custom_commands {
            help.push_str(&format!(
                "- **/{0}** - Recipe: {1}\n",
                mapping.command, mapping.recipe_path
            ));
        }
    }

    help
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashCommandMapping {
    pub command: String,
    pub recipe_path: String,
}

pub fn list_commands() -> Vec<SlashCommandMapping> {
    Config::global()
        .get_param(SLASH_COMMANDS_CONFIG_KEY)
        .unwrap_or_else(|err| {
            warn!(
                "Failed to load {}: {}. Falling back to empty list.",
                SLASH_COMMANDS_CONFIG_KEY, err
            );
            Vec::new()
        })
}

fn save_slash_commands(commands: Vec<SlashCommandMapping>) -> Result<()> {
    Config::global()
        .set_param(SLASH_COMMANDS_CONFIG_KEY, &commands)
        .map_err(|e| anyhow::anyhow!("Failed to save slash commands: {}", e))
}

pub fn set_recipe_slash_command(recipe_path: PathBuf, command: Option<String>) -> Result<()> {
    let recipe_path_str = recipe_path.to_string_lossy().to_string();

    let mut commands = list_commands();
    commands.retain(|mapping| mapping.recipe_path != recipe_path_str);

    if let Some(cmd) = command {
        let normalized_cmd = cmd.trim_start_matches('/').to_lowercase();
        if !normalized_cmd.is_empty() {
            commands.push(SlashCommandMapping {
                command: normalized_cmd,
                recipe_path: recipe_path_str,
            });
        }
    }

    save_slash_commands(commands)
}

pub fn get_recipe_for_command(command: &str) -> Option<PathBuf> {
    let normalized = command.trim_start_matches('/').to_lowercase();
    let commands = list_commands();
    commands
        .into_iter()
        .find(|mapping| mapping.command == normalized)
        .map(|mapping| PathBuf::from(mapping.recipe_path))
}

pub fn resolve_slash_command(command: &str) -> Option<Recipe> {
    let recipe_path = get_recipe_for_command(command)?;

    if !recipe_path.exists() {
        return None;
    }
    let recipe_content = std::fs::read_to_string(&recipe_path).ok()?;
    let recipe = Recipe::from_content(&recipe_content).ok()?;

    Some(recipe)
}
