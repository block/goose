//! Regression tests for the reasoning-model `max_tokens` budget (issue #11142).
//!
//! OpenAI-compatible servers (llama.cpp, vLLM, Ollama) count `reasoning_content`
//! and `content` against a single `max_tokens`, so a verbose thinking pass can
//! consume the whole budget and leave an empty answer. The request builder in
//! [`super::openai`] must reserve budget for both.

use crate::images::ImageFormat;
use crate::model::ModelConfig;
use crate::thinking::ThinkingEffort;
use serde_json::json;

use super::openai::create_request;

fn reasoning_model_config(max_tokens: i32, effort: Option<ThinkingEffort>) -> ModelConfig {
    let mut config = ModelConfig::new("nemotron-35-lightning").with_max_tokens(Some(max_tokens));
    if let Some(effort) = effort {
        config = config.with_thinking_effort(effort);
    }
    config.reasoning = Some(true);
    config
}

#[test]
fn reasoning_model_max_tokens_include_thinking_budget() -> anyhow::Result<()> {
    // A custom OpenAI-compatible provider serving a local reasoning model
    // (llama.cpp / vLLM / Ollama). These servers count reasoning_content and
    // content against a single max_tokens, so a verbose thinking pass can
    // consume the whole budget and leave an empty answer (#11142). The request
    // must reserve budget for both.
    let model_config = reasoning_model_config(4096, Some(ThinkingEffort::High));

    let request = create_request(
        &model_config,
        "system",
        &[],
        &[],
        &ImageFormat::OpenAi,
        true,
    )?;

    // 4096 content budget + 16384 thinking budget = 20480.
    assert_eq!(request["max_tokens"], json!(20480));

    Ok(())
}

#[test]
fn reasoning_model_max_tokens_unchanged_without_thinking() -> anyhow::Result<()> {
    // Same reasoning model, but no thinking effort configured: the budget must
    // be left untouched.
    let model_config = reasoning_model_config(4096, None);

    let request = create_request(
        &model_config,
        "system",
        &[],
        &[],
        &ImageFormat::OpenAi,
        true,
    )?;

    assert_eq!(request["max_tokens"], json!(4096));

    Ok(())
}

#[test]
fn reasoning_model_max_tokens_unchanged_when_thinking_off() -> anyhow::Result<()> {
    // When thinking is explicitly disabled, no extra budget is needed.
    let model_config = reasoning_model_config(4096, Some(ThinkingEffort::Off));

    let request = create_request(
        &model_config,
        "system",
        &[],
        &[],
        &ImageFormat::OpenAi,
        true,
    )?;

    assert_eq!(request["max_tokens"], json!(4096));

    Ok(())
}

#[test]
fn responses_model_max_completion_tokens_unchanged() -> anyhow::Result<()> {
    // The OpenAI Responses API already counts reasoning tokens inside
    // max_completion_tokens; inflating it risks exceeding the hard cap.
    let model_config = ModelConfig::new("gpt-5.4")
        .with_thinking_effort(ThinkingEffort::High)
        .with_max_tokens(Some(16384));

    let request = create_request(
        &model_config,
        "system",
        &[],
        &[],
        &ImageFormat::OpenAi,
        true,
    )?;

    assert_eq!(request["max_completion_tokens"], json!(16384));

    Ok(())
}

#[test]
fn chat_reasoning_model_max_tokens_clamped_at_provider_output_cap() -> anyhow::Result<()> {
    // A hosted OpenAI-compatible reasoning model whose canonical max_tokens is
    // ALREADY the provider's advertised output limit (x-ai/grok-4.3,
    // limit.output 30000). Reserving a thinking budget on top would push the
    // request past the model's hard cap (30000 + 16384 = 46384), so the chat
    // `max_tokens` must stay at the cap.
    let model_config = ModelConfig::new("grok-4.3")
        .with_canonical_limits("xai")
        .with_thinking_effort(ThinkingEffort::High);

    // Sanity check: the canonical output limit is what landed in max_tokens.
    assert_eq!(model_config.max_tokens, Some(30000));

    let request = create_request(
        &model_config,
        "system",
        &[],
        &[],
        &ImageFormat::OpenAi,
        true,
    )?;

    // Effective max_tokens stays at the provider cap, not 30000 + 16384.
    assert_eq!(request["max_tokens"], json!(30000));

    Ok(())
}

#[test]
fn chat_reasoning_model_max_tokens_still_tops_up_with_headroom() -> anyhow::Result<()> {
    // Same hosted reasoning model, but the user set max_tokens below the cap
    // (GOOSE_MAX_TOKENS=10000 < limit.output 30000). The thinking top-up must
    // still be applied because there is headroom left under the hard cap.
    let model_config = ModelConfig::new("grok-4.3")
        .with_max_tokens(Some(10000))
        .with_canonical_limits("xai")
        .with_thinking_effort(ThinkingEffort::High);

    let request = create_request(
        &model_config,
        "system",
        &[],
        &[],
        &ImageFormat::OpenAi,
        true,
    )?;

    // 10000 content budget + 16384 thinking budget = 26384 (<= 30000 cap).
    assert_eq!(request["max_tokens"], json!(26384));

    Ok(())
}
