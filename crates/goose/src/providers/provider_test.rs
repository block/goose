use crate::{conversation::message::Message, model::ModelConfig, providers::create};
use anyhow::Result;
use rmcp::model::ToolAnnotations;
use rmcp::{model::Tool, object};

pub async fn test_provider_configuration(
    provider_name: &str,
    model: &str,
    toolshim_enabled: bool,
    toolshim_model: Option<String>,
) -> Result<()> {
    let model_config = ModelConfig::new(model)?
        .with_max_tokens(Some(50))
        .with_toolshim(toolshim_enabled)
        .with_toolshim_model(toolshim_model);

    let provider = create(provider_name, model_config).await?;

    let messages =
        vec![Message::user().with_text("What is the weather like in San Francisco today?")];

    let tools = if !toolshim_enabled {
        vec![create_sample_weather_tool()]
    } else {
        vec![]
    };

    let _result = provider
        .complete(
            "You are an AI agent called goose. You use tools of connected extensions to solve problems.",
            &messages,
            &tools.into_iter().collect::<Vec<_>>()
        )
        .await?;

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
    .annotate(ToolAnnotations {
        title: Some("Get weather".to_string()),
        read_only_hint: Some(true),
        destructive_hint: Some(false),
        idempotent_hint: Some(false),
        open_world_hint: Some(false),
    })
}
