use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnSubAgentArgs {
    pub recipe_name: Option<String>,
    pub instructions: Option<String>,
    pub message: String,
    pub max_turns: Option<usize>,
    pub timeout_seconds: Option<u64>,
}

impl SpawnSubAgentArgs {
    pub fn new_with_recipe(recipe_name: String, message: String) -> Self {
        Self {
            recipe_name: Some(recipe_name),
            instructions: None,
            message,
            max_turns: None,
            timeout_seconds: None,
        }
    }

    pub fn new_with_instructions(instructions: String, message: String) -> Self {
        Self {
            recipe_name: None,
            instructions: Some(instructions),
            message,
            max_turns: None,
            timeout_seconds: None,
        }
    }

    pub fn with_max_turns(mut self, max_turns: usize) -> Self {
        self.max_turns = Some(max_turns);
        self
    }

    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.timeout_seconds = Some(timeout_seconds);
        self
    }
}
