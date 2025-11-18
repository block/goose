use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Serialize)]
struct ClassificationRequest {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ClassificationResponse {
    score: f32,
    label: Option<String>,
}

pub struct ClassificationClient {
    endpoint_url: String,
    client: reqwest::Client,
    timeout: Duration,
}

impl ClassificationClient {
    pub fn new(endpoint_url: String, timeout_ms: Option<u64>) -> Result<Self> {
        let timeout = Duration::from_millis(timeout_ms.unwrap_or(5000));

        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            endpoint_url,
            client,
            timeout,
        })
    }

    pub async fn classify(&self, text: &str) -> Result<f32> {
        tracing::debug!(
            endpoint = %self.endpoint_url,
            text_length = text.len(),
            text_preview = %text.chars().take(100).collect::<String>(),
            timeout_ms = ?self.timeout.as_millis(),
            "HTTP classification detector scanning text"
        );

        let request = ClassificationRequest {
            text: text.to_string(),
            model: None,   // Reserved for future use
            options: None, // Reserved for future use
        };

        let response = self
            .client
            .post(&self.endpoint_url)
            .json(&request)
            .send()
            .await
            .context("Failed to send classification request")?;

        let status = response.status();
        if !status.is_success() {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read error response".to_string());
            anyhow::bail!(
                "Classification API returned error status {}: {}",
                status,
                error_body
            );
        }

        let classification_response: ClassificationResponse = response
            .json()
            .await
            .context("Failed to parse classification response")?;

        let score = classification_response.score;

        if !(0.0..=1.0).contains(&score) {
            anyhow::bail!(
                "Classification API returned invalid score: {} (must be between 0.0 and 1.0)",
                score
            );
        }

        tracing::info!(
            score = %score,
            label = ?classification_response.label,
            endpoint = %self.endpoint_url,
            "HTTP classification detector results"
        );

        Ok(score)
    }
}

// TODO: add tests
