//! Prompt-cache semantics declared per (provider, model), instead of implied
//! by whichever format module a request flows through.

use serde_json::{json, Value};

use crate::formats::openai::is_openai_responses_model;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheSemantics {
    /// Caller places markers; reuse needs an exact match of the marked bytes.
    ExplicitBreakpoints { max_breakpoints: usize },
    /// The longest matching stored prefix is reused implicitly.
    ImplicitTolerant,
    /// Only extends a prompt reproduced byte-for-byte from the start.
    ImplicitStrict,
    /// No known prompt cache.
    Uncached,
}

impl CacheSemantics {
    /// Unknown pairs default to `ImplicitStrict`, which is safe for every
    /// cache.
    pub fn for_model(provider_name: &str, model_name: &str) -> Self {
        match provider_name {
            "anthropic" | "minimax" | "zai" | "kimi_code" => {
                CacheSemantics::ExplicitBreakpoints { max_breakpoints: 4 }
            }
            "aws_bedrock" | "databricks" | "gcp_vertex_ai" if model_name.contains("claude") => {
                CacheSemantics::ExplicitBreakpoints { max_breakpoints: 4 }
            }
            "openrouter" | "litellm" if model_name.starts_with("anthropic/") => {
                CacheSemantics::ExplicitBreakpoints { max_breakpoints: 4 }
            }
            "openai" | "azure_openai" | "github_copilot" => {
                if is_openai_responses_model(model_name) {
                    CacheSemantics::ImplicitStrict
                } else {
                    CacheSemantics::ImplicitTolerant
                }
            }
            "moonshot" | "custom_deepseek" | "groq" | "together" | "fireworks-ai" | "mistral"
            | "zhipu" | "alibaba" => CacheSemantics::ImplicitTolerant,
            "snowflake" | "sagemaker_tgi" => CacheSemantics::Uncached,
            _ => CacheSemantics::ImplicitStrict,
        }
    }

    pub fn uses_explicit_breakpoints(self) -> bool {
        matches!(self, CacheSemantics::ExplicitBreakpoints { .. })
    }
}

/// Anthropic-dialect breakpoints for OpenAI-style chat payloads (OpenRouter,
/// LiteLLM, Databricks).
pub fn apply_chat_payload_breakpoints(payload: &mut Value) {
    if let Some(messages) = payload
        .get_mut("messages")
        .and_then(|messages| messages.as_array_mut())
    {
        let mut user_count = 0;
        for message in messages.iter_mut().rev() {
            if message.get("role") != Some(&json!("user")) {
                continue;
            }
            if mark_last_content_block(message) {
                user_count += 1;
                if user_count >= 2 {
                    break;
                }
            }
        }

        if let Some(system_message) = messages
            .iter_mut()
            .find(|message| message.get("role") == Some(&json!("system")))
        {
            mark_last_content_block(system_message);
        }
    }

    if let Some(last_tool) = payload
        .get_mut("tools")
        .and_then(|tools| tools.as_array_mut())
        .and_then(|tools| tools.last_mut())
    {
        if let Some(function) = last_tool.get_mut("function").and_then(Value::as_object_mut) {
            function.insert("cache_control".to_string(), json!({ "type": "ephemeral" }));
        }
    }
}

fn mark_last_content_block(message: &mut Value) -> bool {
    let Some(content) = message.get_mut("content") else {
        return false;
    };
    if let Some(text) = content.as_str() {
        *content = json!([{
            "type": "text",
            "text": text,
            "cache_control": { "type": "ephemeral" }
        }]);
        return true;
    }
    if let Some(block) = content
        .as_array_mut()
        .and_then(|blocks| blocks.last_mut())
        .and_then(Value::as_object_mut)
    {
        block.insert("cache_control".to_string(), json!({ "type": "ephemeral" }));
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn for_model_resolves_registered_identifiers_and_defaults_to_strict() {
        let explicit = CacheSemantics::ExplicitBreakpoints { max_breakpoints: 4 };
        for (provider, model, expected) in [
            ("anthropic", "claude-sonnet-4-5", explicit),
            ("kimi_code", "kimi-for-coding", explicit),
            ("aws_bedrock", "anthropic.claude-sonnet-4-5", explicit),
            ("minimax", "MiniMax-M2.5", explicit),
            ("openrouter", "anthropic/claude-sonnet-4-5", explicit),
            (
                "openrouter",
                "openai/gpt-5.2",
                CacheSemantics::ImplicitStrict,
            ),
            ("openai", "gpt-5.2", CacheSemantics::ImplicitStrict),
            ("openai", "gpt-4.1-mini", CacheSemantics::ImplicitTolerant),
            ("snowflake", "llama-3.3-70b", CacheSemantics::Uncached),
            (
                "some_new_vendor",
                "some-model",
                CacheSemantics::ImplicitStrict,
            ),
        ] {
            assert_eq!(
                CacheSemantics::for_model(provider, model),
                expected,
                "{provider}/{model}"
            );
        }
    }

    #[test]
    fn breakpoints_cover_system_tools_and_last_two_user_messages() {
        let mut payload = json!({
            "model": "anthropic/claude-sonnet-4-5",
            "messages": [
                {"role": "system", "content": "be careful"},
                {"role": "user", "content": "first"},
                {"role": "assistant", "content": "ok"},
                {"role": "user", "content": "second"},
                {"role": "assistant", "content": "ok"},
                {"role": "user", "content": "third"}
            ],
            "tools": [
                {"type": "function", "function": {"name": "a"}},
                {"type": "function", "function": {"name": "b"}}
            ]
        });
        apply_chat_payload_breakpoints(&mut payload);

        let messages = payload["messages"].as_array().unwrap();
        assert_eq!(
            messages[0]["content"][0]["cache_control"]["type"],
            "ephemeral"
        );
        assert!(messages[1]["content"].is_string(), "only last two users");
        assert_eq!(
            messages[3]["content"][0]["cache_control"]["type"],
            "ephemeral"
        );
        assert_eq!(
            messages[5]["content"][0]["cache_control"]["type"],
            "ephemeral"
        );
        let tools = payload["tools"].as_array().unwrap();
        assert!(tools[0]["function"].get("cache_control").is_none());
        assert_eq!(tools[1]["function"]["cache_control"]["type"], "ephemeral");
    }

    #[test]
    fn array_content_user_messages_get_a_breakpoint() {
        let mut payload = json!({
            "messages": [
                {"role": "user", "content": [
                    {"type": "text", "text": "look at this"},
                    {"type": "image_url", "image_url": {"url": "data:image/png;base64,xyz"}}
                ]}
            ]
        });
        apply_chat_payload_breakpoints(&mut payload);
        let blocks = payload["messages"][0]["content"].as_array().unwrap();
        assert!(blocks[0].get("cache_control").is_none());
        assert_eq!(blocks[1]["cache_control"]["type"], "ephemeral");
    }
}
