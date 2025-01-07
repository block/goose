use rust_decimal_macros::dec;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::system::{SystemConfig, SystemError, SystemInfo, SystemResult};
use crate::prompt_template::load_prompt_file;
use crate::providers::base::{Provider, ProviderUsage};
use mcp_client::client::{ClientCapabilities, ClientInfo, McpClient};
use mcp_client::transport::{SseTransport, StdioTransport, Transport};
use mcp_core::{Content, Resource, Tool, ToolCall, ToolError, ToolResult};

/// Manages MCP clients and their interactions
pub struct Capabilities {
    clients: HashMap<String, Arc<Mutex<McpClient>>>,
    instructions: HashMap<String, String>,
    provider: Box<dyn Provider>,
    provider_usage: Mutex<Vec<ProviderUsage>>,
}

impl Capabilities {
    /// Create a new Capabilities with the specified provider
    pub fn new(provider: Box<dyn Provider>) -> Self {
        Self {
            clients: HashMap::new(),
            instructions: HashMap::new(),
            provider,
            provider_usage: Mutex::new(Vec::new()),
        }
    }

    /// Add a new MCP system based on the provided client type
    // TODO IMPORTANT need to ensure this times out if the system command is broken!
    pub async fn add_system(&mut self, config: SystemConfig) -> SystemResult<()> {
        let client: McpClient = match config {
            SystemConfig::Sse { ref uri } => {
                let transport = SseTransport::new(uri);
                McpClient::new(transport.start().await?)
            }
            SystemConfig::Stdio { ref cmd, ref args } => {
                let transport = StdioTransport::new(cmd, args.to_vec());
                McpClient::new(transport.start().await?)
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

        // Store the client
        self.clients.insert(
            init_result.server_info.name.clone(),
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
                    if e.cost.is_none() || usage.cost.is_none() {
                        e.cost = None; // Pricing is not available for all models
                    } else {
                        e.cost = Some(e.cost.unwrap_or(dec!(0)) + usage.cost.unwrap_or(dec!(0)));
                    }
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
            let client_tools = client_guard.list_tools().await?;

            for tool in client_tools.tools {
                tools.push(Tool::new(
                    format!("{}__{}", name, tool.name),
                    &tool.description,
                    tool.input_schema,
                ));
            }
        }
        Ok(tools)
    }

    /// Get client resources and their contents
    // TODO this data model needs flattening
    pub async fn get_resources(
        &self,
    ) -> SystemResult<HashMap<String, HashMap<String, (Resource, String)>>> {
        let mut client_resource_content = HashMap::new();
        for (name, client) in &self.clients {
            let client_guard = client.lock().await;
            let resources = client_guard.list_resources().await?;

            let mut resource_content = HashMap::new();
            for resource in resources.resources {
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
                        resource_content.insert(uri, (resource.clone(), content_str));
                    }
                }
            }
            client_resource_content.insert(name.clone(), resource_content);
        }
        Ok(client_resource_content)
    }

    /// Get the system prompt including client instructions
    pub async fn get_system_prompt(&self) -> String {
        let mut context = HashMap::new();
        let systems_info: Vec<SystemInfo> = self
            .clients
            .keys()
            .map(|name| {
                let instructions = self.instructions.get(name).cloned().unwrap_or_default();
                SystemInfo::new(name, "", &instructions)
            })
            .collect();

        context.insert("systems", systems_info);
        load_prompt_file("system.md", &context).expect("Prompt should render")
    }

    /// Find and return a reference to the appropriate client for a tool call
    fn get_client_for_tool(&self, prefixed_name: &str) -> Option<Arc<Mutex<McpClient>>> {
        prefixed_name
            .split_once("__")
            .and_then(|(client_name, _)| self.clients.get(client_name))
            .map(Arc::clone)
    }

    /// Dispatch a single tool call to the appropriate client
    pub async fn dispatch_tool_call(&self, tool_call: ToolCall) -> ToolResult<Vec<Content>> {
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
            .call_tool(tool_name, tool_call.arguments)
            .await
            .map(|result| result.content)
            .map_err(|e| ToolError::ExecutionError(e.to_string()))
    }
}
