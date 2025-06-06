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
    model_config: ModelConfig,
    system_prompt: &str,
    messages: &[Message],
    schema: JsonValueFfi,
) -> Result<ProviderExtractResponse, ProviderError> {
    let provider = create(provider_name, provider_config, model_config)?;
    let resp = provider.extract(system_prompt, messages, &schema).await?;

    Ok(resp)
}
