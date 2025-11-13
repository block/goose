//! Gondola provider for ML model inference.
//!
//! Unlike other providers in this module that implement the `Provider` trait for generative
//! LLM interactions (chat completions, streaming), this provider is designed for classification
//! and inference tasks using BERT-based models. It provides access to Gondola's BatchInfer API
//! for use cases like prompt injection detection.
//!
//! TODO: figure out if this is the right place for this.

use crate::config::Config;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug)]
pub struct GondolaProvider {
    endpoint: String,
    client: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct BatchInferRequest {
    model: String,
    version: String,
    source: String,
    input_names: Vec<String>,
    request_items: Vec<RequestItem>,
}

#[derive(Debug, Serialize)]
struct RequestItem {
    inputs: Vec<Input>,
}

#[derive(Debug, Serialize)]
struct Input {
    string_value: String,
}

#[derive(Debug, Deserialize)]
pub struct BatchInferResponse {
    pub model: String,
    pub version: String,
    pub occurred_at: String,
    pub response_items: Vec<ResponseItem>,
}

#[derive(Debug, Deserialize)]
pub struct ResponseItem {
    #[serde(default)]
    pub double_list_value: Option<DoubleListValue>,
}

#[derive(Debug, Deserialize)]
pub struct DoubleListValue {
    pub double_values: Vec<f64>,
}

impl GondolaProvider {
    pub fn new() -> Result<Self> {
        let config = Config::global();

        let endpoint = config
            .get_param::<String>("GONDOLA_ENDPOINT")
            .context("GONDOLA_ENDPOINT must be configured")?;

        Self::with_endpoint(&endpoint)
    }

    pub fn with_endpoint(endpoint: &str) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            endpoint: endpoint.to_string(),
            client,
        })
    }

    pub async fn batch_infer(
        &self,
        model: &str,
        version: &str,
        input_name: &str,
        texts: &[String],
    ) -> Result<BatchInferResponse> {
        let request = BatchInferRequest {
            model: model.to_string(),
            version: version.to_string(),
            source: "goose-security".to_string(),
            input_names: vec![input_name.to_string()],
            request_items: texts
                .iter()
                .map(|text| RequestItem {
                    inputs: vec![Input {
                        string_value: text.clone(),
                    }],
                })
                .collect(),
        };

        tracing::debug!(model = %model, version = %version, num_texts = texts.len(),
                   "Sending batch inference request to Gondola");

        let response = self
            .client
            .post(&self.endpoint)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Gondola")?;

        let json_response: BatchInferResponse = response
            .error_for_status()?
            .json()
            .await
            .context("Failed to parse Gondola response")?;

        tracing::debug!(
            response_length = json_response.response_items.len(),
            "Received response from Gondola"
        );

        Ok(json_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_gondola_provider_creation() {
        let provider = GondolaProvider::with_endpoint("https://endpoint.com/api");
        assert!(provider.is_ok());
        assert_eq!(provider.unwrap().endpoint, "https://endpoint.com/api");
    }

    #[test]
    fn test_batch_infer_request_serialization() {
        let request = BatchInferRequest {
            model: "test-model".to_string(),
            version: "v1".to_string(),
            source: "test".to_string(),
            input_names: vec!["text_input".to_string()],
            request_items: vec![RequestItem {
                inputs: vec![Input {
                    string_value: "test text".to_string(),
                }],
            }],
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["model"], "test-model");
        assert_eq!(json["version"], "v1");
        assert_eq!(json["input_names"][0], "text_input");
        assert_eq!(
            json["request_items"][0]["inputs"][0]["string_value"],
            "test text"
        );
    }

    #[test]
    fn test_batch_infer_response_deserialization() {
        let json_response = r#"{
            "model": "deberta-prompt-injection-v2",
            "version": "gmv-zve9abhxe9s7fq1zep5dxd807",
            "occurred_at": "1761793135063",
            "response_items": [
                {
                    "double_list_value": {
                        "double_values": [-8.34437084197998, 7.024641036987305]
                    }
                }
            ]
        }"#;

        let response: BatchInferResponse = serde_json::from_str(json_response).unwrap();
        assert_eq!(response.model, "deberta-prompt-injection-v2");
        assert_eq!(response.version, "gmv-zve9abhxe9s7fq1zep5dxd807");
        assert_eq!(response.response_items.len(), 1);

        let first_item = &response.response_items[0];
        assert!(first_item.double_list_value.is_some());

        let double_values = &first_item.double_list_value.as_ref().unwrap().double_values;
        assert_eq!(double_values.len(), 2);
        assert!((double_values[0] - (-8.34437084197998)).abs() < 0.0001);
        assert!((double_values[1] - 7.024641036987305).abs() < 0.0001);
    }

    #[tokio::test]
    async fn test_batch_infer_with_mock() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let response_body = json!({
            "model": "test-model",
            "version": "v1",
            "occurred_at": "123456789",
            "response_items": [
                {
                    "double_list_value": {
                        "double_values": [0.1, 0.9]
                    }
                }
            ]
        });

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let provider = GondolaProvider::with_endpoint(&mock_server.uri()).unwrap();
        let result = provider
            .batch_infer("test-model", "v1", "text_input", &["test".to_string()])
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.model, "test-model");
        assert_eq!(response.response_items.len(), 1);
    }

    #[tokio::test]
    async fn test_error_handling_non_200() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        let provider = GondolaProvider::with_endpoint(&mock_server.uri()).unwrap();
        let result = provider
            .batch_infer("test-model", "v1", "text_input", &["test".to_string()])
            .await;

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("500"));
    }

    #[tokio::test]
    async fn test_error_handling_invalid_json() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_string("invalid json"))
            .mount(&mock_server)
            .await;

        let provider = GondolaProvider::with_endpoint(&mock_server.uri()).unwrap();
        let result = provider
            .batch_infer("test-model", "v1", "text_input", &["test".to_string()])
            .await;

        assert!(result.is_err());
    }
}
