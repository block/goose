use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;

use crate::agents::extension::ExtensionInfo;
use crate::agents::router_tool_selector::RouterToolSelectionStrategy;
use crate::agents::router_tools::{llm_search_tool_prompt, vector_search_tool_prompt};
use crate::providers::base::get_current_model;
use crate::{config::Config, prompt_template};

pub struct PromptManager {
    system_prompt_override: Option<String>,
    system_prompt_extras: Vec<String>,
    current_date_timestamp: String,
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
            // Use the fixed current date time so that prompt cache can be used.
            current_date_timestamp: Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
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

    /// Normalize a model name (replace - and / with _, lower case)
    fn normalize_model_name(name: &str) -> String {
        name.replace(['-', '/', '.'], "_").to_lowercase()
    }

    /// Map model (normalized) to prompt filenames; returns filename if a key is contained in the normalized model
    fn model_prompt_map(model: &str) -> &'static str {
        let mut map = HashMap::new();
        map.insert("gpt_4_1", "system_gpt_4.1.md");
        // Add more mappings as needed
        let norm_model = Self::normalize_model_name(model);
        for (key, val) in &map {
            if norm_model.contains(key) {
                return val;
            }
        }
        "system.md"
    }

    /// Build the final system prompt
    ///
    /// * `extensions_info` – extension information for each extension/MCP
    /// * `frontend_instructions` – instructions for the "frontend" tool
    pub fn build_system_prompt(
        &self,
        extensions_info: Vec<ExtensionInfo>,
        frontend_instructions: Option<String>,
        suggest_disable_extensions_prompt: Value,
        model_name: Option<&str>,
        tool_selection_strategy: Option<RouterToolSelectionStrategy>,
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

        match tool_selection_strategy {
            Some(RouterToolSelectionStrategy::Vector) => {
                context.insert(
                    "tool_selection_strategy",
                    Value::String(vector_search_tool_prompt()),
                );
            }
            Some(RouterToolSelectionStrategy::Llm) => {
                context.insert(
                    "tool_selection_strategy",
                    Value::String(llm_search_tool_prompt()),
                );
            }
            None => {}
        }

        context.insert(
            "current_date_time",
            Value::String(self.current_date_timestamp.clone()),
        );

        // Add the suggestion about disabling extensions if flag is true
        context.insert(
            "suggest_disable",
            Value::String(suggest_disable_extensions_prompt.to_string()),
        );

        // First check the global store, and only if it's not available, fall back to the provided model_name
        let model_to_use: Option<String> =
            get_current_model().or_else(|| model_name.map(|s| s.to_string()));

        // Conditionally load the override prompt or the global system prompt
        let base_prompt = if let Some(override_prompt) = &self.system_prompt_override {
            prompt_template::render_inline_once(override_prompt, &context)
                .expect("Prompt should render")
        } else if let Some(model) = &model_to_use {
            // Use the fuzzy mapping to determine the prompt file, or fall back to legacy logic
            let prompt_file = Self::model_prompt_map(model);
            match prompt_template::render_global_file(prompt_file, &context) {
                Ok(prompt) => prompt,
                Err(_) => {
                    // Fall back to the standard system.md if model-specific one doesn't exist
                    prompt_template::render_global_file("system.md", &context)
                        .expect("Prompt should render")
                }
            }
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

    pub async fn get_recipe_prompt(&self) -> String {
        let context: HashMap<&str, Value> = HashMap::new();
        prompt_template::render_global_file("recipe.md", &context).expect("Prompt should render")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_model_name() {
        assert_eq!(PromptManager::normalize_model_name("gpt-4.1"), "gpt_4_1");
        assert_eq!(PromptManager::normalize_model_name("gpt/3.5"), "gpt_3_5");
        assert_eq!(
            PromptManager::normalize_model_name("GPT-3.5/PLUS"),
            "gpt_3_5_plus"
        );
    }

    #[test]
    fn test_model_prompt_map_matches() {
        // should match prompts based on contained normalized keys
        assert_eq!(
            PromptManager::model_prompt_map("gpt-4.1"),
            "system_gpt_4.1.md"
        );

        assert_eq!(
            PromptManager::model_prompt_map("gpt-4.1-2025-04-14"),
            "system_gpt_4.1.md"
        );

        assert_eq!(
            PromptManager::model_prompt_map("openai/gpt-4.1"),
            "system_gpt_4.1.md"
        );
        assert_eq!(
            PromptManager::model_prompt_map("goose-gpt-4-1"),
            "system_gpt_4.1.md"
        );
        assert_eq!(
            PromptManager::model_prompt_map("gpt-4-1-huge"),
            "system_gpt_4.1.md"
        );
    }

    #[test]
    fn test_model_prompt_map_none() {
        // should return system.md for unrecognized/unsupported model names
        assert_eq!(PromptManager::model_prompt_map("llama-3-70b"), "system.md");
        assert_eq!(PromptManager::model_prompt_map("goose"), "system.md");
        assert_eq!(
            PromptManager::model_prompt_map("claude-3.7-sonnet"),
            "system.md"
        );
        assert_eq!(
            PromptManager::model_prompt_map("xxx-unknown-model"),
            "system.md"
        );
    }
}
