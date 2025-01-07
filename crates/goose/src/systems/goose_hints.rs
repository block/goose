use anyhow::Result as AnyhowResult;
use async_trait::async_trait;
use std::fs;

use crate::errors::{AgentError, AgentResult};
use crate::systems::System;
use mcp_core::{Content, Resource, Tool, ToolCall};

pub struct GooseHintsSystem {
    instructions: String,
}

impl Default for GooseHintsSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl GooseHintsSystem {
    pub fn new() -> Self {
        // Check for .goosehints in current directory
        let mut hints = Vec::new();

        if let Ok(cwd) = std::env::current_dir() {
            let local_hints = cwd.join(".goosehints");
            if local_hints.exists() {
                if let Ok(content) = fs::read_to_string(&local_hints) {
                    hints.push(content);
                }
            }
        }

        // Check for .goosehints in ~/.config/goose/
        if let Some(home) = dirs::home_dir() {
            let config_hints = home.join(".config").join("goose").join(".goosehints");
            if config_hints.exists() {
                if let Ok(content) = fs::read_to_string(&config_hints) {
                    hints.push(content);
                }
            }
        }

        // Join all hints with newlines
        let instructions = hints.join("\n");

        Self { instructions }
    }
}

#[async_trait]
impl System for GooseHintsSystem {
    fn name(&self) -> &str {
        "GooseHintsSystem"
    }

    fn description(&self) -> &str {
        "A system that provides initial context from .goosehints files."
    }

    fn instructions(&self) -> &str {
        &self.instructions
    }

    fn tools(&self) -> &[Tool] {
        &[]
    }

    async fn status(&self) -> AnyhowResult<Vec<Resource>> {
        Ok(Vec::new())
    }

    async fn call(&self, tool_call: ToolCall) -> AgentResult<Vec<Content>> {
        Err(AgentError::ToolNotFound(tool_call.name))
    }

    async fn read_resource(&self, _uri: &str) -> AgentResult<String> {
        Ok("".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_goose_hints_system() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a test .goosehints file
        let hints_path = temp_path.join(".goosehints");
        let mut file = File::create(&hints_path).unwrap();
        writeln!(file, "Test hint from local file").unwrap();

        std::env::set_current_dir(temp_path).unwrap();

        let system = GooseHintsSystem::new();

        assert!(system.instructions().contains("Test hint from local file"));
    }
}
