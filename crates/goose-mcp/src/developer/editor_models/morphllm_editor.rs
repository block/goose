use super::EditorModelImpl;
use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};

/// MorphLLM editor that uses the standard chat completions format
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

    /// Extract content between XML tags
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
            // Look for instruction tags which help provide hints
            if let Some(instruction_content) =
                Self::extract_tag_content(update_snippet, "instruction")
            {
                // Both code and instruction tags found
                return format!(
                    "<instruction>{}</instruction>\n<code>{}</code>\n<update>{}</update>",
                    instruction_content, original_code, code_content
                );
            }
            // Only code tags found, no instruction
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

        // Parse update_snippet for <code> and <instruction> tags
        let user_prompt = Self::format_user_prompt(original_code, update_snippet);

        // Prepare the request body for OpenAI-compatible API
        let body = json!({
            "model": self.model,
            "messages": [
                {
                    "role": "user",
                    "content": user_prompt
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_tag_content_valid() {
        let text = "<code>fn main() {}</code>";
        let result = MorphLLMEditor::extract_tag_content(text, "code");
        assert_eq!(result, Some("fn main() {}".to_string()));
    }

    #[test]
    fn test_extract_tag_content_with_whitespace() {
        let text = "<instruction>  I am adding a print statement  </instruction>";
        let result = MorphLLMEditor::extract_tag_content(text, "instruction");
        assert_eq!(result, Some("I am adding a print statement".to_string()));
    }

    #[test]
    fn test_extract_tag_content_invalid_order() {
        let text = "</code>Invalid<code>";
        let result = MorphLLMEditor::extract_tag_content(text, "code");
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_tag_content_missing_end_tag() {
        let text = "<code>fn main() {}";
        let result = MorphLLMEditor::extract_tag_content(text, "code");
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_tag_content_missing_start_tag() {
        let text = "fn main() {}</code>";
        let result = MorphLLMEditor::extract_tag_content(text, "code");
        assert_eq!(result, None);
    }

    #[test]
    fn test_extract_tag_content_nested_tags() {
        let text = "<code>fn main() { <code>nested</code> }</code>";
        let result = MorphLLMEditor::extract_tag_content(text, "code");
        assert_eq!(result, Some("fn main() { <code>nested".to_string()));
    }

    #[test]
    fn test_format_user_prompt_no_tags() {
        let original_code = "fn main() {}";
        let update_snippet = "Add error handling";
        let result = MorphLLMEditor::format_user_prompt(original_code, update_snippet);
        assert_eq!(
            result,
            "<code>fn main() {}</code>\n<update>Add error handling</update>"
        );
    }

    #[test]
    fn test_format_user_prompt_with_code_tags_only() {
        let original_code = "fn main() {}";
        let update_snippet = "<code>fn main() { println!(\"Hello\"); }</code>";
        let result = MorphLLMEditor::format_user_prompt(original_code, update_snippet);
        assert_eq!(
            result,
            "<code>fn main() {}</code>\n<update>fn main() { println!(\"Hello\"); }</update>"
        );
    }

    #[test]
    fn test_format_user_prompt_with_both_tags() {
        let original_code = "fn main() {}";
        let update_snippet = "<code>fn main() { println!(\"Hello\"); }</code><instruction>I am adding a print statement</instruction>";
        let result = MorphLLMEditor::format_user_prompt(original_code, update_snippet);
        assert_eq!(
            result,
            "<instruction>I am adding a print statement</instruction>\n<code>fn main() {}</code>\n<update>fn main() { println!(\"Hello\"); }</update>"
        );
    }

    #[test]
    fn test_format_user_prompt_with_whitespace() {
        let original_code = "fn main() {}";
        let update_snippet = "<code>  fn main() { println!(\"Hello\"); }  </code><instruction>  I am adding a print statement  </instruction>";
        let result = MorphLLMEditor::format_user_prompt(original_code, update_snippet);
        assert_eq!(
            result,
            "<instruction>I am adding a print statement</instruction>\n<code>fn main() {}</code>\n<update>fn main() { println!(\"Hello\"); }</update>"
        );
    }

    #[test]
    fn test_format_user_prompt_invalid_code_tags() {
        let original_code = "fn main() {}";
        let update_snippet = "</code>Invalid<code>";
        let result = MorphLLMEditor::format_user_prompt(original_code, update_snippet);
        assert_eq!(
            result,
            "<code>fn main() {}</code>\n<update></code>Invalid<code></update>"
        );
    }

    #[test]
    fn test_format_user_prompt_invalid_instruction_tags() {
        let original_code = "fn main() {}";
        let update_snippet =
            "<code>fn main() { println!(\"Hello\"); }</code></instruction>Invalid<instruction>";
        let result = MorphLLMEditor::format_user_prompt(original_code, update_snippet);
        assert_eq!(
            result,
            "<code>fn main() {}</code>\n<update>fn main() { println!(\"Hello\"); }</update>"
        );
    }

    #[test]
    fn test_format_user_prompt_nested_tags() {
        let original_code = "fn main() {}";
        let update_snippet = "<code>fn main() { <code>nested</code> }</code>";
        let result = MorphLLMEditor::format_user_prompt(original_code, update_snippet);
        // Should use the first occurrence of <code> and find its matching </code>
        assert_eq!(
            result,
            "<code>fn main() {}</code>\n<update>fn main() { <code>nested</update>"
        );
    }

    #[test]
    fn test_format_user_prompt_tags_in_different_order() {
        let original_code = "fn main() {}";
        let update_snippet = "<instruction>I am adding a print statement</instruction><code>fn main() { println!(\"Hello\"); }</code>";
        let result = MorphLLMEditor::format_user_prompt(original_code, update_snippet);
        assert_eq!(
            result,
            "<instruction>I am adding a print statement</instruction>\n<code>fn main() {}</code>\n<update>fn main() { println!(\"Hello\"); }</update>"
        );
    }
}
