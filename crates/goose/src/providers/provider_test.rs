use crate::{conversation::message::Message, providers::create};
use anyhow::Result;
use futures::StreamExt;
use rmcp::model::ToolAnnotations;
use rmcp::{model::Tool, object};
use std::time::Duration;
use tokio::time::timeout;

const PROVIDER_TEST_TIMEOUT: Duration = Duration::from_secs(60);

pub fn provider_model_validation_enabled() -> bool {
    !matches!(
        std::env::var("GOOSE_SKIP_PROVIDER_MODEL_VALIDATION"),
        Ok(value) if value == "1" || value.eq_ignore_ascii_case("true")
    )
}

pub fn toolshim_settings_from_env() -> (Option<bool>, Option<String>) {
    let toolshim_enabled = std::env::var("GOOSE_TOOLSHIM")
        .map(|val| val == "1" || val.to_lowercase() == "true")
        .ok();
    let toolshim_model = std::env::var("GOOSE_TOOLSHIM_OLLAMA_MODEL").ok();
    (toolshim_enabled, toolshim_model)
}

pub async fn test_provider_model(provider_name: &str, model: &str) -> Result<()> {
    if !provider_model_validation_enabled() {
        return Ok(());
    }
    test_provider_configuration(provider_name, model, None, None).await
}

pub async fn test_provider_configuration(
    provider_name: &str,
    model: &str,
    toolshim_enabled: Option<bool>,
    toolshim_model: Option<String>,
) -> Result<()> {
    let mut model_config =
        crate::model_config::model_config_from_user_config(provider_name, model)?
            .with_max_tokens(Some(50));

    if let Some(toolshim_enabled) = toolshim_enabled {
        model_config = model_config.with_toolshim(toolshim_enabled);
    }
    if toolshim_model.is_some() {
        model_config = model_config.with_toolshim_model(toolshim_model);
    }

    let provider = create(provider_name, Vec::new()).await?;

    let messages =
        vec![Message::user().with_text("What is the weather like in San Francisco today?")];

    let tools = if !model_config.toolshim {
        vec![create_sample_weather_tool()]
    } else {
        vec![]
    };

    timeout(PROVIDER_TEST_TIMEOUT, async {
        let mut stream = crate::session_context::with_session_id(
            Some("test-session-id".to_string()),
            provider.stream(
                &model_config,
                "You are an AI agent called goose. You use tools of connected extensions to solve problems.",
                &messages,
                &tools.into_iter().collect::<Vec<_>>(),
            ),
        )
        .await?;

        let first_chunk = stream
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("Provider test stream returned no events"))?;
        first_chunk?;

        Ok::<(), anyhow::Error>(())
    })
    .await
    .map_err(|_| {
        anyhow::anyhow!(
            "Provider configuration test timed out after {}s",
            PROVIDER_TEST_TIMEOUT.as_secs()
        )
    })??;

    Ok(())
}

fn create_sample_weather_tool() -> Tool {
    Tool::new(
        "get_weather".to_string(),
        "Get current temperature for a given location.".to_string(),
        object!({
            "type": "object",
            "required": ["location"],
            "properties": {
                "location": {"type": "string"}
            }
        }),
    )
    .annotate(
        ToolAnnotations::with_title("Get weather".to_string())
            .read_only(true)
            .destructive(false)
            .idempotent(false)
            .open_world(false),
    )
}
