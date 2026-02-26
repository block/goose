use super::CanonicalModelLoader;
use crate::providers::canonical::{CanonicalModel, Limit, Modalities, Modality, Pricing};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;

const NANOGPT_MODELS_URL: &str = "https://nano-gpt.com/api/v1/models?detailed=true";
const NANOGPT_PROVIDER: &str = "nano-gpt";

pub struct NanoGptLoader;

/// Fetch canonical models fresh from the NanoGPT API.
///
/// This is the same loader used by `build_canonical_models`, but can be
/// called directly at runtime for fresher data.
pub async fn load_models() -> Result<Vec<CanonicalModel>> {
    NanoGptLoader.load_models().await
}

#[derive(Debug, Deserialize)]
struct NanoGptModelsResponse {
    data: Vec<NanoGptModel>,
}

#[derive(Debug, Deserialize)]
struct NanoGptModel {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    owned_by: Option<String>,
    #[serde(default)]
    context_length: Option<usize>,
    #[serde(default)]
    max_output_tokens: Option<usize>,
    #[serde(default)]
    capabilities: Option<NanoGptCapabilities>,
    #[serde(default)]
    pricing: Option<NanoGptPricing>,
    #[serde(default)]
    created: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
struct NanoGptCapabilities {
    #[serde(default)]
    vision: bool,
    #[serde(default)]
    reasoning: bool,
    #[serde(default)]
    tool_calling: bool,
    #[serde(default)]
    pdf_upload: bool,
}

#[derive(Debug, Deserialize, Default)]
struct NanoGptPricing {
    #[serde(default)]
    prompt: Option<f64>,
    #[serde(default)]
    completion: Option<f64>,
}

fn timestamp_to_date(ts: i64) -> String {
    let dt = chrono::DateTime::from_timestamp(ts, 0);
    dt.map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_default()
}

fn convert_model(model: NanoGptModel) -> CanonicalModel {
    let caps = model.capabilities.unwrap_or_default();
    let pricing = model.pricing.unwrap_or_default();

    // Build input modalities
    let mut input_modalities = vec![Modality::Text];
    if caps.vision {
        input_modalities.push(Modality::Image);
    }
    if caps.pdf_upload {
        input_modalities.push(Modality::Pdf);
    }

    let release_date = model.created.map(timestamp_to_date);
    let display_name = model
        .name
        .unwrap_or_else(|| model.id.clone());

    let canonical_id = format!("{}/{}", NANOGPT_PROVIDER, model.id);

    CanonicalModel {
        id: canonical_id,
        name: display_name,
        family: model.owned_by,
        attachment: Some(caps.vision || caps.pdf_upload),
        reasoning: Some(caps.reasoning),
        tool_call: caps.tool_calling,
        temperature: Some(true),
        knowledge: None,
        release_date,
        last_updated: None,
        modalities: Modalities {
            input: input_modalities,
            output: vec![Modality::Text],
        },
        open_weights: None,
        cost: Pricing {
            input: pricing.prompt,
            output: pricing.completion,
            cache_read: None,
            cache_write: None,
        },
        limit: Limit {
            context: model.context_length.unwrap_or(128_000),
            output: model.max_output_tokens,
        },
    }
}

#[async_trait]
impl CanonicalModelLoader for NanoGptLoader {
    fn provider_name(&self) -> &str {
        NANOGPT_PROVIDER
    }

    async fn load_models(&self) -> Result<Vec<CanonicalModel>> {
        let client = reqwest::Client::new();
        let response = client
            .get(NANOGPT_MODELS_URL)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .context("Failed to fetch NanoGPT models")?;

        let status = response.status();
        if !status.is_success() {
            anyhow::bail!("NanoGPT API returned status {}", status);
        }

        let body: NanoGptModelsResponse = response
            .json()
            .await
            .context("Failed to parse NanoGPT models response")?;

        let models: Vec<CanonicalModel> = body.data.into_iter().map(convert_model).collect();

        Ok(models)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_model_full() {
        let model = NanoGptModel {
            id: "claude-sonnet-4-5-20250929".to_string(),
            name: Some("Claude Sonnet 4.5".to_string()),
            owned_by: Some("anthropic".to_string()),
            context_length: Some(1_000_000),
            max_output_tokens: Some(64_000),
            capabilities: Some(NanoGptCapabilities {
                vision: true,
                reasoning: false,
                tool_calling: true,
                pdf_upload: true,
            }),
            pricing: Some(NanoGptPricing {
                prompt: Some(2.992),
                completion: Some(14.994),
            }),
            created: Some(1759104000),
        };

        let canonical = convert_model(model);

        assert_eq!(canonical.id, "nano-gpt/claude-sonnet-4-5-20250929");
        assert_eq!(canonical.name, "Claude Sonnet 4.5");
        assert_eq!(canonical.family, Some("anthropic".to_string()));
        assert_eq!(canonical.limit.context, 1_000_000);
        assert_eq!(canonical.limit.output, Some(64_000));
        assert!(canonical.tool_call);
        assert_eq!(canonical.reasoning, Some(false));
        assert_eq!(canonical.cost.input, Some(2.992));
        assert_eq!(canonical.cost.output, Some(14.994));
        assert!(canonical.modalities.input.contains(&Modality::Text));
        assert!(canonical.modalities.input.contains(&Modality::Image));
        assert!(canonical.modalities.input.contains(&Modality::Pdf));
    }

    #[test]
    fn test_convert_model_minimal() {
        let model = NanoGptModel {
            id: "auto-model".to_string(),
            name: Some("Auto model".to_string()),
            owned_by: None,
            context_length: None,
            max_output_tokens: None,
            capabilities: None,
            pricing: None,
            created: None,
        };

        let canonical = convert_model(model);

        assert_eq!(canonical.id, "nano-gpt/auto-model");
        assert_eq!(canonical.name, "Auto model");
        assert_eq!(canonical.family, None);
        assert_eq!(canonical.limit.context, 128_000); // default
        assert_eq!(canonical.limit.output, None);
        assert!(!canonical.tool_call);
        assert_eq!(canonical.cost.input, None);
    }

    #[test]
    fn test_timestamp_to_date() {
        assert_eq!(timestamp_to_date(1759104000), "2025-09-29");
        assert_eq!(timestamp_to_date(0), "1970-01-01");
    }
}
