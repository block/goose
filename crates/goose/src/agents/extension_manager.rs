use anyhow::Result;
use axum::http::{HeaderMap, HeaderName};
use chrono::{DateTime, Utc};
use futures::stream::{FuturesUnordered, StreamExt};
use futures::{future, FutureExt};
use mcp_core::handler::require_str_parameter;
use mcp_core::{ToolCall, ToolError};
use rmcp::service::ClientInitializeError;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::transport::{
    ConfigureCommandExt, SseClientTransport, StreamableHttpClientTransport, TokioChildProcess,
};
use std::collections::{HashMap, HashSet};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::Mutex;
use tokio::task;
use tokio_stream::wrappers::ReceiverStream;
use tokio_util::sync::CancellationToken;
use tracing::{error, warn};

use super::extension::{ExtensionConfig, ExtensionError, ExtensionInfo, ExtensionResult, ToolInfo};
use super::tool_execution::ToolCallResult;
use crate::agents::extension::{Envs, ProcessExit};
use crate::config::{Config, ExtensionConfigManager};
use crate::oauth::oauth_flow;
use crate::prompt_template;
use mcp_client::client::{McpClient, McpClientTrait};
use rmcp::model::{Content, GetPromptResult, Prompt, ResourceContents, Tool};
use rmcp::transport::auth::AuthClient;
use serde_json::Value;

type McpClientBox = Arc<Mutex<Box<dyn McpClientTrait>>>;

/// Manages Goose extensions / MCP clients and their interactions
pub struct ExtensionManager {
    clients: HashMap<String, McpClientBox>,
    instructions: HashMap<String, String>,
    resource_capable_extensions: HashSet<String>,
    temp_dirs: HashMap<String, tempfile::TempDir>,
}

/// A flattened representation of a resource used by the agent to prepare inference
#[derive(Debug, Clone)]
pub struct ResourceItem {
    pub client_name: String,      // The name of the client that owns the resource
    pub uri: String,              // The URI of the resource
    pub name: String,             // The name of the resource
    pub content: String,          // The content of the resource
    pub timestamp: DateTime<Utc>, // The timestamp of the resource
    pub priority: f32,            // The priority of the resource
    pub token_count: Option<u32>, // The token count of the resource (filled in by the agent)
}

impl ResourceItem {
    pub fn new(
        client_name: String,
        uri: String,
        name: String,
        content: String,
        timestamp: DateTime<Utc>,
        priority: f32,
    ) -> Self {
        Self {
            client_name,
            uri,
            name,
            content,
            timestamp,
            priority,
            token_count: None,
        }
    }
}

/// Sanitizes a string by replacing invalid characters with underscores.
/// Valid characters match [a-zA-Z0-9_-]
fn normalize(input: String) -> String {
    let mut result = String::with_capacity(input.len());
    for c in input.chars() {
        result.push(match c {
            c if c.is_ascii_alphanumeric() || c == '_' || c == '-' => c,
            c if c.is_whitespace() => continue, // effectively "strip" whitespace
            _ => '_',                           // Replace any other non-ASCII character with '_'
        });
    }
    result.to_lowercase()
}

pub fn get_parameter_names(tool: &Tool) -> Vec<String> {
    tool.input_schema
        .get("properties")
        .and_then(|props| props.as_object())
        .map(|props| props.keys().cloned().collect())
        .unwrap_or_default()
}

impl Default for ExtensionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ExtensionManager {
    /// Create a new ExtensionManager instance
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            instructions: HashMap::new(),
            resource_capable_extensions: HashSet::new(),
            temp_dirs: HashMap::new(),
        }
    }

    pub fn supports_resources(&self) -> bool {
        !self.resource_capable_extensions.is_empty()
    }

    /// Add a new MCP extension based on the provided client type
    // TODO IMPORTANT need to ensure this times out if the extension command is broken!
    pub async fn add_extension(&mut self, config: ExtensionConfig) -> ExtensionResult<()> {
        let config_name = config.key().to_string();
        let sanitized_name = normalize(config_name.clone());

        /// Helper function to merge environment variables from direct envs and keychain-stored env_keys
        async fn merge_environments(
            envs: &Envs,
            env_keys: &[String],
            ext_name: &str,
        ) -> Result<HashMap<String, String>, ExtensionError> {
            let mut all_envs = envs.get_env();
            let config_instance = Config::global();

            for key in env_keys {
                // If the Envs payload already contains the key, prefer that value
                // over looking into the keychain/secret store
                if all_envs.contains_key(key) {
                    continue;
                }

                match config_instance.get(key, true) {
                    Ok(value) => {
                        if value.is_null() {
                            warn!(
                                key = %key,
                                ext_name = %ext_name,
                                "Secret key not found in config (returned null)."
                            );
                            continue;
                        }

                        // Try to get string value
                        if let Some(str_val) = value.as_str() {
                            all_envs.insert(key.clone(), str_val.to_string());
                        } else {
                            warn!(
                                key = %key,
                                ext_name = %ext_name,
                                value_type = %value.get("type").and_then(|t| t.as_str()).unwrap_or("unknown"),
                                "Secret value is not a string; skipping."
                            );
                        }
                    }
                    Err(e) => {
                        error!(
                            key = %key,
                            ext_name = %ext_name,
                            error = %e,
                            "Failed to fetch secret from config."
                        );
                        return Err(ExtensionError::ConfigError(format!(
                            "Failed to fetch secret '{}' from config: {}",
                            key, e
                        )));
                    }
                }
            }

            Ok(all_envs)
        }

        let client: Box<dyn McpClientTrait> = match &config {
            ExtensionConfig::Sse { uri, timeout, .. } => {
                let transport = SseClientTransport::start(uri.to_string()).await.map_err(
                    |transport_error| {
                        ClientInitializeError::transport::<SseClientTransport<reqwest::Client>>(
                            transport_error,
                            "connect",
                        )
                    },
                )?;
                Box::new(
                    McpClient::connect(
                        transport,
                        Duration::from_secs(
                            timeout.unwrap_or(crate::config::DEFAULT_EXTENSION_TIMEOUT),
                        ),
                    )
                    .await?,
                )
            }
            ExtensionConfig::StreamableHttp {
                uri,
                timeout,
                headers,
                name,
                ..
            } => {
                let mut default_headers = HeaderMap::new();
                for (key, value) in headers {
                    default_headers.insert(
                        HeaderName::try_from(key).map_err(|_| {
                            ExtensionError::ConfigError(format!("invalid header: {}", key))
                        })?,
                        value.parse().map_err(|_| {
                            ExtensionError::ConfigError(format!("invalid header value: {}", key))
                        })?,
                    );
                }
                let client = reqwest::Client::builder()
                    .default_headers(default_headers)
                    .build()
                    .map_err(|_| {
                        ExtensionError::ConfigError("could not construct http client".to_string())
                    })?;
                let transport = StreamableHttpClientTransport::with_client(
                    client,
                    StreamableHttpClientTransportConfig {
                        uri: uri.clone().into(),
                        ..Default::default()
                    },
                );
                let client_res = McpClient::connect(
                    transport,
                    Duration::from_secs(
                        timeout.unwrap_or(crate::config::DEFAULT_EXTENSION_TIMEOUT),
                    ),
                )
                .await;
                let client = if let Err(e) = client_res {
                    // make an attempt at oauth, but failing that, return the original error,
                    // because this might not have been an auth error at all
                    let am = match oauth_flow(uri, name).await {
                        Ok(am) => am,
                        Err(_) => return Err(e.into()),
                    };
                    let client = AuthClient::new(reqwest::Client::default(), am);
                    let transport = StreamableHttpClientTransport::with_client(
                        client,
                        StreamableHttpClientTransportConfig {
                            uri: uri.clone().into(),
                            ..Default::default()
                        },
                    );
                    McpClient::connect(
                        transport,
                        Duration::from_secs(
                            timeout.unwrap_or(crate::config::DEFAULT_EXTENSION_TIMEOUT),
                        ),
                    )
                    .await?
                } else {
                    client_res?
                };
                Box::new(client)
            }
            ExtensionConfig::Stdio {
                cmd,
                args,
                envs,
                env_keys,
                timeout,
                ..
            } => {
                let all_envs = merge_environments(envs, env_keys, &sanitized_name).await?;
                let command = Command::new(cmd).configure(|command| {
                    command.args(args).envs(all_envs);
                });
                let (transport, mut stderr) = TokioChildProcess::builder(command)
                    .stderr(Stdio::piped())
                    .spawn()?;
                let mut stderr = stderr
                    .take()
                    .expect("should have a stderr handle because it was requested");

                let stderr_task = tokio::spawn(async move {
                    let mut all_stderr = Vec::new();
                    stderr.read_to_end(&mut all_stderr).await?;
                    Ok::<String, std::io::Error>(String::from_utf8_lossy(&all_stderr).into())
                });

                let client_result = McpClient::connect(
                    transport,
                    Duration::from_secs(
                        timeout.unwrap_or(crate::config::DEFAULT_EXTENSION_TIMEOUT),
                    ),
                )
                .await;

                let client = match client_result {
                    Ok(client) => Ok(client),
                    Err(error) => {
                        let error_task_out = stderr_task.await?;
                        Err::<McpClient, ExtensionError>(match error_task_out {
                            Ok(stderr_content) => ProcessExit::new(stderr_content, error).into(),
                            Err(e) => e.into(),
                        })
                    }
                }?;

                Box::new(client)
            }
            ExtensionConfig::Builtin {
                name,
                display_name: _,
                description: _,
                timeout,
                bundled: _,
            } => {
                let cmd = std::env::current_exe()
                    .expect("should find the current executable")
                    .to_str()
                    .expect("should resolve executable to string path")
                    .to_string();

                let transport = TokioChildProcess::new(Command::new(cmd).configure(|command| {
                    command.arg("mcp").arg(name);
                }))?;
                Box::new(
                    McpClient::connect(
                        transport,
                        Duration::from_secs(
                            timeout.unwrap_or(crate::config::DEFAULT_EXTENSION_TIMEOUT),
                        ),
                    )
                    .await?,
                )
            }
            ExtensionConfig::InlinePython {
                name,
                code,
                timeout,
                dependencies,
                ..
            } => {
                let temp_dir = tempdir()?;
                let file_path = temp_dir.path().join(format!("{}.py", name));
                std::fs::write(&file_path, code)?;

                let command = Command::new("uvx").configure(|command| {
                    command.arg("--with").arg("mcp");

                    dependencies.iter().flatten().for_each(|dep| {
                        command.arg("--with").arg(dep);
                    });

                    command.arg("python").arg(file_path.to_str().unwrap());
                });
                let transport = TokioChildProcess::new(command)?;

                let client = Box::new(
                    McpClient::connect(
                        transport,
                        Duration::from_secs(
                            timeout.unwrap_or(crate::config::DEFAULT_EXTENSION_TIMEOUT),
                        ),
                    )
                    .await?,
                );

                self.temp_dirs.insert(sanitized_name.clone(), temp_dir);

                client
            }
            _ => unreachable!(),
        };

        let info = client.get_info();
        if let Some(instructions) = info.and_then(|info| info.instructions.as_ref()) {
            self.instructions
                .insert(sanitized_name.clone(), instructions.clone());
        }

        if let Some(_resources) = info.and_then(|info| info.capabilities.resources.as_ref()) {
            self.resource_capable_extensions
                .insert(sanitized_name.clone());
        }

        self.add_client(sanitized_name, client);
        Ok(())
    }

    pub fn add_client(&mut self, client_name: String, client: Box<dyn McpClientTrait>) {
        let sanitized_name = normalize(client_name);
        self.clients
            .insert(sanitized_name, Arc::new(Mutex::new(client)));
    }

    /// Get extensions info
    pub async fn get_extensions_info(&self) -> Vec<ExtensionInfo> {
        self.clients
            .keys()
            .map(|name| {
                let instructions = self.instructions.get(name).cloned().unwrap_or_default();
                let has_resources = self.resource_capable_extensions.contains(name);
                ExtensionInfo::new(name, &instructions, has_resources)
            })
            .collect()
    }

    /// Get aggregated usage statistics
    pub async fn remove_extension(&mut self, name: &str) -> ExtensionResult<()> {
        let sanitized_name = normalize(name.to_string());

        self.clients.remove(&sanitized_name);
        self.instructions.remove(&sanitized_name);
        self.resource_capable_extensions.remove(&sanitized_name);
        self.temp_dirs.remove(&sanitized_name);
        Ok(())
    }

    pub async fn suggest_disable_extensions_prompt(&self) -> Value {
        let enabled_extensions_count = self.clients.len();

        let total_tools = self
            .get_prefixed_tools(None)
            .await
            .map(|tools| tools.len())
            .unwrap_or(0);

        // Check if either condition is met
        const MIN_EXTENSIONS: usize = 5;
        const MIN_TOOLS: usize = 50;

        if enabled_extensions_count > MIN_EXTENSIONS || total_tools > MIN_TOOLS {
            Value::String(format!(
                "The user currently has enabled {} extensions with a total of {} tools. \
                Since this exceeds the recommended limits ({} extensions or {} tools), \
                you should ask the user if they would like to disable some extensions for this session.\n\n\
                Use the search_available_extensions tool to find extensions available to disable. \
                You should only disable extensions found from the search_available_extensions tool. \
                List all the extensions available to disable in the response. \
                Explain that minimizing extensions helps with the recall of the correct tools to use.",
                enabled_extensions_count,
                total_tools,
                MIN_EXTENSIONS,
                MIN_TOOLS,
            ))
        } else {
            Value::String(String::new()) // Empty string if under limits
        }
    }

    pub async fn list_extensions(&self) -> ExtensionResult<Vec<String>> {
        Ok(self.clients.keys().cloned().collect())
    }

    /// Get all tools from all clients with proper prefixing
    pub async fn get_prefixed_tools(
        &self,
        extension_name: Option<String>,
    ) -> ExtensionResult<Vec<Tool>> {
        // Filter clients based on the provided extension_name or include all if None
        let filtered_clients = self.clients.iter().filter(|(name, _)| {
            if let Some(ref name_filter) = extension_name {
                *name == name_filter
            } else {
                true
            }
        });

        let client_futures = filtered_clients.map(|(name, client)| {
            let name = name.clone();
            let client = client.clone();

            task::spawn(async move {
                let mut tools = Vec::new();
                let client_guard = client.lock().await;
                let mut client_tools = client_guard
                    .list_tools(None, CancellationToken::default())
                    .await?;

                loop {
                    for tool in client_tools.tools {
                        tools.push(Tool {
                            name: format!("{}__{}", name, tool.name).into(),
                            description: tool.description,
                            input_schema: tool.input_schema,
                            annotations: tool.annotations,
                            output_schema: tool.output_schema,
                        });
                    }

                    // Exit loop when there are no more pages
                    if client_tools.next_cursor.is_none() {
                        break;
                    }

                    client_tools = client_guard
                        .list_tools(client_tools.next_cursor, CancellationToken::default())
                        .await?;
                }

                Ok::<Vec<Tool>, ExtensionError>(tools)
            })
        });

        // Collect all results concurrently
        let results = future::join_all(client_futures).await;

        // Aggregate tools and handle errors
        let mut tools = Vec::new();
        for result in results {
            match result {
                Ok(Ok(client_tools)) => tools.extend(client_tools),
                Ok(Err(err)) => return Err(err),
                Err(join_err) => return Err(ExtensionError::from(join_err)),
            }
        }

        Ok(tools)
    }

    /// Get the extension prompt including client instructions
    pub async fn get_planning_prompt(&self, tools_info: Vec<ToolInfo>) -> String {
        let mut context: HashMap<&str, Value> = HashMap::new();
        context.insert("tools", serde_json::to_value(tools_info).unwrap());

        prompt_template::render_global_file("plan.md", &context).expect("Prompt should render")
    }

    /// Find and return a reference to the appropriate client for a tool call
    fn get_client_for_tool(&self, prefixed_name: &str) -> Option<(&str, McpClientBox)> {
        self.clients
            .iter()
            .find(|(key, _)| prefixed_name.starts_with(*key))
            .map(|(name, client)| (name.as_str(), Arc::clone(client)))
    }

    // Function that gets executed for read_resource tool
    pub async fn read_resource(
        &self,
        params: Value,
        cancellation_token: CancellationToken,
    ) -> Result<Vec<Content>, ToolError> {
        let uri = require_str_parameter(&params, "uri")?;
        let extension_name = params.get("extension_name").and_then(|v| v.as_str());

        // If extension name is provided, we can just look it up
        if extension_name.is_some() {
            let result = self
                .read_resource_from_extension(
                    uri,
                    extension_name.unwrap(),
                    cancellation_token.clone(),
                )
                .await?;
            return Ok(result);
        }

        // If extension name is not provided, we need to search for the resource across all extensions
        // Loop through each extension and try to read the resource, don't raise an error if the resource is not found
        // TODO: do we want to find if a provided uri is in multiple extensions?
        // currently it will return the first match and skip any others
        for extension_name in self.resource_capable_extensions.iter() {
            let result = self
                .read_resource_from_extension(uri, extension_name, cancellation_token.clone())
                .await;
            match result {
                Ok(result) => return Ok(result),
                Err(_) => continue,
            }
        }

        // None of the extensions had the resource so we raise an error
        let available_extensions = self
            .clients
            .keys()
            .map(|s| s.as_str())
            .collect::<Vec<&str>>()
            .join(", ");
        let error_msg = format!(
            "Resource with uri '{}' not found. Here are the available extensions: {}",
            uri, available_extensions
        );

        Err(ToolError::InvalidParameters(error_msg))
    }

    async fn read_resource_from_extension(
        &self,
        uri: &str,
        extension_name: &str,
        cancellation_token: CancellationToken,
    ) -> Result<Vec<Content>, ToolError> {
        let available_extensions = self
            .clients
            .keys()
            .map(|s| s.as_str())
            .collect::<Vec<&str>>()
            .join(", ");
        let error_msg = format!(
            "Extension '{}' not found. Here are the available extensions: {}",
            extension_name, available_extensions
        );

        let client = self
            .clients
            .get(extension_name)
            .ok_or(ToolError::InvalidParameters(error_msg))?;

        let client_guard = client.lock().await;
        let read_result = client_guard
            .read_resource(uri, cancellation_token)
            .await
            .map_err(|_| {
                ToolError::ExecutionError(format!("Could not read resource with uri: {}", uri))
            })?;

        let mut result = Vec::new();
        for content in read_result.contents {
            // Only reading the text resource content; skipping the blob content cause it's too long
            if let ResourceContents::TextResourceContents { text, .. } = content {
                let content_str = format!("{}\n\n{}", uri, text);
                result.push(Content::text(content_str));
            }
        }

        Ok(result)
    }

    async fn list_resources_from_extension(
        &self,
        extension_name: &str,
        cancellation_token: CancellationToken,
    ) -> Result<Vec<Content>, ToolError> {
        let client = self.clients.get(extension_name).ok_or_else(|| {
            ToolError::InvalidParameters(format!("Extension {} is not valid", extension_name))
        })?;

        let client_guard = client.lock().await;
        client_guard
            .list_resources(None, cancellation_token)
            .await
            .map_err(|e| {
                ToolError::ExecutionError(format!(
                    "Unable to list resources for {}, {:?}",
                    extension_name, e
                ))
            })
            .map(|lr| {
                let resource_list = lr
                    .resources
                    .into_iter()
                    .map(|r| format!("{} - {}, uri: ({})", extension_name, r.name, r.uri))
                    .collect::<Vec<String>>()
                    .join("\n");

                vec![Content::text(resource_list)]
            })
    }

    pub async fn list_resources(
        &self,
        params: Value,
        cancellation_token: CancellationToken,
    ) -> Result<Vec<Content>, ToolError> {
        let extension = params.get("extension").and_then(|v| v.as_str());

        match extension {
            Some(extension_name) => {
                // Handle single extension case
                self.list_resources_from_extension(extension_name, cancellation_token)
                    .await
            }
            None => {
                // Handle all extensions case using FuturesUnordered
                let mut futures = FuturesUnordered::new();

                // Create futures for each resource_capable_extension
                for extension_name in &self.resource_capable_extensions {
                    let token = cancellation_token.clone();
                    futures.push(async move {
                        self.list_resources_from_extension(extension_name, token)
                            .await
                    });
                }

                let mut all_resources = Vec::new();
                let mut errors = Vec::new();

                // Process results as they complete
                while let Some(result) = futures.next().await {
                    match result {
                        Ok(content) => {
                            all_resources.extend(content);
                        }
                        Err(tool_error) => {
                            errors.push(tool_error);
                        }
                    }
                }

                // Log any errors that occurred
                if !errors.is_empty() {
                    tracing::error!(
                        errors = ?errors
                            .into_iter()
                            .map(|e| format!("{:?}", e))
                            .collect::<Vec<_>>(),
                        "errors from listing resources"
                    );
                }

                Ok(all_resources)
            }
        }
    }

    pub async fn dispatch_tool_call(
        &self,
        tool_call: ToolCall,
        cancellation_token: CancellationToken,
    ) -> Result<ToolCallResult> {
        // Dispatch tool call based on the prefix naming convention
        let (client_name, client) = self
            .get_client_for_tool(&tool_call.name)
            .ok_or_else(|| ToolError::NotFound(tool_call.name.clone()))?;

        // rsplit returns the iterator in reverse, tool_name is then at 0
        let tool_name = tool_call
            .name
            .strip_prefix(client_name)
            .and_then(|s| s.strip_prefix("__"))
            .ok_or_else(|| ToolError::NotFound(tool_call.name.clone()))?
            .to_string();

        let arguments = tool_call.arguments.clone();
        let client = client.clone();
        let notifications_receiver = client.lock().await.subscribe().await;

        let fut = async move {
            let client_guard = client.lock().await;
            client_guard
                .call_tool(&tool_name, arguments, cancellation_token)
                .await
                .map(|call| call.content.unwrap_or_default())
                .map_err(|e| ToolError::ExecutionError(e.to_string()))
        };

        Ok(ToolCallResult {
            result: Box::new(fut.boxed()),
            notification_stream: Some(Box::new(ReceiverStream::new(notifications_receiver))),
        })
    }

    pub async fn list_prompts_from_extension(
        &self,
        extension_name: &str,
        cancellation_token: CancellationToken,
    ) -> Result<Vec<Prompt>, ToolError> {
        let client = self.clients.get(extension_name).ok_or_else(|| {
            ToolError::InvalidParameters(format!("Extension {} is not valid", extension_name))
        })?;

        let client_guard = client.lock().await;
        client_guard
            .list_prompts(None, cancellation_token)
            .await
            .map_err(|e| {
                ToolError::ExecutionError(format!(
                    "Unable to list prompts for {}, {:?}",
                    extension_name, e
                ))
            })
            .map(|lp| lp.prompts)
    }

    pub async fn list_prompts(
        &self,
        cancellation_token: CancellationToken,
    ) -> Result<HashMap<String, Vec<Prompt>>, ToolError> {
        let mut futures = FuturesUnordered::new();

        for extension_name in self.clients.keys() {
            let token = cancellation_token.clone();
            futures.push(async move {
                (
                    extension_name,
                    self.list_prompts_from_extension(extension_name, token)
                        .await,
                )
            });
        }

        let mut all_prompts = HashMap::new();
        let mut errors = Vec::new();

        // Process results as they complete
        while let Some(result) = futures.next().await {
            let (name, prompts) = result;
            match prompts {
                Ok(content) => {
                    all_prompts.insert(name.to_string(), content);
                }
                Err(tool_error) => {
                    errors.push(tool_error);
                }
            }
        }

        // Log any errors that occurred
        if !errors.is_empty() {
            tracing::debug!(
                errors = ?errors
                    .into_iter()
                    .map(|e| format!("{:?}", e))
                    .collect::<Vec<_>>(),
                "errors from listing prompts"
            );
        }

        Ok(all_prompts)
    }

    pub async fn get_prompt(
        &self,
        extension_name: &str,
        name: &str,
        arguments: Value,
        cancellation_token: CancellationToken,
    ) -> Result<GetPromptResult> {
        let client = self
            .clients
            .get(extension_name)
            .ok_or_else(|| anyhow::anyhow!("Extension {} not found", extension_name))?;

        let client_guard = client.lock().await;
        client_guard
            .get_prompt(name, arguments, cancellation_token)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get prompt: {}", e))
    }

    pub async fn search_available_extensions(&self) -> Result<Vec<Content>, ToolError> {
        let mut output_parts = vec![];

        // First get disabled extensions from current config
        let mut disabled_extensions: Vec<String> = vec![];
        for extension in ExtensionConfigManager::get_all().expect("should load extensions") {
            if !extension.enabled {
                let config = extension.config.clone();
                let description = match &config {
                    ExtensionConfig::Builtin {
                        name, display_name, ..
                    } => {
                        // For builtin extensions, use display name if available
                        display_name
                            .as_ref()
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| name.clone())
                    }
                    ExtensionConfig::Sse {
                        description, name, ..
                    }
                    | ExtensionConfig::StreamableHttp {
                        description, name, ..
                    }
                    | ExtensionConfig::Stdio {
                        description, name, ..
                    }
                    | ExtensionConfig::InlinePython {
                        description, name, ..
                    } => {
                        // For SSE/StreamableHttp/Stdio/InlinePython, use description if available
                        description
                            .as_ref()
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| format!("Extension '{}'", name))
                    }
                    ExtensionConfig::Frontend { name, .. } => {
                        format!("Frontend extension '{}'", name)
                    }
                };
                disabled_extensions.push(format!("- {} - {}", config.name(), description));
            }
        }

        // Get currently enabled extensions that can be disabled
        let enabled_extensions: Vec<String> = self.clients.keys().cloned().collect();

        // Build output string
        if !disabled_extensions.is_empty() {
            output_parts.push(format!(
                "Extensions available to enable:\n{}\n",
                disabled_extensions.join("\n")
            ));
        } else {
            output_parts.push("No extensions available to enable.\n".to_string());
        }

        if !enabled_extensions.is_empty() {
            output_parts.push(format!(
                "\n\nExtensions available to disable:\n{}\n",
                enabled_extensions
                    .iter()
                    .map(|name| format!("- {}", name))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        } else {
            output_parts.push("No extensions that can be disabled.\n".to_string());
        }

        Ok(vec![Content::text(output_parts.join("\n"))])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcp_client::client::Error;
    use mcp_client::client::McpClientTrait;
    use rmcp::model::CallToolResult;
    use rmcp::model::InitializeResult;

    use rmcp::model::ListPromptsResult;
    use rmcp::model::ListResourcesResult;
    use rmcp::model::ListToolsResult;
    use rmcp::model::ReadResourceResult;
    use rmcp::model::ServerNotification;
    use serde_json::json;
    use tokio::sync::mpsc;

    struct MockClient {}

    #[async_trait::async_trait]
    impl McpClientTrait for MockClient {
        fn get_info(&self) -> Option<&InitializeResult> {
            None
        }

        async fn list_resources(
            &self,
            _next_cursor: Option<String>,
            _cancellation_token: CancellationToken,
        ) -> Result<ListResourcesResult, Error> {
            Err(Error::TransportClosed)
        }

        async fn read_resource(
            &self,
            _uri: &str,
            _cancellation_token: CancellationToken,
        ) -> Result<ReadResourceResult, Error> {
            Err(Error::TransportClosed)
        }

        async fn list_tools(
            &self,
            _next_cursor: Option<String>,
            _cancellation_token: CancellationToken,
        ) -> Result<ListToolsResult, Error> {
            Err(Error::TransportClosed)
        }

        async fn call_tool(
            &self,
            name: &str,
            _arguments: Value,
            _cancellation_token: CancellationToken,
        ) -> Result<CallToolResult, Error> {
            match name {
                "tool" | "test__tool" => Ok(CallToolResult {
                    content: Some(vec![]),
                    is_error: None,
                    structured_content: None,
                }),
                _ => Err(Error::TransportClosed),
            }
        }

        async fn list_prompts(
            &self,
            _next_cursor: Option<String>,
            _cancellation_token: CancellationToken,
        ) -> Result<ListPromptsResult, Error> {
            Err(Error::TransportClosed)
        }

        async fn get_prompt(
            &self,
            _name: &str,
            _arguments: Value,
            _cancellation_token: CancellationToken,
        ) -> Result<GetPromptResult, Error> {
            Err(Error::TransportClosed)
        }

        async fn subscribe(&self) -> mpsc::Receiver<ServerNotification> {
            mpsc::channel(1).1
        }
    }

    #[test]
    fn test_get_client_for_tool() {
        let mut extension_manager = ExtensionManager::new();

        // Add some mock clients
        extension_manager.clients.insert(
            normalize("test_client".to_string()),
            Arc::new(Mutex::new(Box::new(MockClient {}))),
        );

        extension_manager.clients.insert(
            normalize("__client".to_string()),
            Arc::new(Mutex::new(Box::new(MockClient {}))),
        );

        extension_manager.clients.insert(
            normalize("__cli__ent__".to_string()),
            Arc::new(Mutex::new(Box::new(MockClient {}))),
        );

        extension_manager.clients.insert(
            normalize("client 🚀".to_string()),
            Arc::new(Mutex::new(Box::new(MockClient {}))),
        );

        // Test basic case
        assert!(extension_manager
            .get_client_for_tool("test_client__tool")
            .is_some());

        // Test leading underscores
        assert!(extension_manager
            .get_client_for_tool("__client__tool")
            .is_some());

        // Test multiple underscores in client name, and ending with __
        assert!(extension_manager
            .get_client_for_tool("__cli__ent____tool")
            .is_some());

        // Test unicode in tool name, "client 🚀" should become "client_"
        assert!(extension_manager
            .get_client_for_tool("client___tool")
            .is_some());
    }

    #[tokio::test]
    async fn test_dispatch_tool_call() {
        // test that dispatch_tool_call parses out the sanitized name correctly, and extracts
        // tool_names
        let mut extension_manager = ExtensionManager::new();

        // Add some mock clients
        extension_manager.clients.insert(
            normalize("test_client".to_string()),
            Arc::new(Mutex::new(Box::new(MockClient {}))),
        );

        extension_manager.clients.insert(
            normalize("__cli__ent__".to_string()),
            Arc::new(Mutex::new(Box::new(MockClient {}))),
        );

        extension_manager.clients.insert(
            normalize("client 🚀".to_string()),
            Arc::new(Mutex::new(Box::new(MockClient {}))),
        );

        // verify a normal tool call
        let tool_call = ToolCall {
            name: "test_client__tool".to_string(),
            arguments: json!({}),
        };

        let result = extension_manager
            .dispatch_tool_call(tool_call, CancellationToken::default())
            .await;
        assert!(result.is_ok());

        let tool_call = ToolCall {
            name: "test_client__test__tool".to_string(),
            arguments: json!({}),
        };

        let result = extension_manager
            .dispatch_tool_call(tool_call, CancellationToken::default())
            .await;
        assert!(result.is_ok());

        // verify a multiple underscores dispatch
        let tool_call = ToolCall {
            name: "__cli__ent____tool".to_string(),
            arguments: json!({}),
        };

        let result = extension_manager
            .dispatch_tool_call(tool_call, CancellationToken::default())
            .await;
        assert!(result.is_ok());

        // Test unicode in tool name, "client 🚀" should become "client_"
        let tool_call = ToolCall {
            name: "client___tool".to_string(),
            arguments: json!({}),
        };

        let result = extension_manager
            .dispatch_tool_call(tool_call, CancellationToken::default())
            .await;
        assert!(result.is_ok());

        let tool_call = ToolCall {
            name: "client___test__tool".to_string(),
            arguments: json!({}),
        };

        let result = extension_manager
            .dispatch_tool_call(tool_call, CancellationToken::default())
            .await;
        assert!(result.is_ok());

        // this should error out, specifically for an ToolError::ExecutionError
        let invalid_tool_call = ToolCall {
            name: "client___tools".to_string(),
            arguments: json!({}),
        };

        let result = extension_manager
            .dispatch_tool_call(invalid_tool_call, CancellationToken::default())
            .await
            .unwrap()
            .result
            .await;
        assert!(matches!(
            result.err().unwrap(),
            ToolError::ExecutionError(_)
        ));

        // this should error out, specifically with an ToolError::NotFound
        // this client doesn't exist
        let invalid_tool_call = ToolCall {
            name: "_client__tools".to_string(),
            arguments: json!({}),
        };

        let result = extension_manager
            .dispatch_tool_call(invalid_tool_call, CancellationToken::default())
            .await;
        if let Err(err) = result {
            let tool_err = err.downcast_ref::<ToolError>().expect("Expected ToolError");
            assert!(matches!(tool_err, ToolError::NotFound(_)));
        } else {
            panic!("Expected ToolError::NotFound");
        }
    }
}
