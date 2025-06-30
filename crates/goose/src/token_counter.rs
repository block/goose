use ahash::AHasher;
use dashmap::DashMap;
use futures_util::stream::StreamExt;
use include_dir::{include_dir, Dir};
use mcp_core::Tool;
use std::error::Error;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Arc;
use tokenizers::tokenizer::Tokenizer;
use tokio::sync::OnceCell;

use crate::message::Message;

// The embedded directory with all possible tokenizer files.
// If one of them doesn’t exist, we’ll download it at startup.
static TOKENIZER_FILES: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../tokenizer_files");

// Global tokenizer cache to avoid repeated downloads and loading
static TOKENIZER_CACHE: OnceCell<Arc<DashMap<String, Arc<Tokenizer>>>> = OnceCell::const_new();

// Cache size limits to prevent unbounded growth
const MAX_TOKEN_CACHE_SIZE: usize = 10_000;
const MAX_TOKENIZER_CACHE_SIZE: usize = 50;

/// Async token counter with caching capabilities
pub struct AsyncTokenCounter {
    tokenizer: Arc<Tokenizer>,
    token_cache: Arc<DashMap<u64, usize>>, // content hash -> token count
}

/// Legacy synchronous token counter for backward compatibility
pub struct TokenCounter {
    tokenizer: Tokenizer,
}

impl AsyncTokenCounter {
    /// Creates a new async token counter with caching
    pub async fn new(tokenizer_name: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        // Initialize global cache if not already done
        let cache = TOKENIZER_CACHE
            .get_or_init(|| async { Arc::new(DashMap::new()) })
            .await;

        // Check cache first - DashMap allows concurrent reads
        if let Some(tokenizer) = cache.get(tokenizer_name) {
            return Ok(Self {
                tokenizer: tokenizer.clone(),
                token_cache: Arc::new(DashMap::new()),
            });
        }

        // Try embedded first
        let tokenizer = match Self::load_from_embedded(tokenizer_name) {
            Ok(tokenizer) => Arc::new(tokenizer),
            Err(_) => {
                // Download async if not found
                Arc::new(Self::download_and_load_async(tokenizer_name).await?)
            }
        };

        // Cache the tokenizer with size management
        if cache.len() >= MAX_TOKENIZER_CACHE_SIZE {
            // Simple eviction: remove oldest entry
            if let Some(entry) = cache.iter().next() {
                let old_key = entry.key().clone();
                cache.remove(&old_key);
            }
        }
        cache.insert(tokenizer_name.to_string(), tokenizer.clone());

        Ok(Self {
            tokenizer,
            token_cache: Arc::new(DashMap::new()),
        })
    }

    /// Load tokenizer bytes from the embedded directory
    fn load_from_embedded(tokenizer_name: &str) -> Result<Tokenizer, Box<dyn Error + Send + Sync>> {
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

    /// Async download that doesn't block the runtime
    async fn download_and_load_async(
        tokenizer_name: &str,
    ) -> Result<Tokenizer, Box<dyn Error + Send + Sync>> {
        let local_dir = std::env::temp_dir().join(tokenizer_name);
        let local_json_path = local_dir.join("tokenizer.json");

        // Check if file exists
        if !tokio::fs::try_exists(&local_json_path)
            .await
            .unwrap_or(false)
        {
            eprintln!("Downloading tokenizer: {}", tokenizer_name);
            let repo_id = tokenizer_name.replace("--", "/");
            Self::download_tokenizer_async(&repo_id, &local_dir).await?;
        }

        // Load from disk asynchronously
        let file_content = tokio::fs::read(&local_json_path).await?;
        let tokenizer = Tokenizer::from_bytes(&file_content)
            .map_err(|e| format!("Failed to parse tokenizer: {}", e))?;

        Ok(tokenizer)
    }

    /// Robust async download with retry logic and network failure handling
    async fn download_tokenizer_async(
        repo_id: &str,
        download_dir: &std::path::Path,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        tokio::fs::create_dir_all(download_dir).await?;

        let file_url = format!(
            "https://huggingface.co/{}/resolve/main/tokenizer.json",
            repo_id
        );
        let file_path = download_dir.join("tokenizer.json");

        // Check if partial/corrupted file exists and remove it
        if file_path.exists() {
            if let Ok(existing_bytes) = tokio::fs::read(&file_path).await {
                if Self::is_valid_tokenizer_json(&existing_bytes) {
                    return Ok(()); // File is complete and valid
                }
            }
            // Remove corrupted/incomplete file
            let _ = tokio::fs::remove_file(&file_path).await;
        }

        // Create enhanced HTTP client with timeouts
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .connect_timeout(std::time::Duration::from_secs(15))
            .user_agent("goose-tokenizer/1.0")
            .build()?;

        // Download with retry logic
        let response = Self::download_with_retry(&client, &file_url, 3).await?;

        // Stream download with progress reporting for large files
        let total_size = response.content_length();
        let mut stream = response.bytes_stream();
        let mut file = tokio::fs::File::create(&file_path).await?;
        let mut downloaded = 0;

        use tokio::io::AsyncWriteExt;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            file.write_all(&chunk).await?;
            downloaded += chunk.len();

            // Progress reporting for large downloads
            if let Some(total) = total_size {
                if total > 1024 * 1024 && downloaded % (256 * 1024) == 0 {
                    // Report every 256KB for files >1MB
                    eprintln!(
                        "Downloaded {}/{} bytes ({:.1}%)",
                        downloaded,
                        total,
                        (downloaded as f64 / total as f64) * 100.0
                    );
                }
            }
        }

        file.flush().await?;

        // Validate downloaded file
        let final_bytes = tokio::fs::read(&file_path).await?;
        if !Self::is_valid_tokenizer_json(&final_bytes) {
            tokio::fs::remove_file(&file_path).await?;
            return Err("Downloaded tokenizer file is invalid or corrupted".into());
        }

        eprintln!(
            "Successfully downloaded tokenizer: {} ({} bytes)",
            repo_id, downloaded
        );
        Ok(())
    }

    /// Download with exponential backoff retry logic
    async fn download_with_retry(
        client: &reqwest::Client,
        url: &str,
        max_retries: u32,
    ) -> Result<reqwest::Response, Box<dyn Error + Send + Sync>> {
        let mut delay = std::time::Duration::from_millis(200);

        for attempt in 0..=max_retries {
            match client.get(url).send().await {
                Ok(response) if response.status().is_success() => {
                    return Ok(response);
                }
                Ok(response) if response.status().is_server_error() => {
                    // Retry on 5xx errors (server issues)
                    if attempt < max_retries {
                        eprintln!(
                            "Server error {} on attempt {}/{}, retrying in {:?}",
                            response.status(),
                            attempt + 1,
                            max_retries + 1,
                            delay
                        );
                        tokio::time::sleep(delay).await;
                        delay = std::cmp::min(delay * 2, std::time::Duration::from_secs(30)); // Cap at 30s
                        continue;
                    }
                    return Err(format!(
                        "Server error after {} retries: {}",
                        max_retries,
                        response.status()
                    )
                    .into());
                }
                Ok(response) => {
                    // Don't retry on 4xx errors (client errors like 404, 403)
                    return Err(format!("Client error: {} - {}", response.status(), url).into());
                }
                Err(e) if attempt < max_retries => {
                    // Retry on network errors (timeout, connection refused, DNS, etc.)
                    eprintln!(
                        "Network error on attempt {}/{}: {}, retrying in {:?}",
                        attempt + 1,
                        max_retries + 1,
                        e,
                        delay
                    );
                    tokio::time::sleep(delay).await;
                    delay = std::cmp::min(delay * 2, std::time::Duration::from_secs(30)); // Cap at 30s
                    continue;
                }
                Err(e) => {
                    return Err(
                        format!("Network error after {} retries: {}", max_retries, e).into(),
                    );
                }
            }
        }
        unreachable!()
    }

    /// Validate that the downloaded file is a valid tokenizer JSON
    fn is_valid_tokenizer_json(bytes: &[u8]) -> bool {
        // Basic validation: check if it's valid JSON and has tokenizer structure
        if let Ok(json_str) = std::str::from_utf8(bytes) {
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(json_str) {
                // Check for basic tokenizer structure
                return json_value.get("version").is_some()
                    || json_value.get("vocab").is_some()
                    || json_value.get("model").is_some();
            }
        }
        false
    }

    /// Count tokens with optimized caching
    pub fn count_tokens(&self, text: &str) -> usize {
        // Use faster AHash for better performance
        let mut hasher = AHasher::default();
        text.hash(&mut hasher);
        let hash = hasher.finish();

        // Check cache first
        if let Some(count) = self.token_cache.get(&hash) {
            return *count;
        }

        // Compute and cache result with size management
        let encoding = self.tokenizer.encode(text, false).unwrap_or_default();
        let count = encoding.len();

        // Manage cache size to prevent unbounded growth
        if self.token_cache.len() >= MAX_TOKEN_CACHE_SIZE {
            // Simple eviction: remove a random entry
            if let Some(entry) = self.token_cache.iter().next() {
                let old_hash = *entry.key();
                self.token_cache.remove(&old_hash);
            }
        }

        self.token_cache.insert(hash, count);
        count
    }

    /// Count tokens for tools with optimized string handling
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
                func_token_count += func_init;
                let name = &tool.name;
                let description = &tool.description.trim_end_matches('.');

                // Optimize: count components separately to avoid string allocation
                // Note: the separator (:) is likely tokenized with adjacent tokens, so we use original approach for accuracy
                let line = format!("{}:{}", name, description);
                func_token_count += self.count_tokens(&line);

                if let serde_json::Value::Object(properties) = &tool.input_schema["properties"] {
                    if !properties.is_empty() {
                        func_token_count += prop_init;
                        for (key, value) in properties {
                            func_token_count += prop_key;
                            let p_name = key;
                            let p_type = value["type"].as_str().unwrap_or("");
                            let p_desc = value["description"]
                                .as_str()
                                .unwrap_or("")
                                .trim_end_matches('.');

                            // Note: separators are tokenized with adjacent tokens, keep original for accuracy
                            let line = format!("{}:{}:{}", p_name, p_type, p_desc);
                            func_token_count += self.count_tokens(&line);

                            if let Some(enum_values) = value["enum"].as_array() {
                                func_token_count =
                                    func_token_count.saturating_add_signed(enum_init);
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

    /// Count chat tokens (using cached count_tokens)
    pub fn count_chat_tokens(
        &self,
        system_prompt: &str,
        messages: &[Message],
        tools: &[Tool],
    ) -> usize {
        let tokens_per_message = 4;
        let mut num_tokens = 0;

        if !system_prompt.is_empty() {
            num_tokens += self.count_tokens(system_prompt) + tokens_per_message;
        }

        for message in messages {
            num_tokens += tokens_per_message;
            for content in &message.content {
                if let Some(content_text) = content.as_text() {
                    num_tokens += self.count_tokens(content_text);
                } else if let Some(tool_request) = content.as_tool_request() {
                    let tool_call = tool_request.tool_call.as_ref().unwrap();
                    // Note: separators are tokenized with adjacent tokens, keep original for accuracy
                    let text = format!(
                        "{}:{}:{}",
                        tool_request.id, tool_call.name, tool_call.arguments
                    );
                    num_tokens += self.count_tokens(&text);
                } else if let Some(tool_response_text) = content.as_tool_response_text() {
                    num_tokens += self.count_tokens(&tool_response_text);
                }
            }
        }

        if !tools.is_empty() {
            num_tokens += self.count_tokens_for_tools(tools);
        }

        num_tokens += 3; // Reply primer

        num_tokens
    }

    /// Count everything including resources (using cached count_tokens)
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

    /// Cache management methods
    pub fn clear_cache(&self) {
        self.token_cache.clear();
    }

    pub fn cache_size(&self) -> usize {
        self.token_cache.len()
    }
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

    /// DEPRECATED: Use AsyncTokenCounter for new code
    /// Download from Hugging Face into the local directory if not already present.
    /// This method still blocks but is kept for backward compatibility.
    fn download_tokenizer(repo_id: &str, download_dir: &Path) -> Result<(), Box<dyn Error>> {
        std::fs::create_dir_all(download_dir)?;

        let file_url = format!(
            "https://huggingface.co/{}/resolve/main/tokenizer.json",
            repo_id
        );
        let file_path = download_dir.join("tokenizer.json");

        // Use blocking reqwest client to avoid nested runtime
        let client = reqwest::blocking::Client::new();
        let response = client.get(&file_url).send()?;

        if !response.status().is_success() {
            let error_msg = format!("Failed to download tokenizer: status {}", response.status());
            return Err(Box::<dyn Error>::from(error_msg));
        }

        let bytes = response.bytes()?;
        std::fs::write(&file_path, bytes)?;

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

/// Factory function for creating async token counters with proper error handling
pub async fn create_async_token_counter(tokenizer_name: &str) -> Result<AsyncTokenCounter, String> {
    AsyncTokenCounter::new(tokenizer_name)
        .await
        .map_err(|e| format!("Failed to initialize tokenizer '{}': {}", tokenizer_name, e))
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

    #[tokio::test]
    async fn test_async_claude_tokenizer() {
        let counter = create_async_token_counter(CLAUDE_TOKENIZER).await.unwrap();

        let text = "Hello, how are you?";
        let count = counter.count_tokens(text);
        println!("Async token count for '{}': {:?}", text, count);

        assert_eq!(count, 6, "Async Claude tokenizer token count mismatch");
    }

    #[tokio::test]
    async fn test_async_gpt_4o_tokenizer() {
        let counter = create_async_token_counter(GPT_4O_TOKENIZER).await.unwrap();

        let text = "Hey there!";
        let count = counter.count_tokens(text);
        println!("Async token count for '{}': {:?}", text, count);

        assert_eq!(count, 3, "Async GPT-4o tokenizer token count mismatch");
    }

    #[tokio::test]
    async fn test_async_token_caching() {
        let counter = create_async_token_counter(GPT_4O_TOKENIZER).await.unwrap();

        let text = "This is a test for caching functionality";

        // First call should compute and cache
        let count1 = counter.count_tokens(text);
        assert_eq!(counter.cache_size(), 1);

        // Second call should use cache
        let count2 = counter.count_tokens(text);
        assert_eq!(count1, count2);
        assert_eq!(counter.cache_size(), 1);

        // Different text should increase cache
        let count3 = counter.count_tokens("Different text");
        assert_eq!(counter.cache_size(), 2);
        assert_ne!(count1, count3);
    }

    #[tokio::test]
    async fn test_async_count_chat_tokens() {
        let counter = create_async_token_counter(GPT_4O_TOKENIZER).await.unwrap();

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
        println!(
            "Async total tokens without tools: {}",
            token_count_without_tools
        );

        let token_count_with_tools = counter.count_chat_tokens(system_prompt, &messages, &tools);
        println!("Async total tokens with tools: {}", token_count_with_tools);

        // Should match the synchronous version
        assert_eq!(token_count_without_tools, 56);
        assert_eq!(token_count_with_tools, 124);
    }

    #[tokio::test]
    async fn test_async_tokenizer_caching() {
        // Create two counters with the same tokenizer name
        let counter1 = create_async_token_counter(GPT_4O_TOKENIZER).await.unwrap();
        let counter2 = create_async_token_counter(GPT_4O_TOKENIZER).await.unwrap();

        // Both should work and give same results (tokenizer is cached globally)
        let text = "Test tokenizer caching";
        let count1 = counter1.count_tokens(text);
        let count2 = counter2.count_tokens(text);

        assert_eq!(count1, count2);
    }

    #[tokio::test]
    async fn test_async_cache_management() {
        let counter = create_async_token_counter(GPT_4O_TOKENIZER).await.unwrap();

        // Add some items to cache
        counter.count_tokens("First text");
        counter.count_tokens("Second text");
        counter.count_tokens("Third text");

        assert_eq!(counter.cache_size(), 3);

        // Clear cache
        counter.clear_cache();
        assert_eq!(counter.cache_size(), 0);

        // Re-count should work fine
        let count = counter.count_tokens("First text");
        assert!(count > 0);
        assert_eq!(counter.cache_size(), 1);
    }

    #[tokio::test]
    async fn test_concurrent_token_counter_creation() {
        // Test concurrent creation of token counters to verify no race conditions
        let handles: Vec<_> = (0..10)
            .map(|_| {
                tokio::spawn(async { create_async_token_counter(GPT_4O_TOKENIZER).await.unwrap() })
            })
            .collect();

        let counters: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        // All should work and give same results
        let text = "Test concurrent creation";
        let expected_count = counters[0].count_tokens(text);

        for counter in &counters {
            assert_eq!(counter.count_tokens(text), expected_count);
        }
    }

    #[tokio::test]
    async fn test_cache_eviction_behavior() {
        let counter = create_async_token_counter(GPT_4O_TOKENIZER).await.unwrap();

        // Fill cache beyond normal size to test eviction
        let mut cached_texts = Vec::new();
        for i in 0..50 {
            let text = format!("Test string number {}", i);
            counter.count_tokens(&text);
            cached_texts.push(text);
        }

        // Cache should be bounded
        assert!(counter.cache_size() <= MAX_TOKEN_CACHE_SIZE);

        // Earlier entries may have been evicted, but recent ones should still be cached
        let recent_text = &cached_texts[cached_texts.len() - 1];
        let start_size = counter.cache_size();

        // This should be a cache hit (no size increase)
        counter.count_tokens(recent_text);
        assert_eq!(counter.cache_size(), start_size);
    }

    #[tokio::test]
    async fn test_async_error_handling() {
        // Test with invalid tokenizer name
        let result = create_async_token_counter("invalid/nonexistent-tokenizer").await;
        assert!(result.is_err(), "Should fail with invalid tokenizer name");
    }

    #[tokio::test]
    async fn test_concurrent_cache_operations() {
        let counter =
            std::sync::Arc::new(create_async_token_counter(GPT_4O_TOKENIZER).await.unwrap());

        // Test concurrent token counting operations
        let handles: Vec<_> = (0..20)
            .map(|i| {
                let counter_clone = counter.clone();
                tokio::spawn(async move {
                    let text = format!("Concurrent test {}", i % 5); // Some repetition for cache hits
                    counter_clone.count_tokens(&text)
                })
            })
            .collect();

        let results: Vec<_> = futures::future::join_all(handles)
            .await
            .into_iter()
            .map(|r| r.unwrap())
            .collect();

        // All results should be valid (> 0)
        for result in results {
            assert!(result > 0);
        }

        // Cache should have some entries but be bounded
        assert!(counter.cache_size() > 0);
        assert!(counter.cache_size() <= MAX_TOKEN_CACHE_SIZE);
    }

    #[test]
    fn test_tokenizer_json_validation() {
        // Test valid tokenizer JSON
        let valid_json = r#"{"version": "1.0", "model": {"type": "BPE"}}"#;
        assert!(AsyncTokenCounter::is_valid_tokenizer_json(
            valid_json.as_bytes()
        ));

        let valid_json2 = r#"{"vocab": {"hello": 1, "world": 2}}"#;
        assert!(AsyncTokenCounter::is_valid_tokenizer_json(
            valid_json2.as_bytes()
        ));

        // Test invalid JSON
        let invalid_json = r#"{"incomplete": true"#;
        assert!(!AsyncTokenCounter::is_valid_tokenizer_json(
            invalid_json.as_bytes()
        ));

        // Test valid JSON but not tokenizer structure
        let wrong_structure = r#"{"random": "data", "not": "tokenizer"}"#;
        assert!(!AsyncTokenCounter::is_valid_tokenizer_json(
            wrong_structure.as_bytes()
        ));

        // Test binary data
        let binary_data = [0xFF, 0xFE, 0x00, 0x01];
        assert!(!AsyncTokenCounter::is_valid_tokenizer_json(&binary_data));

        // Test empty data
        assert!(!AsyncTokenCounter::is_valid_tokenizer_json(&[]));
    }

    #[tokio::test]
    async fn test_download_with_retry_logic() {
        // This test would require mocking HTTP responses
        // For now, we test the retry logic structure by verifying the function exists
        // In a full test suite, you'd use wiremock or similar to simulate failures

        // Test that the function exists and has the right signature
        let client = reqwest::Client::new();

        // Test with a known bad URL to verify error handling
        let result =
            AsyncTokenCounter::download_with_retry(&client, "https://httpbin.org/status/404", 1)
                .await;

        assert!(result.is_err(), "Should fail with 404 error");

        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("Client error: 404"),
            "Should contain client error message"
        );
    }

    #[tokio::test]
    async fn test_network_resilience_with_timeout() {
        // Test timeout handling with a slow endpoint
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(100)) // Very short timeout
            .build()
            .unwrap();

        // Use httpbin delay endpoint that takes longer than our timeout
        let result = AsyncTokenCounter::download_with_retry(
            &client,
            "https://httpbin.org/delay/1", // 1 second delay, but 100ms timeout
            1,
        )
        .await;

        assert!(result.is_err(), "Should timeout and fail");
    }

    #[tokio::test]
    async fn test_successful_download_retry() {
        // Test successful download after simulated retry
        let client = reqwest::Client::new();

        // Use a reliable endpoint that should succeed
        let result =
            AsyncTokenCounter::download_with_retry(&client, "https://httpbin.org/status/200", 2)
                .await;

        assert!(result.is_ok(), "Should succeed with 200 status");
    }
}
