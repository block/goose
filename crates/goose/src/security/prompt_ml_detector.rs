use crate::providers::gondola::GondolaProvider;
use anyhow::{Context, Result};
use std::collections::HashMap;

pub struct MlDetector {
    provider: GondolaProvider,
    config: ModelConfig,
}

pub struct ModelConfig {
    pub model: String,
    pub version: String,
    pub input_name: String,
}

impl ModelConfig {
    // TODO: check if this is the best way to keep this registry (maybe there's a better solution that doesn't hardcode)
    fn model_registry() -> HashMap<&'static str, (&'static str, &'static str)> {
        let mut registry = HashMap::new();
        registry.insert(
            "deberta-prompt-injection-v2",
            ("gmv-zve9abhxe9s7fq1zep5dxd807", "text_input"),
        );
        registry
    }

    pub fn from_model_name(model_name: &str) -> Result<Self> {
        let registry = Self::model_registry();

        let (version, input_name) = registry.get(model_name).ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown model '{}'. Available models: {}",
                model_name,
                registry.keys().copied().collect::<Vec<_>>().join(", ")
            )
        })?;

        Ok(Self {
            model: model_name.to_string(),
            version: version.to_string(),
            input_name: input_name.to_string(),
        })
    }

    pub fn from_config() -> Result<Self> {
        let config = crate::config::Config::global();

        let model_name = config
            .get_param::<String>("SECURITY_PROMPT_ML_MODEL")
            .context(
                "ML model name not configured. Please set 'SECURITY_PROMPT_ML_MODEL' in settings.",
            )?;

        Self::from_model_name(&model_name)
    }
}

impl MlDetector {
    pub fn new(provider: GondolaProvider, config: ModelConfig) -> Self {
        Self { provider, config }
    }

    pub fn new_from_config() -> Result<Self> {
        let provider = GondolaProvider::new().context("Failed to initialize Gondola provider")?;

        let config = ModelConfig::from_config().context("Failed to load ML model configuration")?;

        Ok(Self::new(provider, config))
    }

    // TODO: truncation + whitespace elimination (leave for a follow-up PR)
    pub async fn scan(&self, text: &str) -> Result<f32> {
        tracing::debug!(
            text_length = text.len(),
            text_preview = %text.chars().take(100).collect::<String>(),
            "ML detection scanning text"
        );

        let response = self
            .provider
            .batch_infer(
                &self.config.model,
                &self.config.version,
                &self.config.input_name,
                &[text.to_string()],
            )
            .await
            .context("ML inference failed")?;

        let item = response
            .response_items
            .first()
            .context("No response items from ML model")?;

        let logits = item
            .double_list_value
            .as_ref()
            .context("No logits in response")?
            .double_values
            .as_slice();

        if logits.len() < 2 {
            anyhow::bail!("Expected 2 logits (safe, malicious), got {}", logits.len());
        }

        let exp_safe = logits[0].exp();
        let exp_malicious = logits[1].exp();
        let sum = exp_safe + exp_malicious;
        let confidence = (exp_malicious / sum) as f32;

        tracing::info!(
            logit_safe = %logits[0],
            logit_malicious = %logits[1],
            prob_safe = %(exp_safe / sum),
            prob_malicious = %(exp_malicious / sum),
            confidence = %confidence,
            "ML detection raw results"
        );

        Ok(confidence)
    }
}
