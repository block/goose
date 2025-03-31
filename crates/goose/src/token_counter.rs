use include_dir::{include_dir, Dir};
use mcp_core::Tool;
use std::error::Error;
use std::fs;
use std::path::Path;
use tokenizers::tokenizer::Tokenizer;

use crate::message::Message;

// The embedded directory with all possible tokenizer files.
// If one of them doesn’t exist, we’ll download it at startup.
static TOKENIZER_FILES: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../tokenizer_files");

/// The `TokenCounter` now stores exactly one `Tokenizer`.
pub struct TokenCounter {
    tokenizer: Tokenizer,
}

impl TokenCounter {
    /// Creates a new `TokenCounter` using the given HuggingFace tokenizer name.
    ///
    /// * `tokenizer_name` might look like "Xenova--gpt-4o"
    ///   or "Qwen--Qwen2.5-Coder-32B-Instruct", etc.
    pub fn new(tokenizer_name: &str) -> Self {
        match Self::load_from_embedded(tokenizer_name) {
            Ok(tokenizer) => Self { tokenizer },
            Err(e) => {
                println!(
                    "Tokenizer '{}' not found in embedded dir: {}",
                    tokenizer_name, e
                );
                println!("Attempting to download tokenizer and load...");
                // Fallback to download tokenizer and load from disk
                match Self::download_and_load(tokenizer_name) {
                    Ok(counter) => counter,
                    Err(e) => panic!("Failed to initialize tokenizer: {}", e),
                }
            }
        }
    }

    /// Load tokenizer bytes from the embedded directory (via `include_dir!`).
    fn load_from_embedded(tokenizer_name: &str) -> Result<Tokenizer, Box<dyn Error>> {
        let tokenizer_file_path = format!("{}/tokenizer.json", tokenizer_name);
        let file = TOKENIZER_FILES
            .get_file(&tokenizer_file_path)
            .ok_or_else(|| {
                format!(
                    "Tokenizer file not found in embedded: {}",
                    tokenizer_file_path
                )
            })?;
        let contents = file.contents();
        let tokenizer = Tokenizer::from_bytes(contents)
            .map_err(|e| format!("Failed to parse tokenizer bytes: {}", e))?;
        Ok(tokenizer)
    }

    /// Fallback: If not found in embedded, we look in `base_dir` on disk.
    /// If not on disk, we download from Hugging Face, then load from disk.
    fn download_and_load(tokenizer_name: &str) -> Result<Self, Box<dyn Error>> {
        let local_dir = std::env::temp_dir().join(tokenizer_name);
        let local_json_path = local_dir.join("tokenizer.json");

        // If the file doesn't already exist, we download from HF
        if !Path::new(&local_json_path).exists() {
            eprintln!("Tokenizer file not on disk, downloading…");
            let repo_id = tokenizer_name.replace("--", "/");
            // e.g. "Xenova--llama3-tokenizer" -> "Xenova/llama3-tokenizer"
            Self::download_tokenizer(&repo_id, &local_dir)?;
        }

        // Load from disk
        let file_content = fs::read(&local_json_path)?;
        let tokenizer = Tokenizer::from_bytes(&file_content)
            .map_err(|e| format!("Failed to parse tokenizer after download: {}", e))?;

        Ok(Self { tokenizer })
    }

    /// Download from Hugging Face into the local directory if not already present.
    /// Synchronous version using a blocking runtime for simplicity.
    fn download_tokenizer(repo_id: &str, download_dir: &Path) -> Result<(), Box<dyn Error>> {
        fs::create_dir_all(download_dir)?;

        let file_url = format!(
            "https://huggingface.co/{}/resolve/main/tokenizer.json",
            repo_id
        );
        let file_path = download_dir.join("tokenizer.json");

        // Blocking for example: just spawn a short-lived runtime
        let content = tokio::runtime::Runtime::new()?.block_on(async {
            let response = reqwest::get(&file_url).await?;
            if !response.status().is_success() {
                let error_msg =
                    format!("Failed to download tokenizer: status {}", response.status());
                return Err(Box::<dyn Error>::from(error_msg));
            }
            let bytes = response.bytes().await?;
            Ok(bytes)
        })?;

        fs::write(&file_path, content)?;

        Ok(())
    }

    /// Count tokens for a piece of text using our single tokenizer.
    pub fn count_tokens(&self, text: &str) -> usize {
        let encoding = self.tokenizer.encode(text, false).unwrap();
        encoding.len()
    }

    pub fn count_tokens_for_tools(&self, tools: &[Tool]) -> usize {
        // Token counts for different function components
        let func_init = 7; // Tokens for function initialization
        let prop_init = 3; // Tokens for properties initialization
        let prop_key = 3; // Tokens for each property key
        let enum_init: isize = -3; // Tokens adjustment for enum list start
        let enum_item = 3; // Tokens for each enum item
        let func_end = 12; // Tokens for function ending

        let mut func_token_count = 0;
        if !tools.is_empty() {
            for tool in tools {
                func_token_count += func_init; // Add tokens for start of each function
                let name = &tool.name;
                let description = &tool.description.trim_end_matches('.');
                let line = format!("{}:{}", name, description);
                func_token_count += self.count_tokens(&line); // Add tokens for name and description

                if let serde_json::Value::Object(properties) = &tool.input_schema["properties"] {
                    if !properties.is_empty() {
                        func_token_count += prop_init; // Add tokens for start of properties
                        for (key, value) in properties {
                            func_token_count += prop_key; // Add tokens for each property
                            let p_name = key;
                            let p_type = value["type"].as_str().unwrap_or("");
                            let p_desc = value["description"]
                                .as_str()
                                .unwrap_or("")
                                .trim_end_matches('.');
                            let line = format!("{}:{}:{}", p_name, p_type, p_desc);
                            func_token_count += self.count_tokens(&line);
                            if let Some(enum_values) = value["enum"].as_array() {
                                func_token_count =
                                    func_token_count.saturating_add_signed(enum_init); // Add tokens if property has enum list
                                for item in enum_values {
                                    if let Some(item_str) = item.as_str() {
                                        func_token_count += enum_item;
                                        func_token_count += self.count_tokens(item_str);
                                    }
                                }
                            }
                        }
                    }
                }
            }
            func_token_count += func_end;
        }

        func_token_count
    }

    pub fn count_chat_tokens(
        &self,
        system_prompt: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> usize {
        // <|im_start|>ROLE<|im_sep|>MESSAGE<|im_end|>
        let tokens_per_message = 4;

        // Count tokens in the system prompt
        let mut num_tokens = 0;
        if !system_prompt.is_empty() {
            num_tokens += self.count_tokens(system_prompt) + tokens_per_message;
        }

        for message in messages {
            num_tokens += tokens_per_message;
            // Count tokens in the content
            for content in &message.content {
                // content can either be text response or tool request
                if let Some(content_text) = content.as_text() {
                    num_tokens += self.count_tokens(content_text);
                } else if let Some(tool_request) = content.as_tool_request() {
                    // TODO: count tokens for tool request
                    let tool_call = tool_request.tool_call.as_ref().unwrap();
                    let text = format!(
                        "{}:{}:{}",
                        tool_request.id, tool_call.name, tool_call.arguments
                    );
                    num_tokens += self.count_tokens(&text);
                } else if let Some(tool_response_text) = content.as_tool_response_text() {
                    num_tokens += self.count_tokens(&tool_response_text);
                } else {
                    // unsupported content type such as image - pass
                    continue;
                }
            }
        }

        // Count tokens for tools if provided
        if !tools.is_empty() {
            num_tokens += self.count_tokens_for_tools(tools);
        }

        // Every reply is primed with <|start|>assistant<|message|>
        num_tokens += 3;

        num_tokens
    }

    pub fn count_everything(
        &self,
        system_prompt: &str,
        messages: &[Message],
        tools: &[Tool],
        resources: &[String],
    ) -> usize {
        let mut num_tokens = self.count_chat_tokens(system_prompt, messages, tools);

        if !resources.is_empty() {
            for resource in resources {
                num_tokens += self.count_tokens(resource);
            }
        }
        num_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{Message, MessageContent}; // or however your `Message` is imported
    use crate::model::{CLAUDE_TOKENIZER, GPT_4O_TOKENIZER};
    use mcp_core::role::Role;
    use mcp_core::tool::Tool;
    use serde_json::json;

    #[test]
    fn test_claude_tokenizer() {
        let counter = TokenCounter::new(CLAUDE_TOKENIZER);

        let text = "Hello, how are you?";
        let count = counter.count_tokens(text);
        println!("Token count for '{}': {:?}", text, count);

        // The old test expected 6 tokens
        assert_eq!(count, 6, "Claude tokenizer token count mismatch");
    }

    #[test]
    fn test_gpt_4o_tokenizer() {
        let counter = TokenCounter::new(GPT_4O_TOKENIZER);

        let text = "Hey there!";
        let count = counter.count_tokens(text);
        println!("Token count for '{}': {:?}", text, count);

        // The old test expected 3 tokens
        assert_eq!(count, 3, "GPT-4o tokenizer token count mismatch");
    }

    #[test]
    fn test_count_chat_tokens() {
        let counter = TokenCounter::new(GPT_4O_TOKENIZER);

        let system_prompt =
            "You are a helpful assistant that can answer questions about the weather.";

        let messages = vec![
            Message {
                role: Role::User,
                created: 0,
                content: vec![MessageContent::text(
                    "What's the weather like in San Francisco?",
                )],
            },
            Message {
                role: Role::Assistant,
                created: 1,
                content: vec![MessageContent::text(
                    "Looks like it's 60 degrees Fahrenheit in San Francisco.",
                )],
            },
            Message {
                role: Role::User,
                created: 2,
                content: vec![MessageContent::text("How about New York?")],
            },
        ];

        let tools = vec![Tool {
            name: "get_current_weather".to_string(),
            description: "Get the current weather in a given location".to_string(),
            input_schema: json!({
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "The city and state, e.g. San Francisco, CA"
                    },
                    "unit": {
                        "type": "string",
                        "description": "The unit of temperature to return",
                        "enum": ["celsius", "fahrenheit"]
                    }
                },
                "required": ["location"]
            }),
            annotations: None,
        }];

        let token_count_without_tools = counter.count_chat_tokens(system_prompt, &messages, &[]);
        println!("Total tokens without tools: {}", token_count_without_tools);

        let token_count_with_tools = counter.count_chat_tokens(system_prompt, &messages, &tools);
        println!("Total tokens with tools: {}", token_count_with_tools);

        // The old test used 56 / 124 for GPT-4o. Adjust if your actual tokenizer changes
        assert_eq!(token_count_without_tools, 56);
        assert_eq!(token_count_with_tools, 124);
    }

    #[test]
    #[should_panic]
    fn test_panic_if_provided_tokenizer_doesnt_exist() {
        // This should panic because the tokenizer doesn't exist
        // in the embedded directory and the download fails

        TokenCounter::new("nonexistent-tokenizer");
    }

    // Optional test to confirm that fallback download works if not found in embedded:
    // Ignored cause this actually downloads a tokenizer from Hugging Face
    #[test]
    #[ignore]
    fn test_download_tokenizer_successfully_if_not_embedded() {
        let non_embedded_key = "openai-community/gpt2";
        let counter = TokenCounter::new(non_embedded_key);

        // If it downloads successfully, we can do a quick count to ensure it's valid
        let text = "print('hello world')";
        let count = counter.count_tokens(text);
        println!(
            "Downloaded tokenizer, token count for '{}': {}",
            text, count
        );

        // https://tiktokenizer.vercel.app/?model=gpt2
        assert!(count == 5, "Expected 5 tokens from downloaded tokenizer");
    }
}
