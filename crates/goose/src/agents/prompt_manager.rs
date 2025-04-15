use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;

use crate::agents::extension::ExtensionInfo;
use crate::{config::Config, prompt_template};

pub struct PromptManager {
    system_prompt_override: Option<String>,
    system_prompt_extras: Vec<String>,
}

impl Default for PromptManager {
    fn default() -> Self {
        PromptManager::new()
    }
}

impl PromptManager {
    pub fn new() -> Self {
        PromptManager {
            system_prompt_override: None,
            system_prompt_extras: Vec::new(),
        }
    }

    /// Add an additional instruction to the system prompt
    pub fn add_system_prompt_extra(&mut self, instruction: String) {
        self.system_prompt_extras.push(instruction);
    }

    /// Override the system prompt with custom text
    pub fn set_system_prompt_override(&mut self, template: String) {
        self.system_prompt_override = Some(template);
    }

    /// Build the final system prompt
    ///
    /// * `extensions_info` – extension information for each extension/MCP
    /// * `frontend_instructions` – instructions for the "frontend" tool
    pub fn build_system_prompt(
        &self,
        extensions_info: Vec<ExtensionInfo>,
        frontend_instructions: Option<String>,
    ) -> String {
        let mut context: HashMap<&str, Value> = HashMap::new();
        let mut extensions_info = extensions_info.clone();

        // Add frontend instructions to extensions_info to simplify json rendering
        if let Some(frontend_instructions) = frontend_instructions {
            extensions_info.push(ExtensionInfo::new(
                "frontend",
                &frontend_instructions,
                false,
            ));
        }

        context.insert("extensions", serde_json::to_value(extensions_info).unwrap());

        let current_date_time = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        context.insert("current_date_time", Value::String(current_date_time));

        // Conditionally load the override prompt or the global system prompt
        let base_prompt = if let Some(override_prompt) = &self.system_prompt_override {
            prompt_template::render_inline_once(override_prompt, &context)
                .expect("Prompt should render")
        } else {
            prompt_template::render_global_file("system.md", &context)
                .expect("Prompt should render")
        };

        let mut system_prompt_extras = self.system_prompt_extras.clone();
        let config = Config::global();
        let goose_mode = config.get_param("GOOSE_MODE").unwrap_or("auto".to_string());
        if goose_mode == "chat" {
            system_prompt_extras.push(
                "Right now you are in the chat only mode, no access to any tool use and system."
                    .to_string(),
            );
        } else {
            system_prompt_extras
                .push("Right now you are *NOT* in the chat only mode and have access to tool use and system.".to_string());
        }

        if system_prompt_extras.is_empty() {
            base_prompt
        } else {
            format!(
                "{}\n\n# Additional Instructions:\n\n{}",
                base_prompt,
                system_prompt_extras.join("\n\n")
            )
        }
    }

    /// Get the recipe prompt
    pub async fn get_recipe_prompt(&self) -> String {
        let context: HashMap<&str, Value> = HashMap::new();
        prompt_template::render_global_file("recipe.md", &context).expect("Prompt should render")
    }
}
