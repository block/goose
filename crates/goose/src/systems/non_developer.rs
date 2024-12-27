use anyhow::Result as AnyhowResult;
use async_trait::async_trait;
use base64::Engine;
use indoc::{formatdoc, indoc};
use reqwest::{Client, Url};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Mutex;
use tokio::process::Command;

use crate::errors::{AgentError, AgentResult};
use crate::systems::System;
use mcp_core::{Content, Resource, Tool, ToolCall};

/// A system designed for non-developers to help them with common tasks like
/// web scraping, data processing, and automation.
pub struct NonDeveloperSystem {
    tools: Vec<Tool>,
    cache_dir: PathBuf,
    active_resources: Mutex<HashMap<String, Resource>>,
    http_client: Client,
    instructions: String,
}

impl Default for NonDeveloperSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl NonDeveloperSystem {
    pub fn new() -> Self {
        // Create tools for the system
        let web_scrape_tool = Tool::new(
            "web_scrape",
            indoc! {r#"
                Fetch and save content from a web page. The content can be saved as:
                - text (for HTML pages)
                - json (for API responses)
                - binary (for images and other files)
                
                The content is cached locally and can be accessed later using the cache_path
                returned in the response.
            "#},
            json!({
                "type": "object",
                "required": ["url"],
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to fetch content from"
                    },
                    "save_as": {
                        "type": "string",
                        "enum": ["text", "json", "binary"],
                        "default": "text",
                        "description": "How to interpret and save the content"
                    }
                }
            }),
        );

        let data_process_tool = Tool::new(
            "data_process",
            indoc! {r#"
                Process data from a file using common operations:
                - filter: Keep only lines matching a pattern
                - extract: Extract specific patterns from each line
                - sort: Sort lines alphabetically
                - unique: Remove duplicate lines
                - count: Count occurrences of patterns
                - split: Split file into smaller files
                - join: Join multiple files
                
                Results are saved to a new file in the cache directory.
            "#},
            json!({
                "type": "object",
                "required": ["input_path", "operation"],
                "properties": {
                    "input_path": {
                        "type": "string",
                        "description": "Path to the input file(s). For join operation, provide multiple paths separated by commas"
                    },
                    "operation": {
                        "type": "string",
                        "enum": ["filter", "extract", "sort", "unique", "count", "split", "join"],
                        "description": "The operation to perform on the data"
                    },
                    "pattern": {
                        "type": "string",
                        "description": "Pattern to use for filter/extract/count operations. Uses regular expressions"
                    },
                    "chunk_size": {
                        "type": "integer",
                        "description": "Number of lines per file for split operation",
                        "default": 1000
                    }
                }
            }),
        );

        let quick_script_tool = Tool::new(
            "quick_script",
            indoc! {r#"
                Create and run small scripts for automation tasks.
                Supports Shell, AppleScript, and Ruby (on macOS).
                
                The script is saved to a temporary file and executed.
                Consider using shell script (bash) for most simple tasks first.
                Applescript for more complex automations, and Ruby for text processing
                or when you need more sophisticated scripting capabilities.
            "#},
            json!({
                "type": "object",
                "required": ["language", "script"],
                "properties": {
                    "language": {
                        "type": "string",
                        "enum": ["shell", "applescript", "ruby"],
                        "description": "The scripting language to use"
                    },
                    "script": {
                        "type": "string",
                        "description": "The script content"
                    },
                    "save_output": {
                        "type": "boolean",
                        "default": false,
                        "description": "Whether to save the script output to a file"
                    }
                }
            }),
        );

        let cache_tool = Tool::new(
            "cache",
            indoc! {r#"
                Manage cached files and data:
                - list: List all cached files
                - view: View content of a cached file
                - delete: Delete a cached file
                - clear: Clear all cached files
            "#},
            json!({
                "type": "object",
                "required": ["operation"],
                "properties": {
                    "operation": {
                        "type": "string",
                        "enum": ["list", "view", "delete", "clear"],
                        "description": "The operation to perform"
                    },
                    "path": {
                        "type": "string",
                        "description": "Path to the cached file for view/delete operations"
                    }
                }
            }),
        );

        // Create cache directory in user's home directory
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join("goose")
            .join("non_developer");
        fs::create_dir_all(&cache_dir).unwrap_or_else(|_| {
            println!(
                "Warning: Failed to create cache directory at {:?}",
                cache_dir
            )
        });

        let instructions = formatdoc! {r#"
            You are a helpful assistant to a power user who is not a professional developer, but you may use devleopment tools to help assist them.
            The user will likely not know how to break down tasks, so you will need to ensure that you do, and run things in batches as needed.
            You can use scripting as needed to work with text files of data, such as csvs, json, or text files etc.
            Using the developer system is allowed, but use it sparingly (and check what cli tools they have already on their system) 
            for more sophisticated tasks or instructed to (js or py can be helpful for more complex tasks if tools are available).

            The NonDeveloperSystem helps you with common tasks like web scraping,
            data processing, and automation without requiring programming expertise.
            
            The user may not have as many tools pre-installed however as a professional developer would, so consider that when running scripts to use what is available.
            Accessing web sites, even apis, may be common (you can use bash scripting to do this) without troubling them too much (they won't know what limits are).

            Try to do your best to find ways to complete a task without too many quesitons or offering options unless it is really unclear, find a way if you can. 
            You can also guide them steps if they can help out as you go along.

            Do use:
                bash_tool when needed.
                screen_capture_tool with list_windows_tool if it helps to see content on screen (you can ask them to open websites, or open them with open bash command to screenshot).
                the developer system when needing to write to files or do something more sophisticated.

            Here are some extra tools:

            web_scrape
              - Fetch content from websites and APIs
              - Save as text, JSON, or binary files
              - Content is cached locally for later use
              - if website doesn't support it find an alternative way.

            data_process
              - Process text data with common operations
              - Filter, extract, sort, count, and more
              - Works with both small and large files
              - Results are saved to new files

            quick_script
              - Create and run simple automation scripts
              - Supports Shell (such as bash), AppleScript (on macos), Ruby (on macos)
              - Scripts can save their output to files

            cache
              - Manage your cached files
              - List, view, delete files
              - Clear all cached data

            The system automatically manages:
            - Cache directory: {cache_dir}
            - File organization and cleanup
            "#,
            cache_dir = cache_dir.display()
        };

        Self {
            tools: vec![
                web_scrape_tool,
                data_process_tool,
                quick_script_tool,
                cache_tool,
            ],
            cache_dir,
            active_resources: Mutex::new(HashMap::new()),
            http_client: Client::new(),
            instructions,
        }
    }

    // Helper function to generate a cache file path
    fn get_cache_path(&self, prefix: &str, extension: &str) -> PathBuf {
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        self.cache_dir
            .join(format!("{}_{}.{}", prefix, timestamp, extension))
    }

    // Helper function to save content to cache
    async fn save_to_cache(
        &self,
        content: &[u8],
        prefix: &str,
        extension: &str,
    ) -> AgentResult<PathBuf> {
        let cache_path = self.get_cache_path(prefix, extension);
        fs::write(&cache_path, content)
            .map_err(|e| AgentError::ExecutionError(format!("Failed to write to cache: {}", e)))?;
        Ok(cache_path)
    }

    // Implement web_scrape tool functionality
    async fn web_scrape(&self, params: Value) -> AgentResult<Vec<Content>> {
        let url = params
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentError::InvalidParameters("Missing 'url' parameter".into()))?;

        let save_as = params
            .get("save_as")
            .and_then(|v| v.as_str())
            .unwrap_or("text");

        // Fetch the content
        let response = self
            .http_client
            .get(url)
            .send()
            .await
            .map_err(|e| AgentError::ExecutionError(format!("Failed to fetch URL: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(AgentError::ExecutionError(format!(
                "HTTP request failed with status: {}",
                status
            )));
        }

        // Process based on save_as parameter
        let (content, extension) = match save_as {
            "text" => {
                let text = response.text().await.map_err(|e| {
                    AgentError::ExecutionError(format!("Failed to get text: {}", e))
                })?;
                (text.into_bytes(), "txt")
            }
            "json" => {
                let text = response.text().await.map_err(|e| {
                    AgentError::ExecutionError(format!("Failed to get text: {}", e))
                })?;
                // Verify it's valid JSON
                serde_json::from_str::<Value>(&text).map_err(|e| {
                    AgentError::ExecutionError(format!("Invalid JSON response: {}", e))
                })?;
                (text.into_bytes(), "json")
            }
            "binary" => {
                let bytes = response.bytes().await.map_err(|e| {
                    AgentError::ExecutionError(format!("Failed to get bytes: {}", e))
                })?;
                (bytes.to_vec(), "bin")
            }
            _ => unreachable!(), // Prevented by enum in tool definition
        };

        // Save to cache
        let cache_path = self.save_to_cache(&content, "web", extension).await?;

        // Register as a resource
        let uri = Url::from_file_path(&cache_path)
            .map_err(|_| AgentError::ExecutionError("Invalid cache path".into()))?
            .to_string();

        let resource = Resource::new(
            uri.clone(),
            Some(save_as.to_string()),
            Some(cache_path.to_string_lossy().into_owned()),
        )
        .map_err(|e| AgentError::ExecutionError(e.to_string()))?;

        self.active_resources.lock().unwrap().insert(uri, resource);

        Ok(vec![Content::text(format!(
            "Content saved to: {}",
            cache_path.display()
        ))])
    }

    // Implement data_process tool functionality
    async fn data_process(&self, params: Value) -> AgentResult<Vec<Content>> {
        let input_path = params
            .get("input_path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AgentError::InvalidParameters("Missing 'input_path' parameter".into())
            })?;

        let operation = params
            .get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentError::InvalidParameters("Missing 'operation' parameter".into()))?;

        // Read input file(s)
        let input_paths: Vec<&str> = input_path.split(',').map(str::trim).collect();
        let mut input_contents = Vec::new();
        for path in input_paths {
            let content = fs::read_to_string(path).map_err(|e| {
                AgentError::ExecutionError(format!("Failed to read input file: {}", e))
            })?;
            input_contents.push(content);
        }

        // Process based on operation
        let (result, extension) = match operation {
            "filter" => {
                let pattern = params
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AgentError::InvalidParameters(
                            "Missing 'pattern' parameter for filter".into(),
                        )
                    })?;
                let regex = regex::Regex::new(pattern).map_err(|e| {
                    AgentError::InvalidParameters(format!("Invalid regex pattern: {}", e))
                })?;
                let filtered: Vec<String> = input_contents[0]
                    .lines()
                    .filter(|line| regex.is_match(line))
                    .map(String::from)
                    .collect();
                (filtered.join("\n"), "txt")
            }
            "extract" => {
                let pattern = params
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AgentError::InvalidParameters(
                            "Missing 'pattern' parameter for extract".into(),
                        )
                    })?;
                let regex = regex::Regex::new(pattern).map_err(|e| {
                    AgentError::InvalidParameters(format!("Invalid regex pattern: {}", e))
                })?;
                let extracted: Vec<String> = input_contents[0]
                    .lines()
                    .filter_map(|line| regex.captures(line))
                    .filter_map(|cap| cap.get(1))
                    .map(|m| m.as_str().to_string())
                    .collect();
                (extracted.join("\n"), "txt")
            }
            "sort" => {
                let mut lines: Vec<String> = input_contents[0].lines().map(String::from).collect();
                lines.sort();
                (lines.join("\n"), "txt")
            }
            "unique" => {
                let mut lines: Vec<String> = input_contents[0].lines().map(String::from).collect();
                lines.sort();
                lines.dedup();
                (lines.join("\n"), "txt")
            }
            "count" => {
                let pattern = params
                    .get("pattern")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AgentError::InvalidParameters(
                            "Missing 'pattern' parameter for count".into(),
                        )
                    })?;
                let regex = regex::Regex::new(pattern).map_err(|e| {
                    AgentError::InvalidParameters(format!("Invalid regex pattern: {}", e))
                })?;
                let count = input_contents[0]
                    .lines()
                    .filter(|line| regex.is_match(line))
                    .count();
                (count.to_string(), "txt")
            }
            "split" => {
                let chunk_size = params
                    .get("chunk_size")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1000) as usize;

                let lines: Vec<&str> = input_contents[0].lines().collect();
                let chunks = lines.chunks(chunk_size);
                let mut paths = Vec::new();

                for (i, chunk) in chunks.enumerate() {
                    let chunk_path = self
                        .save_to_cache(
                            chunk.join("\n").as_bytes(),
                            &format!("split_{}", i + 1),
                            "txt",
                        )
                        .await?;
                    paths.push(chunk_path.display().to_string());
                }

                (paths.join("\n"), "txt")
            }
            "join" => {
                let joined = input_contents.join("\n");
                (joined, "txt")
            }
            _ => unreachable!(), // Prevented by enum in tool definition
        };

        // Save result
        let cache_path = self
            .save_to_cache(result.as_bytes(), operation, extension)
            .await?;

        // Register as a resource
        let uri = Url::from_file_path(&cache_path)
            .map_err(|_| AgentError::ExecutionError("Invalid cache path".into()))?
            .to_string();

        let resource = Resource::new(
            uri.clone(),
            Some("text".to_string()),
            Some(cache_path.to_string_lossy().into_owned()),
        )
        .map_err(|e| AgentError::ExecutionError(e.to_string()))?;

        self.active_resources.lock().unwrap().insert(uri, resource);

        Ok(vec![Content::text(format!(
            "Result saved to: {}",
            cache_path.display()
        ))])
    }

    // Implement quick_script tool functionality
    async fn quick_script(&self, params: Value) -> AgentResult<Vec<Content>> {
        let language = params
            .get("language")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentError::InvalidParameters("Missing 'language' parameter".into()))?;

        let script = params
            .get("script")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentError::InvalidParameters("Missing 'script' parameter".into()))?;

        let save_output = params
            .get("save_output")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Create a temporary directory for the script
        let script_dir = tempfile::tempdir().map_err(|e| {
            AgentError::ExecutionError(format!("Failed to create temporary directory: {}", e))
        })?;

        let command = match language {
            "shell" => {
                let script_path = script_dir.path().join("script.sh");
                fs::write(&script_path, script).map_err(|e| {
                    AgentError::ExecutionError(format!("Failed to write script: {}", e))
                })?;

                fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).map_err(
                    |e| {
                        AgentError::ExecutionError(format!(
                            "Failed to set script permissions: {}",
                            e
                        ))
                    },
                )?;

                script_path.display().to_string()
            }
            "applescript" => {
                if std::env::consts::OS != "macos" {
                    return Err(AgentError::ExecutionError(
                        "AppleScript is only supported on macOS".into(),
                    ));
                }

                let script_path = script_dir.path().join("script.scpt");
                fs::write(&script_path, script).map_err(|e| {
                    AgentError::ExecutionError(format!("Failed to write script: {}", e))
                })?;

                format!("osascript {}", script_path.display())
            }
            "ruby" => {
                if std::env::consts::OS != "macos" {
                    return Err(AgentError::ExecutionError(
                        "Ruby scripting is only supported on macOS".into(),
                    ));
                }

                let script_path = script_dir.path().join("script.rb");
                fs::write(&script_path, script).map_err(|e| {
                    AgentError::ExecutionError(format!("Failed to write script: {}", e))
                })?;

                format!("ruby {}", script_path.display())
            }
            _ => unreachable!(), // Prevented by enum in tool definition
        };

        // Run the script
        let output = Command::new("bash")
            .arg("-c")
            .arg(&command)
            .output()
            .await
            .map_err(|e| AgentError::ExecutionError(format!("Failed to run script: {}", e)))?;

        let output_str = String::from_utf8_lossy(&output.stdout).into_owned();
        let error_str = String::from_utf8_lossy(&output.stderr).into_owned();

        let mut result = if output.status.success() {
            format!("Script completed successfully.\n\nOutput:\n{}", output_str)
        } else {
            format!(
                "Script failed with error code {}.\n\nError:\n{}\nOutput:\n{}",
                output.status, error_str, output_str
            )
        };

        // Save output if requested
        if save_output && !output_str.is_empty() {
            let cache_path = self
                .save_to_cache(output_str.as_bytes(), "script_output", "txt")
                .await?;
            result.push_str(&format!("\n\nOutput saved to: {}", cache_path.display()));

            // Register as a resource
            let uri = Url::from_file_path(&cache_path)
                .map_err(|_| AgentError::ExecutionError("Invalid cache path".into()))?
                .to_string();

            let resource = Resource::new(
                uri.clone(),
                Some("text".to_string()),
                Some(cache_path.to_string_lossy().into_owned()),
            )
            .map_err(|e| AgentError::ExecutionError(e.to_string()))?;

            self.active_resources.lock().unwrap().insert(uri, resource);
        }

        Ok(vec![Content::text(result)])
    }

    // Implement cache tool functionality
    async fn cache(&self, params: Value) -> AgentResult<Vec<Content>> {
        let operation = params
            .get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentError::InvalidParameters("Missing 'operation' parameter".into()))?;

        match operation {
            "list" => {
                let mut files = Vec::new();
                for entry in fs::read_dir(&self.cache_dir).map_err(|e| {
                    AgentError::ExecutionError(format!("Failed to read cache directory: {}", e))
                })? {
                    let entry = entry.map_err(|e| {
                        AgentError::ExecutionError(format!("Failed to read directory entry: {}", e))
                    })?;
                    files.push(format!("{}", entry.path().display()));
                }
                files.sort();
                Ok(vec![Content::text(format!(
                    "Cached files:\n{}",
                    files.join("\n")
                ))])
            }
            "view" => {
                let path = params.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
                    AgentError::InvalidParameters("Missing 'path' parameter for view".into())
                })?;

                let content = fs::read_to_string(path).map_err(|e| {
                    AgentError::ExecutionError(format!("Failed to read file: {}", e))
                })?;

                Ok(vec![Content::text(format!(
                    "Content of {}:\n\n{}",
                    path, content
                ))])
            }
            "delete" => {
                let path = params.get("path").and_then(|v| v.as_str()).ok_or_else(|| {
                    AgentError::InvalidParameters("Missing 'path' parameter for delete".into())
                })?;

                fs::remove_file(path).map_err(|e| {
                    AgentError::ExecutionError(format!("Failed to delete file: {}", e))
                })?;

                // Remove from active resources if present
                if let Ok(url) = Url::from_file_path(path) {
                    self.active_resources
                        .lock()
                        .unwrap()
                        .remove(&url.to_string());
                }

                Ok(vec![Content::text(format!("Deleted file: {}", path))])
            }
            "clear" => {
                fs::remove_dir_all(&self.cache_dir).map_err(|e| {
                    AgentError::ExecutionError(format!("Failed to clear cache directory: {}", e))
                })?;
                fs::create_dir_all(&self.cache_dir).map_err(|e| {
                    AgentError::ExecutionError(format!("Failed to recreate cache directory: {}", e))
                })?;

                // Clear active resources
                self.active_resources.lock().unwrap().clear();

                Ok(vec![Content::text("Cache cleared successfully.")])
            }
            _ => unreachable!(), // Prevented by enum in tool definition
        }
    }
}
#[async_trait]
impl System for NonDeveloperSystem {
    fn name(&self) -> &str {
        "NonDeveloperSystem"
    }

    fn description(&self) -> &str {
        "A system designed for non-developers to help them with common tasks like web scraping, data processing, and automation."
    }

    fn instructions(&self) -> &str {
        &self.instructions
    }

    fn tools(&self) -> &[Tool] {
        &self.tools
    }

    async fn status(&self) -> AnyhowResult<Vec<Resource>> {
        let active_resources = self.active_resources.lock().unwrap();
        Ok(active_resources.values().cloned().collect())
    }

    async fn call(&self, tool_call: ToolCall) -> AgentResult<Vec<Content>> {
        match tool_call.name.as_str() {
            "web_scrape" => self.web_scrape(tool_call.arguments).await,
            "data_process" => self.data_process(tool_call.arguments).await,
            "quick_script" => self.quick_script(tool_call.arguments).await,
            "cache" => self.cache(tool_call.arguments).await,
            _ => Err(AgentError::ToolNotFound(tool_call.name)),
        }
    }

    async fn read_resource(&self, uri: &str) -> AgentResult<String> {
        let active_resources = self.active_resources.lock().unwrap();
        let resource = active_resources
            .get(uri)
            .ok_or_else(|| AgentError::InvalidParameters(format!("Resource not found: {}", uri)))?;

        let url = Url::parse(uri)
            .map_err(|e| AgentError::InvalidParameters(format!("Invalid URI: {}", e)))?;

        if url.scheme() != "file" {
            return Err(AgentError::InvalidParameters(
                "Only file:// URIs are supported".into(),
            ));
        }

        let path = url
            .to_file_path()
            .map_err(|_| AgentError::InvalidParameters("Invalid file path in URI".into()))?;

        match resource.mime_type.as_str() {
            "text" | "json" => fs::read_to_string(&path)
                .map_err(|e| AgentError::ExecutionError(format!("Failed to read file: {}", e))),
            "binary" => {
                let bytes = fs::read(&path).map_err(|e| {
                    AgentError::ExecutionError(format!("Failed to read file: {}", e))
                })?;
                Ok(base64::prelude::BASE64_STANDARD.encode(bytes))
            }
            mime_type => Err(AgentError::InvalidParameters(format!(
                "Unsupported mime type: {}",
                mime_type
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_web_scrape() {
        let system = NonDeveloperSystem::new();

        // Test with a known API endpoint
        let tool_call = ToolCall::new(
            "web_scrape",
            json!({
                "url": "https://httpbin.org/json",
                "save_as": "json"
            }),
        );

        let result = system.call(tool_call).await.unwrap();
        assert!(result[0].as_text().unwrap().contains("saved to:"));
    }

    #[tokio::test]
    async fn test_data_process() {
        let system = NonDeveloperSystem::new();
        let temp_dir = TempDir::new().unwrap();
        let input_path = temp_dir.path().join("test.txt");

        // Create test file
        let mut file = File::create(&input_path).unwrap();
        writeln!(file, "apple\nbanana\napple\ncherry").unwrap();

        // Test unique operation
        let tool_call = ToolCall::new(
            "data_process",
            json!({
                "input_path": input_path.to_str().unwrap(),
                "operation": "unique"
            }),
        );

        let result = system.call(tool_call).await.unwrap();
        assert!(result[0].as_text().unwrap().contains("saved to:"));
    }

    #[tokio::test]
    async fn test_quick_script() {
        let system = NonDeveloperSystem::new();

        // Test shell script
        let tool_call = ToolCall::new(
            "quick_script",
            json!({
                "language": "shell",
                "script": "echo 'Hello, World!'",
                "save_output": true
            }),
        );

        let result = system.call(tool_call).await.unwrap();
        assert!(result[0].as_text().unwrap().contains("Hello, World!"));
    }

    #[tokio::test]
    async fn test_ruby_script() {
        let system = NonDeveloperSystem::new();

        // Skip test if not on macOS
        if std::env::consts::OS != "macos" {
            return;
        }

        // Test Ruby script
        let tool_call = ToolCall::new(
            "quick_script",
            json!({
                "language": "ruby",
                "script": "puts 'Hello from Ruby!'",
                "save_output": true
            }),
        );

        let result = system.call(tool_call).await.unwrap();
        let output = result[0].as_text().unwrap();
        assert!(output.contains("Hello from Ruby!"));
        assert!(output.contains("Script completed successfully"));
    }

    #[tokio::test]
    async fn test_cache() {
        let system = NonDeveloperSystem::new();

        // Test list operation
        let tool_call = ToolCall::new(
            "cache",
            json!({
                "operation": "list"
            }),
        );

        let result = system.call(tool_call).await.unwrap();
        assert!(result[0].as_text().unwrap().contains("Cached files:"));
    }
}
