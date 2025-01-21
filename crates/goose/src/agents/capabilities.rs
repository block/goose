use chrono::{DateTime, TimeZone, Utc};
use futures::stream::{FuturesUnordered, StreamExt};
use mcp_client::McpService;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use super::system::{SystemConfig, SystemError, SystemInfo, SystemResult};
use crate::prompt_template::load_prompt_file;
use crate::providers::base::{Provider, ProviderUsage};
use mcp_client::client::{ClientCapabilities, ClientInfo, McpClient, McpClientTrait};
use mcp_client::transport::{SseTransport, StdioTransport, Transport};
use mcp_core::{Content, Tool, ToolCall, ToolError, ToolResult};
use serde_json::Value;

// By default, we set it to Jan 1, 2020 if the resource does not have a timestamp
// This is to ensure that the resource is considered less important than resources with a more recent timestamp
static DEFAULT_TIMESTAMP: LazyLock<DateTime<Utc>> =
    LazyLock::new(|| Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap());

/// Manages MCP clients and their interactions
pub struct Capabilities {
    clients: HashMap<String, Arc<Mutex<Box<dyn McpClientTrait>>>>,
    instructions: HashMap<String, String>,
    resource_capable_systems: HashSet<String>,
    provider: Box<dyn Provider>,
    provider_usage: Mutex<Vec<ProviderUsage>>,
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
fn sanitize(input: String) -> String {
    let mut result = String::with_capacity(input.len());
    for c in input.chars() {
        result.push(if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
            c
        } else {
            '_'
        });
    }
    result
}

impl Capabilities {
    /// Create a new Capabilities with the specified provider
    pub fn new(provider: Box<dyn Provider>) -> Self {
        Self {
            clients: HashMap::new(),
            instructions: HashMap::new(),
            resource_capable_systems: HashSet::new(),
            provider,
            provider_usage: Mutex::new(Vec::new()),
        }
    }

    pub fn supports_resources(&self) -> bool {
        !self.resource_capable_systems.is_empty()
    }

    /// Add a new MCP system based on the provided client type
    // TODO IMPORTANT need to ensure this times out if the system command is broken!
    pub async fn add_system(&mut self, config: SystemConfig) -> SystemResult<()> {
        let mut client: Box<dyn McpClientTrait> = match config {
            SystemConfig::Sse { ref uri, ref envs } => {
                let transport = SseTransport::new(uri, envs.get_env());
                let handle = transport.start().await?;
                let service = McpService::with_timeout(handle, Duration::from_secs(100));
                Box::new(McpClient::new(service))
            }
            SystemConfig::Stdio {
                ref cmd,
                ref args,
                ref envs,
            } => {
                let transport = StdioTransport::new(cmd, args.to_vec(), envs.get_env());
                let handle = transport.start().await?;
                let service = McpService::with_timeout(handle, Duration::from_secs(100));
                Box::new(McpClient::new(service))
            }
            SystemConfig::Builtin { ref name } => {
                // For builtin systems, we run the current executable with mcp and system name
                let cmd = std::env::current_exe()
                    .expect("should find the current executable")
                    .to_str()
                    .expect("should resolve executable to string path")
                    .to_string();
                let transport = StdioTransport::new(
                    &cmd,
                    vec!["mcp".to_string(), name.clone()],
                    HashMap::new(),
                );
                let handle = transport.start().await?;
                let service = McpService::with_timeout(handle, Duration::from_secs(100));
                Box::new(McpClient::new(service))
            }
        };

        // Initialize the client with default capabilities
        let info = ClientInfo {
            name: "goose".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        };
        let capabilities = ClientCapabilities::default();

        let init_result = client
            .initialize(info, capabilities)
            .await
            .map_err(|_| SystemError::Initialization(config.clone()))?;

        // Store instructions if provided
        if let Some(instructions) = init_result.instructions {
            self.instructions
                .insert(init_result.server_info.name.clone(), instructions);
        }

        // if the server is capable if resources we track it
        if init_result.capabilities.resources.is_some() {
            self.resource_capable_systems
                .insert(sanitize(init_result.server_info.name.clone()));
        }

        // Store the client
        self.clients.insert(
            sanitize(init_result.server_info.name.clone()),
            Arc::new(Mutex::new(client)),
        );

        Ok(())
    }

    /// Get a reference to the provider
    pub fn provider(&self) -> &dyn Provider {
        &*self.provider
    }

    /// Record provider usage
    // TODO consider moving this off to the provider or as a form of logging
    pub async fn record_usage(&self, usage: ProviderUsage) {
        self.provider_usage.lock().await.push(usage);
    }

    /// Get aggregated usage statistics
    pub async fn remove_system(&mut self, name: &str) -> SystemResult<()> {
        self.clients.remove(name);
        Ok(())
    }

    pub async fn list_systems(&self) -> SystemResult<Vec<String>> {
        let mut systems = Vec::new();
        for name in self.clients.keys() {
            systems.push(name.clone());
        }
        Ok(systems)
    }

    pub async fn get_usage(&self) -> Vec<ProviderUsage> {
        let provider_usage = self.provider_usage.lock().await.clone();
        let mut usage_map: HashMap<String, ProviderUsage> = HashMap::new();

        provider_usage.iter().for_each(|usage| {
            usage_map
                .entry(usage.model.clone())
                .and_modify(|e| {
                    e.usage.input_tokens = Some(
                        e.usage.input_tokens.unwrap_or(0) + usage.usage.input_tokens.unwrap_or(0),
                    );
                    e.usage.output_tokens = Some(
                        e.usage.output_tokens.unwrap_or(0) + usage.usage.output_tokens.unwrap_or(0),
                    );
                    e.usage.total_tokens = Some(
                        e.usage.total_tokens.unwrap_or(0) + usage.usage.total_tokens.unwrap_or(0),
                    );
                })
                .or_insert_with(|| usage.clone());
        });
        usage_map.into_values().collect()
    }

    /// Get all tools from all clients with proper prefixing
    pub async fn get_prefixed_tools(&mut self) -> SystemResult<Vec<Tool>> {
        let mut tools = Vec::new();
        for (name, client) in &self.clients {
            let client_guard = client.lock().await;
            let mut client_tools = client_guard.list_tools(None).await?;

            loop {
                for tool in client_tools.tools {
                    tools.push(Tool::new(
                        format!("{}__{}", name, tool.name),
                        &tool.description,
                        tool.input_schema,
                    ));
                }

                // exit loop when there are no more pages
                if client_tools.next_cursor.is_none() {
                    break;
                }

                client_tools = client_guard.list_tools(client_tools.next_cursor).await?;
            }
        }
        Ok(tools)
    }

    /// Get client resources and their contents
    pub async fn get_resources(&self) -> SystemResult<Vec<ResourceItem>> {
        let mut result: Vec<ResourceItem> = Vec::new();

        for (name, client) in &self.clients {
            let client_guard = client.lock().await;
            let resources = client_guard.list_resources(None).await?;

            for resource in resources.resources {
                // Skip reading the resource if it's not marked active
                // This avoids blowing up the context with inactive resources
                if !resource.is_active() {
                    continue;
                }

                if let Ok(contents) = client_guard.read_resource(&resource.uri).await {
                    for content in contents.contents {
                        let (uri, content_str) = match content {
                            mcp_core::resource::ResourceContents::TextResourceContents {
                                uri,
                                text,
                                ..
                            } => (uri, text),
                            mcp_core::resource::ResourceContents::BlobResourceContents {
                                uri,
                                blob,
                                ..
                            } => (uri, blob),
                        };

                        result.push(ResourceItem::new(
                            name.clone(),
                            uri,
                            resource.name.clone(),
                            content_str,
                            resource.timestamp().unwrap_or(*DEFAULT_TIMESTAMP),
                            resource.priority().unwrap_or(0.0),
                        ));
                    }
                }
            }
        }
        Ok(result)
    }

    /// Get the system prompt including client instructions
    pub async fn get_system_prompt(&self) -> String {
        let mut context = HashMap::new();
        let systems_info: Vec<SystemInfo> = self
            .clients
            .keys()
            .map(|name| {
                let instructions = self.instructions.get(name).cloned().unwrap_or_default();
                let has_resources = self.resource_capable_systems.contains(name);
                SystemInfo::new(name, &instructions, has_resources)
            })
            .collect();

        context.insert("systems", systems_info);
        load_prompt_file("system.md", &context).expect("Prompt should render")
    }

    /// Find and return a reference to the appropriate client for a tool call
    fn get_client_for_tool(
        &self,
        prefixed_name: &str,
    ) -> Option<Arc<Mutex<Box<dyn McpClientTrait>>>> {
        prefixed_name
            .split_once("__")
            .and_then(|(client_name, _)| self.clients.get(client_name))
            .map(Arc::clone)
    }

    // Function that gets executed for read_resource tool
    async fn read_resource(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let uri = params
            .get("uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("Missing 'uri' parameter".to_string()))?;

        let system_name = params.get("system_name").and_then(|v| v.as_str());

        // If system name is provided, we can just look it up
        if system_name.is_some() {
            let result = self
                .read_resource_from_system(uri, system_name.unwrap())
                .await?;
            return Ok(result);
        }

        // If system name is not provided, we need to search for the resource across all systems
        // Loop through each system and try to read the resource, don't raise an error if the resource is not found
        // TODO: do we want to find if a provided uri is in multiple systems?
        // currently it will reutrn the first match and skip any systems
        for system_name in self.resource_capable_systems.iter() {
            let result = self.read_resource_from_system(uri, system_name).await;
            match result {
                Ok(result) => return Ok(result),
                Err(_) => continue,
            }
        }

        // None of the systems had the resource so we raise an error
        let available_systems = self
            .clients
            .keys()
            .map(|s| s.as_str())
            .collect::<Vec<&str>>()
            .join(", ");
        let error_msg = format!(
            "Resource with uri '{}' not found. Here are the available systems: {}",
            uri, available_systems
        );

        Err(ToolError::InvalidParameters(error_msg))
    }

    async fn read_resource_from_system(
        &self,
        uri: &str,
        system_name: &str,
    ) -> Result<Vec<Content>, ToolError> {
        let available_systems = self
            .clients
            .keys()
            .map(|s| s.as_str())
            .collect::<Vec<&str>>()
            .join(", ");
        let error_msg = format!(
            "System '{}' not found. Here are the available systems: {}",
            system_name, available_systems
        );

        let client = self
            .clients
            .get(system_name)
            .ok_or(ToolError::InvalidParameters(error_msg))?;

        let client_guard = client.lock().await;
        let read_result = client_guard.read_resource(uri).await.map_err(|_| {
            ToolError::ExecutionError(format!("Could not read resource with uri: {}", uri))
        })?;

        let mut result = Vec::new();
        for content in read_result.contents {
            // Only reading the text resource content; skipping the blob content cause it's too long
            if let mcp_core::resource::ResourceContents::TextResourceContents { text, .. } = content
            {
                let content_str = format!("{}\n\n{}", uri, text);
                result.push(Content::text(content_str));
            }
        }

        Ok(result)
    }

    async fn list_resources_from_system(
        &self,
        system_name: &str,
    ) -> Result<Vec<Content>, ToolError> {
        let client = self.clients.get(system_name).ok_or_else(|| {
            ToolError::InvalidParameters(format!("System {} is not valid", system_name))
        })?;

        let client_guard = client.lock().await;
        client_guard
            .list_resources(None)
            .await
            .map_err(|e| {
                ToolError::ExecutionError(format!(
                    "Unable to list resources for {}, {:?}",
                    system_name, e
                ))
            })
            .map(|lr| {
                let resource_list = lr
                    .resources
                    .into_iter()
                    .map(|r| format!("{} - {}, uri: ({})", system_name, r.name, r.uri))
                    .collect::<Vec<String>>()
                    .join("\n");

                vec![Content::text(resource_list)]
            })
    }

    async fn list_resources(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let system = params.get("system").and_then(|v| v.as_str());

        match system {
            Some(system_name) => {
                // Handle single system case
                self.list_resources_from_system(system_name).await
            }
            None => {
                // Handle all systems case using FuturesUnordered
                let mut futures = FuturesUnordered::new();

                // Create futures for each resource_capable_system
                for system_name in &self.resource_capable_systems {
                    futures.push(async move { self.list_resources_from_system(system_name).await });
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

    /// Dispatch a single tool call to the appropriate client
    #[instrument(skip(self, tool_call), fields(input, output))]
    pub async fn dispatch_tool_call(&self, tool_call: ToolCall) -> ToolResult<Vec<Content>> {
        let result = if tool_call.name == "platform__read_resource" {
            // Check if the tool is read_resource and handle it separately
            self.read_resource(tool_call.arguments.clone()).await
        } else if tool_call.name == "platform__list_resources" {
            self.list_resources(tool_call.arguments.clone()).await
        } else {
            // Else, dispatch tool call based on the prefix naming convention
            let client = self
                .get_client_for_tool(&tool_call.name)
                .ok_or_else(|| ToolError::NotFound(tool_call.name.clone()))?;

            let tool_name = tool_call
                .name
                .split("__")
                .nth(1)
                .ok_or_else(|| ToolError::NotFound(tool_call.name.clone()))?;

            let client_guard = client.lock().await;

            client_guard
                .call_tool(tool_name, tool_call.clone().arguments)
                .await
                .map(|result| result.content)
                .map_err(|e| ToolError::ExecutionError(e.to_string()))
        };

        debug!(
            "input" = serde_json::to_string(&tool_call).unwrap(),
            "output" = serde_json::to_string(&result).unwrap(),
        );

        result
    }
}
