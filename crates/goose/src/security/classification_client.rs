use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Request format following HuggingFace Inference API specification
#[derive(Debug, Serialize)]
struct ClassificationRequest {
    inputs: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    parameters: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ClassificationLabel {
    label: String,
    score: f32,
}

type ClassificationResponse = Vec<Vec<ClassificationLabel>>;

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
            inputs: text.to_string(),
            parameters: None, // Reserved for future use (e.g., truncation, max_length)
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

        let batch_result = classification_response
            .first()
            .context("Classification API returned empty response")?;

        let top_label = batch_result
            .iter()
            .max_by(|a, b| {
                a.score
                    .partial_cmp(&b.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .context("Classification API returned no labels")?;

        let injection_score = match top_label.label.as_str() {
            "INJECTION" | "LABEL_1" => top_label.score,
            "SAFE" | "LABEL_0" => 1.0 - top_label.score,
            _ => {
                tracing::warn!(
                    label = %top_label.label,
                    score = %top_label.score,
                    "Unknown classification label, defaulting to safe"
                );
                0.0
            }
        };

        if !(0.0..=1.0).contains(&injection_score) {
            anyhow::bail!(
                "Calculated injection score is invalid: {} (must be between 0.0 and 1.0)",
                injection_score
            );
        }

        tracing::info!(
            injection_score = %injection_score,
            top_label = %top_label.label,
            top_score = %top_label.score,
            all_labels = ?batch_result,
            endpoint = %self.endpoint_url,
            "HTTP classification detector results"
        );

        Ok(injection_score)
    }
}

// TODO: add tests
