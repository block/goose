use crate::config::Config;
use crate::security::classification_client::ClassificationClient;
use anyhow::{Context, Result};

pub struct MlDetector {
    client: ClassificationClient,
}

impl MlDetector {
    pub fn new(client: ClassificationClient) -> Self {
        Self { client }
    }

    pub fn new_from_config() -> Result<Self> {
        let config = Config::global();

        let endpoint = config
            .get_param::<String>("SECURITY_PROMPT_ML_ENDPOINT")
            .context("ML endpoint not configured.")?;

        let auth_token = config.get_param::<String>("SECURITY_PROMPT_ML_TOKEN").ok();

        let client = ClassificationClient::new(endpoint, None, auth_token)?;

        Ok(Self::new(client))
    }

    pub async fn scan(&self, text: &str) -> Result<f32> {
        tracing::debug!(
            text_length = text.len(),
            text_preview = %text.chars().take(100).collect::<String>(),
            "ML detection scanning text"
        );

        let score = self.client.classify(text).await?;

        tracing::info!(
            score = %score,
            "ML detection result"
        );

        Ok(score)
    }
}
