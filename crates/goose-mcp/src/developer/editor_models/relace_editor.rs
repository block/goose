use super::EditorModelImpl;
use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};

/// Relace-specific editor that uses the predicted outputs convention
#[derive(Debug, Clone)]
pub struct RelaceEditor {
    api_key: String,
    host: String,
    model: String,
}

impl RelaceEditor {
    pub fn new(api_key: String, host: String, model: String) -> Self {
        Self {
            api_key,
            host,
            model,
        }
    }
}

impl EditorModelImpl for RelaceEditor {
    async fn edit_code(
        &self,
        original_code: &str,
        _old_str: &str,
        update_snippet: &str,
    ) -> Result<String, String> {
        eprintln!("Calling Relace Editor API");

        // Construct the full URL
        let provider_url = if self.host.ends_with("/chat/completions") {
            self.host.clone()
        } else if self.host.ends_with('/') {
            format!("{}chat/completions", self.host)
        } else {
            format!("{}/chat/completions", self.host)
        };

        // Create the client
        let client = Client::new();

        // Prepare the request body for Relace API
        // The Relace endpoint expects the OpenAI predicted outputs convention
        // where the original code is supplied under `prediction` and the
        // update snippet is the sole user message.
        let body = json!({
            "model": self.model,
            "prediction": {
                "content": original_code
            },
            "messages": [
                {
                    "role": "user",
                    "content": update_snippet
                }
            ]
        });

        // Send the request
        let response = match client
            .post(&provider_url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => return Err(format!("Request error: {}", e)),
        };

        // Process the response
        if !response.status().is_success() {
            return Err(format!("API error: HTTP {}", response.status()));
        }

        // Parse the JSON response
        let response_json: Value = match response.json().await {
            Ok(json) => json,
            Err(e) => return Err(format!("Failed to parse response: {}", e)),
        };

        // Extract the content from the response
        let content = response_json
            .get("choices")
            .and_then(|choices| choices.get(0))
            .and_then(|choice| choice.get("message"))
            .and_then(|message| message.get("content"))
            .and_then(|content| content.as_str())
            .ok_or_else(|| "Invalid response format".to_string())?;

        eprintln!("Relace Editor API worked");
        Ok(content.to_string())
    }

    fn get_str_replace_description(&self) -> &'static str {
        "edit_file will take the new_str and work out how to place old_str with it intelligently."
    }
}
