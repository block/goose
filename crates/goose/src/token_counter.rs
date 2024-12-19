use crate::message::Message;
use include_dir::{include_dir, Dir};
use mcp_core::tool::Tool;
use std::collections::HashMap;
use tokenizers::tokenizer::Tokenizer;

// Embed the tokenizer files directory
static TOKENIZER_FILES: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../tokenizer_files");

pub struct TokenCounter {
    tokenizers: HashMap<String, Tokenizer>,
}

const GPT_4O_TOKENIZER_KEY: &str = "Xenova--gpt-4o";
const CLAUDE_TOKENIZER_KEY: &str = "Xenova--claude-tokenizer";
const GOOGLE_TOKENIZER_KEY: &str = "Xenova--gemma-2-tokenizer";
const QWEN_TOKENIZER_KEY: &str = "Qwen--Qwen2.5-Coder-32B-Instruct";

impl Default for TokenCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenCounter {
    fn load_tokenizer(&mut self, tokenizer_key: &str) {
        // Load from embedded tokenizer files. The tokenizer_key must match the directory name.
        let tokenizer_path = format!("{}/tokenizer.json", tokenizer_key);
        let file_content = TOKENIZER_FILES
            .get_file(&tokenizer_path)
            .map(|f| f.contents())
            .ok_or_else(|| format!("Embedded tokenizer file not found: {}", tokenizer_path))
            .unwrap();

        let tokenizer = Tokenizer::from_bytes(file_content);

        match tokenizer {
            Ok(tokenizer) => {
                self.tokenizers.insert(tokenizer_key.to_string(), tokenizer);
            }
            Err(e) => {
                eprintln!("Failed to load tokenizer {}: {}", tokenizer_key, e);
            }
        }
    }

    pub fn new() -> Self {
        let mut counter = TokenCounter {
            tokenizers: HashMap::new(),
        };
        // Add default tokenizers
        for tokenizer_key in [
            GPT_4O_TOKENIZER_KEY,
            CLAUDE_TOKENIZER_KEY,
            GOOGLE_TOKENIZER_KEY,
        ] {
            counter.load_tokenizer(tokenizer_key);
        }
        counter
    }

    pub fn add_tokenizer(&mut self, tokenizer_key: &str) {
        self.load_tokenizer(tokenizer_key);
    }

    fn model_to_tokenizer_key(model_name: Option<&str>) -> &str {
        let model_name = model_name.unwrap_or("gpt-4o").to_lowercase();
        if model_name.contains("claude") {
            CLAUDE_TOKENIZER_KEY
        } else if model_name.contains("qwen") {
            QWEN_TOKENIZER_KEY
        } else if model_name.contains("gemini") {
            GOOGLE_TOKENIZER_KEY
        } else {
            // default
            GPT_4O_TOKENIZER_KEY
        }
    }

    fn get_tokenizer(&self, model_name: Option<&str>) -> &Tokenizer {
        let tokenizer_key = Self::model_to_tokenizer_key(model_name);
        self.tokenizers
            .get(tokenizer_key)
            .expect("Tokenizer not found")
    }

    pub fn count_tokens(&self, text: &str, model_name: Option<&str>) -> usize {
        let tokenizer = self.get_tokenizer(model_name);
        let encoding = tokenizer.encode(text, false).unwrap();
        encoding.len()
    }

    fn count_tokens_for_tools(&self, tools: &[Tool], model_name: Option<&str>) -> usize {
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
                func_token_count += self.count_tokens(&line, model_name); // Add tokens for name and description

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
                            func_token_count += self.count_tokens(&line, model_name);
                            if let Some(enum_values) = value["enum"].as_array() {
                                func_token_count =
                                    func_token_count.saturating_add_signed(enum_init); // Add tokens if property has enum list
                                for item in enum_values {
                                    if let Some(item_str) = item.as_str() {
                                        func_token_count += enum_item;
                                        func_token_count += self.count_tokens(item_str, model_name);
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
        model_name: Option<&str>,
    ) -> usize {
        // <|im_start|>ROLE<|im_sep|>MESSAGE<|im_end|>
        let tokens_per_message = 4;

        // Count tokens in the system prompt
        let mut num_tokens = 0;
        if !system_prompt.is_empty() {
            num_tokens += self.count_tokens(system_prompt, model_name) + tokens_per_message;
        }

        for message in messages {
            num_tokens += tokens_per_message;
            // Count tokens in the content
            for content in &message.content {
                // content can either be text response or tool request
                if let Some(content_text) = content.as_text() {
                    num_tokens += self.count_tokens(content_text, model_name);
                } else if let Some(tool_request) = content.as_tool_request() {
                    // TODO: count tokens for tool request
                    let tool_call = tool_request.tool_call.as_ref().unwrap();
                    let text = format!(
                        "{}:{}:{}",
                        tool_request.id, tool_call.name, tool_call.arguments
                    );
                    num_tokens += self.count_tokens(&text, model_name);
                } else if let Some(tool_response_text) = content.as_tool_response_text() {
                    num_tokens += self.count_tokens(&tool_response_text, model_name);
                } else {
                    // unsupported content type such as image - pass
                    continue;
                }
            }
        }

        // Count tokens for tools if provided
        if !tools.is_empty() {
            num_tokens += self.count_tokens_for_tools(tools, model_name);
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
        model_name: Option<&str>,
    ) -> usize {
        let mut num_tokens = self.count_chat_tokens(system_prompt, messages, tools, model_name);

        if !resources.is_empty() {
            for resource in resources {
                num_tokens += self.count_tokens(resource, model_name);
            }
        }
        num_tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::MessageContent;
    use mcp_core::role::Role;
    use serde_json::json;

    #[test]
    fn test_add_tokenizer_and_count_tokens() {
        let mut counter = TokenCounter::new();
        counter.add_tokenizer(QWEN_TOKENIZER_KEY);
        let text = "Hey there!";
        let count = counter.count_tokens(text, Some("qwen2.5-ollama"));
        println!("Token count for '{}': {:?}", text, count);
        assert_eq!(count, 3);
    }

    // Update the default tokenizer test similarly
    #[test]
    fn test_specific_claude_tokenizer() {
        let counter = TokenCounter::new();
        let text = "Hello, how are you?";
        let count = counter.count_tokens(text, Some("claude-3-5-sonnet-2"));
        println!("Token count for '{}': {:?}", text, count);
        assert_eq!(count, 6);
    }

    #[test]
    fn test_default_gpt_4o_tokenizer() {
        let counter = TokenCounter::new();
        let count = counter.count_tokens("Hey there!", None);
        assert_eq!(count, 3);
    }

    #[test]
    fn test_count_chat_tokens() {
        let token_counter = TokenCounter::new();

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
        }];

        let token_count_without_tools =
            token_counter.count_chat_tokens(system_prompt, &messages, &[], Some("gpt-4o"));
        println!("Total tokens without tools: {}", token_count_without_tools);

        let token_count_with_tools =
            token_counter.count_chat_tokens(system_prompt, &messages, &tools, Some("gpt-4o"));
        println!("Total tokens with tools: {}", token_count_with_tools);

        // The token count for messages without tools is calculated using the tokenizer - https://tiktokenizer.vercel.app/
        // The token count for messages with tools is taken from tiktoken github repo example (notebook)
        assert_eq!(token_count_without_tools, 56);
        assert_eq!(token_count_with_tools, 124);
    }
}
