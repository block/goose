use super::*;

pub(super) const GEMINI25_DEFAULT_THINKING_BUDGET: i32 = 8192;

pub(super) fn get_thinking_config(model_config: &ModelConfig) -> Option<ThinkingConfig> {
    let model_name = model_config.model_name.to_lowercase();
    let is_gemini_3 = model_name.starts_with("gemini-3");
    let is_gemini_25 = model_name.starts_with("gemini-2.5");
    if !is_gemini_3 && !is_gemini_25 {
        return None;
    }

    if is_gemini_3 {
        use crate::model::ThinkingEffort;
        let effort = model_config
            .thinking_effort()
            .unwrap_or(ThinkingEffort::Off);
        if effort == ThinkingEffort::Off {
            return None;
        }
        let thinking_level = match effort {
            ThinkingEffort::Off | ThinkingEffort::Low | ThinkingEffort::Medium => {
                ThinkingLevel::Low
            }
            ThinkingEffort::High | ThinkingEffort::Max => ThinkingLevel::High,
        };

        Some(ThinkingConfig {
            thinking_level: Some(thinking_level),
            thinking_budget: None,
            include_thoughts: true,
        })
    } else {
        let thinking_budget = match model_config
            .get_config_param::<i32>("thinking_budget", "GEMINI25_THINKING_BUDGET")
        {
            Some(budget) if budget >= 0 => budget,
            Some(budget) => {
                tracing::warn!(
                    "Invalid thinking budget '{}' for model '{}'. Must be >= 0. Using '{}'.",
                    budget,
                    model_config.model_name,
                    GEMINI25_DEFAULT_THINKING_BUDGET,
                );
                GEMINI25_DEFAULT_THINKING_BUDGET
            }
            None => GEMINI25_DEFAULT_THINKING_BUDGET,
        };
        Some(ThinkingConfig {
            thinking_level: None,
            thinking_budget: Some(thinking_budget),
            include_thoughts: true,
        })
    }
}

pub fn create_request(
    model_config: &ModelConfig,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
) -> Result<Value> {
    let tools_wrapper = if tools.is_empty() {
        None
    } else {
        Some(ToolsWrapper {
            function_declarations: format_tools(tools),
        })
    };

    let thinking_config = get_thinking_config(model_config);

    let generation_config = Some(GenerationConfig {
        temperature: model_config.temperature.map(|t| t as f64),
        max_output_tokens: Some(model_config.max_output_tokens()),
        thinking_config,
    });

    let request = GoogleRequest {
        system_instruction: SystemInstruction {
            parts: [TextPart { text: system }],
        },
        contents: format_messages(messages),
        tools: tools_wrapper,
        generation_config,
    };

    Ok(serde_json::to_value(request)?)
}
