mod errors;
mod lang;
mod process_store;

use anyhow::Result;
use indoc::formatdoc;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
};
use tokio::process::Command;
use url::Url;

use mcp_core::{
    handler::{ResourceError, ToolError},
    protocol::ServerCapabilities,
    resource::Resource,
    tool::Tool,
};
use mcp_server::router::{CapabilitiesBuilder, RouterService};

use crate::errors::{AgentError, AgentResult};
use mcp_core::content::Content;
use mcp_core::role::Role;

use mcp_server::{ByteTransport, Router, Server};
use std::process::Stdio;
use std::sync::Mutex;
use tokio::io::{stdin, stdout};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{self, EnvFilter};

pub struct DeveloperRouter {
    tools: Vec<Tool>,
    cwd: Mutex<PathBuf>,
    active_resources: Mutex<HashMap<String, Resource>>,
    file_history: Mutex<HashMap<PathBuf, Vec<String>>>,
    instructions: String,
}

impl DeveloperRouter {
    pub fn new() -> Self {
        let bash_tool = Tool::new(
            "bash".to_string(),
            "Run a bash command in the shell in the current working directory".to_string(),
            json!({
                "type": "object",
                "required": ["command"],
                "properties": {
                    "command": {"type": "string"}
                }
            }),
        );

        let text_editor_tool = Tool::new(
            "text_editor".to_string(),
            "Perform text editing operations on files.".to_string(),
            json!({
                "type": "object",
                "required": ["command", "path"],
                "properties": {
                    "path": {"type": "string"},
                    "command": {
                        "type": "string",
                        "enum": ["view", "write", "str_replace", "undo_edit"]
                    },
                    "new_str": {"type": "string"},
                    "old_str": {"type": "string"},
                    "file_text": {"type": "string"}
                }
            }),
        );

        let instructions = "Developer instructions...".to_string(); // Reuse from original code

        let cwd = std::env::current_dir().unwrap();
        let mut resources = HashMap::new();
        let uri = format!("str:///{}", cwd.display());
        let resource = Resource::new(
            uri.clone(),
            Some("text".to_string()),
            Some("cwd".to_string()),
        )
        .unwrap();
        resources.insert(uri, resource);

        Self {
            tools: vec![bash_tool, text_editor_tool],
            cwd: Mutex::new(cwd),
            active_resources: Mutex::new(resources),
            file_history: Mutex::new(HashMap::new()),
            instructions,
        }
    }

    // Example utility function to call the underlying logic
    async fn call_bash(&self, args: Value) -> Result<Value, ToolError> {
        let result = self.bash(args).await; // adapt your logic from DeveloperSystem
        self.map_agent_result_to_value(result)
    }

    async fn call_text_editor(&self, args: Value) -> Result<Value, ToolError> {
        let result = self.text_editor(args).await; // adapt from DeveloperSystem
        self.map_agent_result_to_value(result)
    }

    // Convert AgentResult<Vec<Content>> to Result<Value, ToolError>
    fn map_agent_result_to_value(
        &self,
        result: AgentResult<Vec<Content>>,
    ) -> Result<Value, ToolError> {
        match result {
            Ok(contents) => {
                let messages: Vec<Value> = contents
                    .iter()
                    .map(|c| {
                        json!({
                            "text": c.as_text().unwrap_or(""),
                            "audience": c.audience(),
                            "priority": c.priority()
                        })
                    })
                    .collect();
                Ok(json!({"messages": messages}))
            }
            Err(e) => Err(e.into()),
        }
    }

    // Helper method to resolve a path relative to cwd
    fn resolve_path(&self, path_str: &str) -> AgentResult<PathBuf> {
        let cwd = self.cwd.lock().unwrap();
        let expanded = shellexpand::tilde(path_str);
        let path = Path::new(expanded.as_ref());
        let resolved_path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            cwd.join(path)
        };

        Ok(resolved_path)
    }

    // Implement bash tool functionality
    async fn bash(&self, params: Value) -> AgentResult<Vec<Content>> {
        let command =
            params
                .get("command")
                .and_then(|v| v.as_str())
                .ok_or(AgentError::InvalidParameters(
                    "The command string is required".into(),
                ))?;

        // Disallow commands that should use other tools
        if command.trim_start().starts_with("cat") {
            return Err(AgentError::InvalidParameters(
                "Do not use `cat` to read files, use the view mode on the text editor tool"
                    .to_string(),
            ));
        }
        // TODO consider enforcing ripgrep over find?

        // Redirect stderr to stdout to interleave outputs
        let cmd_with_redirect = format!("{} 2>&1", command);

        // Execute the command
        let child = Command::new("bash")
            .stdout(Stdio::piped()) // These two pipes required to capture output later.
            .stderr(Stdio::piped())
            .kill_on_drop(true) // Critical so that the command is killed when the agent.reply stream is interrupted.
            .arg("-c")
            .arg(cmd_with_redirect)
            .spawn()
            .map_err(|e| AgentError::ExecutionError(e.to_string()))?;

        // Store the process ID with the command as the key
        let pid: Option<u32> = child.id();
        if let Some(pid) = pid {
            crate::process_store::store_process(pid);
        }

        // Wait for the command to complete and get output
        let output = child
            .wait_with_output()
            .await
            .map_err(|e| AgentError::ExecutionError(e.to_string()))?;

        // Remove the process ID from the store
        if let Some(pid) = pid {
            crate::process_store::remove_process(pid);
        }

        let output_str = format!(
            "Finished with Status Code: {}\nOutput:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout)
        );
        Ok(vec![
            Content::text(output_str.clone()).with_audience(vec![Role::Assistant]),
            Content::text(output_str)
                .with_audience(vec![Role::User])
                .with_priority(0.0),
        ])
    }

    // Implement text_editor tool functionality
    async fn text_editor(&self, params: Value) -> AgentResult<Vec<Content>> {
        let command = params
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentError::InvalidParameters("Missing 'command' parameter".into()))?;

        let path_str = params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentError::InvalidParameters("Missing 'path' parameter".into()))?;

        let path = self.resolve_path(path_str)?;

        match command {
            "view" => self.text_editor_view(&path).await,
            "write" => {
                let file_text = params
                    .get("file_text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AgentError::InvalidParameters("Missing 'file_text' parameter".into())
                    })?;

                self.text_editor_write(&path, file_text).await
            }
            "str_replace" => {
                let old_str = params
                    .get("old_str")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AgentError::InvalidParameters("Missing 'old_str' parameter".into())
                    })?;
                let new_str = params
                    .get("new_str")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        AgentError::InvalidParameters("Missing 'new_str' parameter".into())
                    })?;

                self.text_editor_replace(&path, old_str, new_str).await
            }
            "undo_edit" => self.text_editor_undo(&path).await,
            _ => Err(AgentError::InvalidParameters(format!(
                "Unknown command '{}'",
                command
            ))),
        }
    }

    async fn text_editor_view(&self, path: &PathBuf) -> AgentResult<Vec<Content>> {
        if path.is_file() {
            // Check file size first (2MB limit)
            const MAX_FILE_SIZE: u64 = 2 * 1024 * 1024; // 2MB in bytes
            const MAX_CHAR_COUNT: usize = 1 << 20; // 2^20 characters (1,048,576)

            let file_size = std::fs::metadata(path)
                .map_err(|e| {
                    AgentError::ExecutionError(format!("Failed to get file metadata: {}", e))
                })?
                .len();

            if file_size > MAX_FILE_SIZE {
                return Err(AgentError::ExecutionError(format!(
                    "File '{}' is too large ({:.2}MB). Maximum size is 2MB to prevent memory issues.",
                    path.display(),
                    file_size as f64 / 1024.0 / 1024.0
                )));
            }

            // Create a new resource and add it to active_resources
            let uri = Url::from_file_path(path)
                .map_err(|_| AgentError::ExecutionError("Invalid file path".into()))?
                .to_string();

            // Read the content once
            let content = std::fs::read_to_string(path)
                .map_err(|e| AgentError::ExecutionError(format!("Failed to read file: {}", e)))?;

            let char_count = content.chars().count();
            if char_count > MAX_CHAR_COUNT {
                return Err(AgentError::ExecutionError(format!(
                    "File '{}' has too many characters ({}). Maximum character count is {}.",
                    path.display(),
                    char_count,
                    MAX_CHAR_COUNT
                )));
            }

            // Create and store the resource
            let resource =
                Resource::new(uri.clone(), Some("text".to_string()), None).map_err(|e| {
                    AgentError::ExecutionError(format!("Failed to create resource: {}", e))
                })?;

            self.active_resources.lock().unwrap().insert(uri, resource);

            let language = lang::get_language_identifier(path);
            let formatted = formatdoc! {"
                ### {path}
                ```{language}
                {content}
                ```
                ",
                path=path.display(),
                language=language,
                content=content,
            };

            // The LLM gets just a quick update as we expect the file to view in the status
            // but we send a low priority message for the human
            Ok(vec![
                Content::text(format!(
                    "The file content for {} is now available in the system status.",
                    path.display()
                ))
                .with_audience(vec![Role::Assistant]),
                Content::text(formatted)
                    .with_audience(vec![Role::User])
                    .with_priority(0.0),
            ])
        } else {
            Err(AgentError::ExecutionError(format!(
                "The path '{}' does not exist or is not a file.",
                path.display()
            )))
        }
    }

    async fn text_editor_write(
        &self,
        path: &PathBuf,
        file_text: &str,
    ) -> AgentResult<Vec<Content>> {
        // Get the URI for the file
        let uri = Url::from_file_path(path)
            .map_err(|_| AgentError::ExecutionError("Invalid file path".into()))?
            .to_string();

        // Check if file already exists and is active
        if path.exists() && !self.active_resources.lock().unwrap().contains_key(&uri) {
            return Err(AgentError::InvalidParameters(format!(
                "File '{}' exists but is not active. View it first before overwriting.",
                path.display()
            )));
        }

        // Save history for undo
        self.save_file_history(path)?;

        // Write to the file
        std::fs::write(path, file_text)
            .map_err(|e| AgentError::ExecutionError(format!("Failed to write file: {}", e)))?;

        // Create and store resource

        let resource = Resource::new(uri.clone(), Some("text".to_string()), None)
            .map_err(|e| AgentError::ExecutionError(e.to_string()))?;
        self.active_resources.lock().unwrap().insert(uri, resource);

        // Try to detect the language from the file extension
        let language = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        Ok(vec![
            Content::text(format!("Successfully wrote to {}", path.display()))
                .with_audience(vec![Role::Assistant]),
            Content::text(formatdoc! {r#"
                ### {path}
                ```{language}
                {content}
                ```
                "#,
                path=path.display(),
                language=language,
                content=file_text,
            })
            .with_audience(vec![Role::User])
            .with_priority(0.2),
        ])
    }

    async fn text_editor_replace(
        &self,
        path: &PathBuf,
        old_str: &str,
        new_str: &str,
    ) -> AgentResult<Vec<Content>> {
        // Get the URI for the file
        let uri = Url::from_file_path(path)
            .map_err(|_| AgentError::ExecutionError("Invalid file path".into()))?
            .to_string();

        // Check if file exists and is active
        if !path.exists() {
            return Err(AgentError::InvalidParameters(format!(
                "File '{}' does not exist",
                path.display()
            )));
        }
        if !self.active_resources.lock().unwrap().contains_key(&uri) {
            return Err(AgentError::InvalidParameters(format!(
                "You must view '{}' before editing it",
                path.display()
            )));
        }

        // Read content
        let content = std::fs::read_to_string(path)
            .map_err(|e| AgentError::ExecutionError(format!("Failed to read file: {}", e)))?;

        // Ensure 'old_str' appears exactly once
        if content.matches(old_str).count() > 1 {
            return Err(AgentError::InvalidParameters(
                "'old_str' must appear exactly once in the file, but it appears multiple times"
                    .into(),
            ));
        }
        if content.matches(old_str).count() == 0 {
            return Err(AgentError::InvalidParameters(
                "'old_str' must appear exactly once in the file, but it does not appear in the file. Make sure the string exactly matches existing file content, including spacing.".into(),
            ));
        }

        // Save history for undo
        self.save_file_history(path)?;

        // Replace and write back
        let new_content = content.replace(old_str, new_str);
        std::fs::write(path, &new_content)
            .map_err(|e| AgentError::ExecutionError(format!("Failed to write file: {}", e)))?;

        // Update resource
        if let Some(resource) = self.active_resources.lock().unwrap().get_mut(&uri) {
            resource.update_timestamp();
        }

        // Try to detect the language from the file extension
        let language = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        Ok(vec![
            Content::text("Successfully replaced text").with_audience(vec![Role::Assistant]),
            Content::text(formatdoc! {r#"
                ### {path}

                *Before*:
                ```{language}
                {old_str}
                ```

                *After*:
                ```{language}
                {new_str}
                ```
                "#,
                path=path.display(),
                language=language,
                old_str=old_str,
                new_str=new_str,
            })
            .with_audience(vec![Role::User])
            .with_priority(0.2),
        ])
    }

    async fn text_editor_undo(&self, path: &PathBuf) -> AgentResult<Vec<Content>> {
        let mut history = self.file_history.lock().unwrap();
        if let Some(contents) = history.get_mut(path) {
            if let Some(previous_content) = contents.pop() {
                // Write previous content back to file
                std::fs::write(path, previous_content).map_err(|e| {
                    AgentError::ExecutionError(format!("Failed to write file: {}", e))
                })?;
                Ok(vec![Content::text("Undid the last edit")])
            } else {
                Err(AgentError::InvalidParameters(
                    "No edit history available to undo".into(),
                ))
            }
        } else {
            Err(AgentError::InvalidParameters(
                "No edit history available to undo".into(),
            ))
        }
    }

    fn save_file_history(&self, path: &PathBuf) -> AgentResult<()> {
        let mut history = self.file_history.lock().unwrap();
        let content = if path.exists() {
            std::fs::read_to_string(path)
                .map_err(|e| AgentError::ExecutionError(format!("Failed to read file: {}", e)))?
        } else {
            String::new()
        };
        history.entry(path.clone()).or_default().push(content);
        Ok(())
    }

    async fn do_read_resource(&self, uri: &str) -> AgentResult<String> {
        let content = self.read_resource(uri).await.map_err(AgentError::from)?;
        Ok(content)
    }
}

impl Router for DeveloperRouter {
    fn instructions(&self) -> String {
        self.instructions.clone()
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new().with_tools(true).build()
    }

    fn list_tools(&self) -> Vec<Tool> {
        self.tools.clone()
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Pin<Box<dyn Future<Output = Result<Value, ToolError>> + Send + 'static>> {
        let this = self.clone();
        let tool_name = tool_name.to_string();
        Box::pin(async move {
            match tool_name.as_str() {
                "bash" => this.call_bash(arguments).await,
                "text_editor" => this.call_text_editor(arguments).await,
                _ => Err(ToolError::NotFound(format!("Tool {} not found", tool_name))),
            }
        })
    }

    fn list_resources(&self) -> Vec<Resource> {
        self.active_resources
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect()
    }

    fn read_resource(
        &self,
        uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        let this = self.clone();
        let uri = uri.to_string();
        Box::pin(async move {
            match this.do_read_resource(&uri).await {
                Ok(content) => Ok(content),
                Err(e) => Err(e.into()),
            }
        })
    }
}

impl Clone for DeveloperRouter {
    fn clone(&self) -> Self {
        Self {
            tools: self.tools.clone(),
            cwd: Mutex::new(self.cwd.lock().unwrap().clone()),
            active_resources: Mutex::new(self.active_resources.lock().unwrap().clone()),
            file_history: Mutex::new(self.file_history.lock().unwrap().clone()),
            instructions: self.instructions.clone(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Set up file appender for logging
    let file_appender = RollingFileAppender::new(Rotation::DAILY, "logs", "mcp-server.log");

    // Initialize the tracing subscriber with file and stdout logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_writer(file_appender)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    tracing::info!("Starting MCP server");

    // Create an instance of our counter router
    let router = RouterService(DeveloperRouter::new());

    // Create and run the server
    let server = Server::new(router);
    let transport = ByteTransport::new(stdin(), stdout());

    tracing::info!("Server initialized and ready to handle requests");
    Ok(server.run(transport).await?)
}
