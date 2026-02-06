use super::EditorModelImpl;
use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct OpenAICompatibleEditor {
    api_key: String,
    host: String,
    model: String,
}

impl OpenAICompatibleEditor {
    pub fn new(api_key: String, host: String, model: String) -> Self {
        Self {
            api_key,
            host,
            model,
        }
    }
}

impl EditorModelImpl for OpenAICompatibleEditor {
    async fn edit_code(
        &self,
        original_code: &str,
        _old_str: &str,
        update_snippet: &str,
    ) -> Result<String, String> {
        let provider_url = if self.host.ends_with("/chat/completions") {
            self.host.clone()
        } else if self.host.ends_with('/') {
            format!("{}chat/completions", self.host)
        } else {
            format!("{}/chat/completions", self.host)
        };

        let client = Client::new();

        let user_prompt = format!(
            "<code>{}</code>\n<update>{}</update>",
            original_code, update_snippet
        );

        let body = json!({
            "model": self.model,
            "messages": [
                {
                    "role": "user",
                    "content": user_prompt
                }
            ]
        });

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

        if !response.status().is_success() {
            return Err(format!("API error: HTTP {}", response.status()));
        }

        let response_json: Value = match response.json().await {
            Ok(json) => json,
            Err(e) => return Err(format!("Failed to parse response: {}", e)),
        };

        let content = response_json
            .get("choices")
            .and_then(|choices| choices.get(0))
            .and_then(|choice| choice.get("message"))
            .and_then(|message| message.get("content"))
            .and_then(|content| content.as_str())
            .ok_or_else(|| "Invalid response format".to_string())?;

        Ok(content.to_string())
    }

    fn get_str_replace_description(&self) -> &'static str {
        "Edit the file with the new content."
    }
}
