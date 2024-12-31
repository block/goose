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
        let web_search_tool = Tool::new(
            "web_search",
            indoc! {r#"
                Search the web for a single word (proper noun ideally) using DuckDuckGo's API. Returns results in JSON format.
                The results are cached locally for future reference.
                Be sparing as there is a limited number of api calls allowed.
            "#},
            json!({
                "type": "object",
                "required": ["query"],
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "A single word to search for, a topic, propernoun, brand name that you may not know about"
                    }
                }
            }),
        );

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

        let quick_script_tool = Tool::new(
            "quick_script",
            indoc! {r#"
                Create and run small scripts for automation tasks.
                Supports Shell, AppleScript, and Ruby (on macOS).
                
                The script is saved to a temporary file and executed.
                Consider using shell script (bash) for most simple tasks first.
                Applescript for more complex automations (and controlling applications which may not have an api, but do be careful to ensure not too much data is returned at once), and Ruby for text processing or when you need more sophisticated scripting capabilities.

                Some examples of shell:
                    - create a sorted list of unique lines: sort file.txt | uniq
                    - extract 2nd column in csv: awk -F "," '{ print $2}'
                    - pattern matching: grep pattern file.txt
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
                "required": ["command"],
                "properties": {
                    "command": {
                        "type": "string",
                        "enum": ["list", "view", "delete", "clear"],
                        "description": "The command to perform"
                    },
                    "path": {
                        "type": "string",
                        "description": "Path to the cached file for view/delete commands"
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
            The user may not know how to break down tasks, so you will need to ensure that you do, and run things in batches as needed.
            The NonDeveloperSystem helps you with common tasks like web scraping,
            data processing, and automation without requiring programming expertise,
            supplementing the Developer System.

            You can use scripting as needed to work with text files of data, such as csvs, json, or text files etc.
            Using the developer system is allowed for more sophisticated tasks or instructed to (js or py can be helpful for more complex tasks if tools are available).
            
            Accessing web sites, even apis, may be common (you can use bash scripting to do this) without troubling them too much (they won't know what limits are).

            Try to do your best to find ways to complete a task without too many quesitons or offering options unless it is really unclear, find a way if you can. 
            You can also guide them steps if they can help out as you go along.

            Here are some extra tools:

            web_scrape
              - Fetch content from websites and APIs
              - Save as text, JSON, or binary files
              - Content is cached locally for later use
              - if website doesn't support it find an alternative way.

            quick_script
              - Create and run simple automation scripts
              - Supports Shell (such as bash), AppleScript (on macos), Ruby (on macos)
              - Scripts can save their output to files
              - on macos, can use applescript to interact with the desktop, eg calendars, notes and more, anything apple script can do for apps that support it: 
                    AppleScript is a powerful scripting language designed for automating tasks on macOS. It allows users to control applications and system features programmatically. Here's an overview of what AppleScript can automate:
                    Application Control
                        Launch, quit, or manage applications.
                        Interact with app-specific features (e.g., sending an email in Mail, creating a document in Pages, or editing photos in Preview).
                        Perform tasks in third-party apps that support AppleScript, such as Adobe Photoshop, Microsoft Office, or Safari.
                    User Interface Automation
                        Simulate user interactions like clicking buttons, selecting menu items, or typing text.
                        Fill out forms or automate repetitive tasks in apps.
                    System Settings and Utilities
                        Change system preferences (e.g., volume, screen brightness, Wi-Fi settings).
                        Automate tasks like shutting down, restarting, or putting the system to sleep.
                        Monitor system events or logs.
                    Web Automation
                        Open specific URLs in Safari or other AppleScript-enabled browsers.
                        Automate web interactions (e.g., filling forms, navigating pages).
                        Scrape information from websites.
                    Email and Messaging
                        Automate sending and organizing emails in the Mail app.
                        Extract email contents or attachments.
                        Send messages via Messages.
                    Media Management
                        Organize and edit iTunes/Music libraries (e.g., create playlists, change metadata).
                        Manage photos in Photos (e.g., creating albums, importing/exporting images).
                        Automate tasks in video or music production tools like Final Cut Pro or GarageBand.
                    Data Processing
                        Process text files or other types of documents.
                        Extract or format data from files or apps.
                        Interact with spreadsheets (e.g., Numbers or Excel).
                    Integration with Other Scripts
                        Execute shell scripts, Ruby scripts, or other automation scripts.
                        Combine workflows across scripting languages.
                    Complex Workflows
                        Automate multi-step tasks involving multiple apps or system features.
                        Create scheduled tasks using Calendar or other scheduling apps.

            web_search
              - Search the web using DuckDuckGo's API for general topics or keywords

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
                web_search_tool,
                web_scrape_tool,
                quick_script_tool,
                cache_tool,
            ],
            cache_dir,
            active_resources: Mutex::new(HashMap::new()),
            http_client: Client::builder().user_agent("Goose/1.0").build().unwrap(),
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

    // Helper function to register a file as a resource
    fn register_as_resource(&self, cache_path: &PathBuf, mime_type: &str) -> AgentResult<()> {
        let uri = Url::from_file_path(cache_path)
            .map_err(|_| AgentError::ExecutionError("Invalid cache path".into()))?
            .to_string();

        let resource = Resource::new(
            uri.clone(),
            Some(mime_type.to_string()),
            Some(cache_path.to_string_lossy().into_owned()),
        )
        .map_err(|e| AgentError::ExecutionError(e.to_string()))?;

        self.active_resources.lock().unwrap().insert(uri, resource);
        Ok(())
    }

    // Implement web_scrape tool functionality
    async fn web_search(&self, params: Value) -> AgentResult<Vec<Content>> {
        let query = params
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentError::InvalidParameters("Missing 'query' parameter".into()))?;

        // Create the DuckDuckGo API URL
        let url = format!(
            "https://api.duckduckgo.com/?q={}&format=json&pretty=1",
            urlencoding::encode(query)
        );

        // Fetch the results
        let response = self.http_client.get(&url).send().await.map_err(|e| {
            AgentError::ExecutionError(format!("Failed to fetch search results: {}", e))
        })?;

        let status = response.status();
        if !status.is_success() {
            return Err(AgentError::ExecutionError(format!(
                "HTTP request failed with status: {}",
                status
            )));
        }

        // Get the JSON response
        let json_text = response.text().await.map_err(|e| {
            AgentError::ExecutionError(format!("Failed to get response text: {}", e))
        })?;

        // Save to cache
        let cache_path = self
            .save_to_cache(json_text.as_bytes(), "search", "json")
            .await?;

        // Register as a resource
        self.register_as_resource(&cache_path, "json")?;

        Ok(vec![Content::text(format!(
            "Search results saved to: {}",
            cache_path.display()
        ))])
    }

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
        self.register_as_resource(&cache_path, save_as)?;

        Ok(vec![Content::text(format!(
            "Content saved to: {}",
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
            self.register_as_resource(&cache_path, "text")?;
        }

        Ok(vec![Content::text(result)])
    }

    // Implement cache tool functionality
    async fn cache(&self, params: Value) -> AgentResult<Vec<Content>> {
        let command = params
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentError::InvalidParameters("Missing 'command' parameter".into()))?;

        match command {
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
            "web_search" => self.web_search(tool_call.arguments).await,
            "web_scrape" => self.web_scrape(tool_call.arguments).await,
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

    #[tokio::test]
    async fn test_web_search() {
        let system = NonDeveloperSystem::new();

        // Test with a simple search query
        let tool_call = ToolCall::new(
            "web_search",
            json!({
                "query": "rust programming"
            }),
        );

        let result = system.call(tool_call).await.unwrap();
        assert!(result[0].as_text().unwrap().contains("saved to:"));
    }

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

        // Test list command
        let tool_call = ToolCall::new(
            "cache",
            json!({
                "command": "list"
            }),
        );

        let result = system.call(tool_call).await.unwrap();
        assert!(result[0].as_text().unwrap().contains("Cached files:"));
    }
}
