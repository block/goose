use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use url::Url;

/// Request format following HuggingFace Inference Text Classification API specification
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

#[derive(Debug, Deserialize, Clone)]
pub struct ModelEndpointInfo {
    pub endpoint: String,
    #[serde(flatten)]
    pub extra_params: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct ModelMappingConfig {
    #[serde(flatten)]
    pub models: HashMap<String, ModelEndpointInfo>,
}

#[derive(Debug)]
pub struct ClassificationClient {
    endpoint_url: String,
    client: reqwest::Client,
    auth_token: Option<String>,
    extra_params: Option<HashMap<String, serde_json::Value>>,
}

impl ClassificationClient {
    pub fn new(
        endpoint_url: String,
        timeout_ms: Option<u64>,
        auth_token: Option<String>,
        extra_params: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<Self> {
        let timeout = Duration::from_millis(timeout_ms.unwrap_or(5000));

        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            endpoint_url,
            client,
            auth_token,
            extra_params,
        })
    }

    pub fn from_model_name(model_name: &str, timeout_ms: Option<u64>) -> Result<Self> {
        let mapping_json = std::env::var("ML_MODEL_MAPPING")
            .context("ML_MODEL_MAPPING environment variable not set")?;

        let mapping = serde_json::from_str::<ModelMappingConfig>(&mapping_json)
            .context("Failed to parse ML_MODEL_MAPPING JSON")?;

        let model_info = mapping.models.get(model_name).context(format!(
            "Model '{}' not found in ML_MODEL_MAPPING",
            model_name
        ))?;

        tracing::info!(
            model_name = %model_name,
            endpoint = %model_info.endpoint,
            extra_params = ?model_info.extra_params,
            "Creating classification client from model mapping"
        );

        Self::new(
            model_info.endpoint.clone(),
            timeout_ms,
            None,
            Some(model_info.extra_params.clone()),
        )
    }

    pub fn from_endpoint(
        endpoint_url: String,
        timeout_ms: Option<u64>,
        auth_token: Option<String>,
    ) -> Result<Self> {
        Url::parse(&endpoint_url)
            .context("Invalid endpoint URL format. Must be a valid HTTP/HTTPS URL")?;

        let auth_token = auth_token
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty());

        tracing::info!(
            endpoint = %endpoint_url,
            has_token = auth_token.is_some(),
            "Creating classification client from endpoint"
        );

        Self::new(endpoint_url, timeout_ms, auth_token, None)
    }

    pub async fn classify(&self, text: &str) -> Result<f32> {
        tracing::debug!(
            endpoint = %self.endpoint_url,
            text_length = text.len(),
            "Sending classification request"
        );

        let parameters = self
            .extra_params
            .as_ref()
            .map(|params| serde_json::to_value(params))
            .transpose()?;

        let request = ClassificationRequest {
            inputs: text.to_string(),
            parameters,
        };

        let mut request_builder = self.client.post(&self.endpoint_url).json(&request);

        if let Some(token) = &self.auth_token {
            request_builder = request_builder.header("Authorization", format!("Bearer {}", token));
        }

        let response = request_builder
            .send()
            .await
            .context("Failed to send classification request")?;

        let status = response.status();
        let response = if !status.is_success() {
            let error_body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Classification API returned error status {}: {}",
                status,
                error_body
            ));
        } else {
            response
        };

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

        tracing::info!(
            injection_score = %injection_score,
            top_label = %top_label.label,
            top_score = %top_label.score,
            "Classification complete"
        );

        Ok(injection_score)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_client() {
        let client = ClassificationClient::new(
            "http://localhost:8000/classify".to_string(),
            Some(3000),
            None,
            None,
        );
        assert!(client.is_ok());
    }

    #[test]
    fn test_new_client_with_auth_token() {
        let client = ClassificationClient::new(
            "http://localhost:8000/classify".to_string(),
            None,
            Some("test_token".to_string()),
            None,
        );
        assert!(client.is_ok());
        assert_eq!(client.unwrap().auth_token, Some("test_token".to_string()));
    }

    #[test]
    fn test_from_endpoint() {
        let client = ClassificationClient::from_endpoint(
            "http://localhost:8000/classify".to_string(),
            Some(3000),
            Some("test_token".to_string()),
        );
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.endpoint_url, "http://localhost:8000/classify");
        assert_eq!(client.auth_token, Some("test_token".to_string()));
        assert!(client.extra_params.is_none());
    }

    #[test]
    fn test_from_endpoint_without_token() {
        let client = ClassificationClient::from_endpoint(
            "http://localhost:8000/classify".to_string(),
            None,
            None,
        );
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.endpoint_url, "http://localhost:8000/classify");
        assert!(client.auth_token.is_none());
    }

    #[test]
    fn test_from_model_name_without_mapping() {
        // Should fail when ML_MODEL_MAPPING is not set
        std::env::remove_var("ML_MODEL_MAPPING");
        let result = ClassificationClient::from_model_name("test-model", None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("ML_MODEL_MAPPING"));
    }

    #[test]
    fn test_from_endpoint_invalid_url() {
        // Should fail with invalid URL
        let result = ClassificationClient::from_endpoint("not-a-valid-url".to_string(), None, None);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid endpoint URL"));
    }

    #[test]
    fn test_from_endpoint_valid_https_url() {
        let result = ClassificationClient::from_endpoint(
            "https://api.example.com/classify".to_string(),
            None,
            None,
        );
        assert!(result.is_ok());
    }
}
