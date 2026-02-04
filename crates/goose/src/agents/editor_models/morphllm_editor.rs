use super::EditorModelImpl;
use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};

#[derive(Debug, Clone)]
pub struct MorphLLMEditor {
    api_key: String,
    host: String,
    model: String,
}

impl MorphLLMEditor {
    pub fn new(api_key: String, host: String, model: String) -> Self {
        Self {
            api_key,
            host,
            model,
        }
    }

    fn extract_tag_content(text: &str, tag_name: &str) -> Option<String> {
        let start_tag = format!("<{}>", tag_name);
        let end_tag = format!("</{}>", tag_name);

        if let (Some(start_pos), Some(end_pos)) = (text.find(&start_tag), text.find(&end_tag)) {
            if start_pos < end_pos {
                let content_start = start_pos + start_tag.len();
                if let Some(content) = text.get(content_start..end_pos) {
                    return Some(content.trim().to_string());
                }
            }
        }
        None
    }

    fn format_user_prompt(original_code: &str, update_snippet: &str) -> String {
        if let Some(code_content) = Self::extract_tag_content(update_snippet, "code") {
            if let Some(instruction_content) =
                Self::extract_tag_content(update_snippet, "instruction")
            {
                return format!(
                    "<instruction>{}</instruction>\n<code>{}</code>\n<update>{}</update>",
                    instruction_content, original_code, code_content
                );
            }
            return format!(
                "<code>{}</code>\n<update>{}</update>",
                original_code, code_content
            );
        }
        format!(
            "<code>{}</code>\n<update>{}</update>",
            original_code, update_snippet
        )
    }
}

impl EditorModelImpl for MorphLLMEditor {
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
        let user_prompt = Self::format_user_prompt(original_code, update_snippet);

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
        "Use the edit_file to propose an edit to an existing file.
        This will be read by a less intelligent model, which will quickly apply the edit. You should make it clear what the edit is, while also minimizing the unchanged code you write.

        **IMPORTANT**: in the new_str parameter, you must also provide an `instruction` - a single sentence written in the first person describing what you are going to do for the sketched edit.
        This instruction helps the less intelligent model understand and apply your edit correctly.

         Examples of good instructions:
        - I am adding error handling to the user authentication function and removing the old authentication method
        - The instruction should be specific enough to disambiguate any uncertainty in your edit.

        The format for new_str should be like this example:

        <code>
          new code here you want to add
        </code>
        <instruction>
         adding new code with error handling
        </instruction>

        provide this to new_str as a single string.

        When writing the edit, you should specify each edit in sequence, with the special comment // ... existing code ... to represent unchanged code in between edited lines.

        For example:
        // ... existing code ...
        FIRST_EDIT
        // ... existing code ...
        SECOND_EDIT
        // ... existing code ...
        THIRD_EDIT
        // ... existing code ...

        You should bias towards repeating as few lines of the original file as possible to convey the change.
        Each edit should contain sufficient context of unchanged lines around the code you're editing to resolve ambiguity.
        If you plan on deleting a section, you must provide surrounding context to indicate the deletion.
        DO NOT omit spans of pre-existing code without using the // ... existing code ... comment to indicate its absence.
        "
    }
}
