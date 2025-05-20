use crate::{
    providers::{create, errors::ProviderError, ProviderExtractResponse},
    types::json_value_ffi::JsonValueFfi,
    Message, ModelConfig,
};

/// Generates a structured output based on the provided schema,
/// system prompt and user messages.
#[uniffi::export(async_runtime = "tokio")]
pub async fn generate_structured_outputs(
    provider_name: &str,
    provider_config: JsonValueFfi,
    system_prompt: &str,
    messages: &[Message],
    schema: JsonValueFfi,
) -> Result<ProviderExtractResponse, ProviderError> {
    // Use OpenAI models specifically for this task
    let model_name = if provider_name == "databricks" {
        "goose-gpt-4-1"
    } else {
        "gpt-4.1"
    };
    let model_cfg = ModelConfig::new(model_name.to_string()).with_temperature(Some(0.0));
    let provider = create(provider_name, provider_config, model_cfg)?;

    let resp = provider.extract(system_prompt, messages, &schema).await?;

    Ok(resp)
}
