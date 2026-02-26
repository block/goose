use super::CanonicalModelLoader;
use crate::providers::canonical::{CanonicalModel, Limit, Modalities, Modality, Pricing};
use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::Deserialize;

const NANOGPT_MODELS_URL: &str = "https://nano-gpt.com/api/v1/models?detailed=true";
const NANOGPT_PROVIDER: &str = "nano-gpt";

pub struct NanoGptLoader;

/// Fetch canonical models fresh from the NanoGPT API.
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

    let mut input_modalities = vec![Modality::Text];
    if caps.vision {
        input_modalities.push(Modality::Image);
    }
    if caps.pdf_upload {
        input_modalities.push(Modality::Pdf);
    }

    let release_date = model.created.map(timestamp_to_date);
    let display_name = model.name.unwrap_or_else(|| model.id.clone());

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