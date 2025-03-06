use anyhow::Result;
use chrono::{DateTime, TimeZone, Utc};
use futures::stream::{FuturesUnordered, StreamExt};
use mcp_client::McpService;
use mcp_core::protocol::GetPromptResult;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{debug, instrument};

use super::extension::{ExtensionConfig, ExtensionError, ExtensionInfo, ExtensionResult};
use crate::config::Config;
use crate::prompt_template;
use crate::providers::base::{Provider, ProviderUsage};
use mcp_client::client::{ClientCapabilities, ClientInfo, McpClient, McpClientTrait};
use mcp_client::transport::{SseTransport, StdioTransport, Transport};
use mcp_core::{prompt::Prompt, Content, Tool, ToolCall, ToolError, ToolResult};
use serde_json::Value;

// By default, we set it to Jan 1, 2020 if the resource does not have a timestamp
// This is to ensure that the resource is considered less important than resources with a more recent timestamp
static DEFAULT_TIMESTAMP: LazyLock<DateTime<Utc>> =
    LazyLock::new(|| Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap());

type McpClientBox = Arc<Mutex<Box<dyn McpClientTrait>>>;

/// Manages MCP clients and their interactions
pub struct Capabilities {
    clients: HashMap<String, McpClientBox>,
    instructions: HashMap<String, String>,
    resource_capable_extensions: HashSet<String>,
    provider: Arc<Box<dyn Provider>>,
    provider_usage: Mutex<Vec<ProviderUsage>>,
    system_prompt_override: Option<String>,
    system_prompt_extensions: Vec<String>,
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

impl Capabilities {
    /// Create a new Capabilities with the specified provider
    pub fn new(provider: Box<dyn Provider>) -> Self {
        Self {
            clients: HashMap::new(),
            instructions: HashMap::new(),
            resource_capable_extensions: HashSet::new(),
            provider: Arc::new(provider),
            provider_usage: Mutex::new(Vec::new()),
            system_prompt_override: None,
            system_prompt_extensions: Vec::new(),
        }
    }

    pub fn supports_resources(&self) -> bool {
        !self.resource_capable_extensions.is_empty()
    }

    /// Add a new MCP extension based on the provided client type
    // TODO IMPORTANT need to ensure this times out if the extension command is broken!
    pub async fn add_extension(&mut self, config: ExtensionConfig) -> ExtensionResult<()> {
        let mut client: Box<dyn McpClientTrait> = match &config {
            ExtensionConfig::Sse {
                uri, envs, timeout, ..
            } => {
                let transport = SseTransport::new(uri, envs.get_env());
                let handle = transport.start().await?;
                let service = McpService::with_timeout(
                    handle,
                    Duration::from_secs(
                        timeout.unwrap_or(crate::config::DEFAULT_EXTENSION_TIMEOUT),
                    ),
                );
                Box::new(McpClient::new(service))
            }
            ExtensionConfig::Stdio {
                cmd,
                args,
                envs,
                timeout,
                ..
            } => {
                let transport = StdioTransport::new(cmd, args.to_vec(), envs.get_env());
                let handle = transport.start().await?;
                let service = McpService::with_timeout(
                    handle,
                    Duration::from_secs(
                        timeout.unwrap_or(crate::config::DEFAULT_EXTENSION_TIMEOUT),
                    ),
                );
                Box::new(McpClient::new(service))
            }
            ExtensionConfig::Builtin { name, timeout } => {
                // For builtin extensions, we run the current executable with mcp and extension name
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
                let service = McpService::with_timeout(
                    handle,
                    Duration::from_secs(
                        timeout.unwrap_or(crate::config::DEFAULT_EXTENSION_TIMEOUT),
                    ),
                );
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
            .map_err(|e| ExtensionError::Initialization(config.clone(), e))?;

        let sanitized_name = normalize(config.name().to_string());

        // Store instructions if provided
        if let Some(instructions) = init_result.instructions {
            self.instructions
                .insert(sanitized_name.clone(), instructions);
        }

        // if the server is capable if resources we track it
        if init_result.capabilities.resources.is_some() {
            self.resource_capable_extensions
                .insert(sanitized_name.clone());
        }

        // Store the client using the provided name
        self.clients
            .insert(sanitized_name.clone(), Arc::new(Mutex::new(client)));

        Ok(())
    }

    /// Add a system prompt extension
    pub fn add_system_prompt_extension(&mut self, extension: String) {
        self.system_prompt_extensions.push(extension);
    }

    /// Override the system prompt with custom text
    pub fn set_system_prompt_override(&mut self, template: String) {
        self.system_prompt_override = Some(template);
    }

    /// Get a reference to the provider
    pub fn provider(&self) -> Arc<Box<dyn Provider>> {
        Arc::clone(&self.provider)
    }

    /// Record provider usage
    // TODO consider moving this off to the provider or as a form of logging
    pub async fn record_usage(&self, usage: ProviderUsage) {
        self.provider_usage.lock().await.push(usage);
    }

    /// Get aggregated usage statistics
    pub async fn remove_extension(&mut self, name: &str) -> ExtensionResult<()> {
        let sanitized_name = normalize(name.to_string());

        self.clients.remove(&sanitized_name);
        self.instructions.remove(&sanitized_name);
        self.resource_capable_extensions.remove(&sanitized_name);
        Ok(())
    }

    pub async fn list_extensions(&self) -> ExtensionResult<Vec<String>> {
        Ok(self.clients.keys().cloned().collect())
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
    pub async fn get_prefixed_tools(&mut self) -> ExtensionResult<Vec<Tool>> {
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
    pub async fn get_resources(&self) -> ExtensionResult<Vec<ResourceItem>> {
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

    /// Get the extension prompt including client instructions
    pub async fn get_system_prompt(&self) -> String {
        let mut context: HashMap<&str, Value> = HashMap::new();

        let extensions_info: Vec<ExtensionInfo> = self
            .clients
            .keys()
            .map(|name| {
                let instructions = self.instructions.get(name).cloned().unwrap_or_default();
                let has_resources = self.resource_capable_extensions.contains(name);
                ExtensionInfo::new(name, &instructions, has_resources)
            })
            .collect();
        context.insert("extensions", serde_json::to_value(extensions_info).unwrap());

        let current_date_time = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        context.insert("current_date_time", Value::String(current_date_time));

        // Conditionally load the override prompt or the global system prompt
        let base_prompt = if let Some(override_prompt) = &self.system_prompt_override {
            prompt_template::render_inline_once(override_prompt, &context)
                .expect("Prompt should render")
        } else {
            prompt_template::render_global_file("system.md", &context)
                .expect("Prompt should render")
        };

        let mut system_prompt_extensions = self.system_prompt_extensions.clone();
        let config = Config::global();
        let goose_mode = config.get("GOOSE_MODE").unwrap_or("auto".to_string());
        if goose_mode == "chat" {
            system_prompt_extensions.push(
                "Right now you are in the chat only mode, no access to any tool use and system."
                    .to_string(),
            );
        } else {
            system_prompt_extensions
                .push("Right now you are *NOT* in the chat only mode and have access to tool use and system.".to_string());
        }

        if system_prompt_extensions.is_empty() {
            base_prompt
        } else {
            format!(
                "{}\n\n# Additional Instructions:\n\n{}",
                base_prompt,
                system_prompt_extensions.join("\n\n")
            )
        }
    }

    /// Find and return a reference to the appropriate client for a tool call
    fn get_client_for_tool(&self, prefixed_name: &str) -> Option<(&str, McpClientBox)> {
        self.clients
            .iter()
            .find(|(key, _)| prefixed_name.starts_with(*key))
            .map(|(name, client)| (name.as_str(), Arc::clone(client)))
    }

    // Function that gets executed for read_resource tool
    async fn read_resource(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let uri = params
            .get("uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParameters("Missing 'uri' parameter".to_string()))?;

        let extension_name = params.get("extension_name").and_then(|v| v.as_str());

        // If extension name is provided, we can just look it up
        if extension_name.is_some() {
            let result = self
                .read_resource_from_extension(uri, extension_name.unwrap())
                .await?;
            return Ok(result);
        }

        // If extension name is not provided, we need to search for the resource across all extensions
        // Loop through each extension and try to read the resource, don't raise an error if the resource is not found
        // TODO: do we want to find if a provided uri is in multiple extensions?
        // currently it will return the first match and skip any others
        for extension_name in self.resource_capable_extensions.iter() {
            let result = self.read_resource_from_extension(uri, extension_name).await;
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

    async fn list_resources_from_extension(
        &self,
        extension_name: &str,
    ) -> Result<Vec<Content>, ToolError> {
        let client = self.clients.get(extension_name).ok_or_else(|| {
            ToolError::InvalidParameters(format!("Extension {} is not valid", extension_name))
        })?;

        let client_guard = client.lock().await;
        client_guard
            .list_resources(None)
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

    async fn list_resources(&self, params: Value) -> Result<Vec<Content>, ToolError> {
        let extension = params.get("extension").and_then(|v| v.as_str());

        match extension {
            Some(extension_name) => {
                // Handle single extension case
                self.list_resources_from_extension(extension_name).await
            }
            None => {
                // Handle all extensions case using FuturesUnordered
                let mut futures = FuturesUnordered::new();

                // Create futures for each resource_capable_extension
                for extension_name in &self.resource_capable_extensions {
                    futures.push(async move {
                        self.list_resources_from_extension(extension_name).await
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
            let (client_name, client) = self
                .get_client_for_tool(&tool_call.name)
                .ok_or_else(|| ToolError::NotFound(tool_call.name.clone()))?;

            // rsplit returns the iterator in reverse, tool_name is then at 0
            let tool_name = tool_call
                .name
                .strip_prefix(client_name)
                .and_then(|s| s.strip_prefix("__"))
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

    pub async fn list_prompts_from_extension(
        &self,
        extension_name: &str,
    ) -> Result<Vec<Prompt>, ToolError> {
        let client = self.clients.get(extension_name).ok_or_else(|| {
            ToolError::InvalidParameters(format!("Extension {} is not valid", extension_name))
        })?;

        let client_guard = client.lock().await;
        client_guard
            .list_prompts(None)
            .await
            .map_err(|e| {
                ToolError::ExecutionError(format!(
                    "Unable to list prompts for {}, {:?}",
                    extension_name, e
                ))
            })
            .map(|lp| lp.prompts)
    }

    pub async fn list_prompts(&self) -> Result<HashMap<String, Vec<Prompt>>, ToolError> {
        let mut futures = FuturesUnordered::new();

        for extension_name in self.clients.keys() {
            futures.push(async move {
                (
                    extension_name,
                    self.list_prompts_from_extension(extension_name).await,
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
    ) -> Result<GetPromptResult> {
        let client = self
            .clients
            .get(extension_name)
            .ok_or_else(|| anyhow::anyhow!("Extension {} not found", extension_name))?;

        let client_guard = client.lock().await;
        client_guard
            .get_prompt(name, arguments)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get prompt: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::Message;
    use crate::model::ModelConfig;
    use crate::providers::base::{Provider, ProviderMetadata, ProviderUsage, Usage};
    use crate::providers::errors::ProviderError;
    use mcp_client::client::Error;
    use mcp_client::client::McpClientTrait;
    use mcp_core::protocol::{
        CallToolResult, GetPromptResult, InitializeResult, ListPromptsResult, ListResourcesResult,
        ListToolsResult, ReadResourceResult,
    };
    use serde_json::json;

    // Mock Provider implementation for testing
    #[derive(Clone)]
    struct MockProvider {
        model_config: ModelConfig,
    }

    #[async_trait::async_trait]
    impl Provider for MockProvider {
        fn metadata() -> ProviderMetadata {
            ProviderMetadata::empty()
        }

        fn get_model_config(&self) -> ModelConfig {
            self.model_config.clone()
        }

        async fn complete(
            &self,
            _system: &str,
            _messages: &[Message],
            _tools: &[Tool],
        ) -> anyhow::Result<(Message, ProviderUsage), ProviderError> {
            Ok((
                Message::assistant().with_text("Mock response"),
                ProviderUsage::new("mock".to_string(), Usage::default()),
            ))
        }
    }

    struct MockClient {}

    #[async_trait::async_trait]
    impl McpClientTrait for MockClient {
        async fn initialize(
            &mut self,
            _info: ClientInfo,
            _capabilities: ClientCapabilities,
        ) -> Result<InitializeResult, Error> {
            Err(Error::NotInitialized)
        }

        async fn list_resources(
            &self,
            _next_cursor: Option<String>,
        ) -> Result<ListResourcesResult, Error> {
            Err(Error::NotInitialized)
        }

        async fn read_resource(&self, _uri: &str) -> Result<ReadResourceResult, Error> {
            Err(Error::NotInitialized)
        }

        async fn list_tools(&self, _next_cursor: Option<String>) -> Result<ListToolsResult, Error> {
            Err(Error::NotInitialized)
        }

        async fn call_tool(&self, name: &str, _arguments: Value) -> Result<CallToolResult, Error> {
            match name {
                "tool" | "test__tool" => Ok(CallToolResult {
                    content: vec![],
                    is_error: None,
                }),
                _ => Err(Error::NotInitialized),
            }
        }

        async fn list_prompts(
            &self,
            _next_cursor: Option<String>,
        ) -> Result<ListPromptsResult, Error> {
            Err(Error::NotInitialized)
        }

        async fn get_prompt(
            &self,
            _name: &str,
            _arguments: Value,
        ) -> Result<GetPromptResult, Error> {
            Err(Error::NotInitialized)
        }
    }

    #[test]
    fn test_get_client_for_tool() {
        let mock_model_config =
            ModelConfig::new("test-model".to_string()).with_context_limit(200_000.into());

        let mut capabilities = Capabilities::new(Box::new(MockProvider {
            model_config: mock_model_config,
        }));

        // Add some mock clients
        capabilities.clients.insert(
            normalize("test_client".to_string()),
            Arc::new(Mutex::new(Box::new(MockClient {}))),
        );

        capabilities.clients.insert(
            normalize("__client".to_string()),
            Arc::new(Mutex::new(Box::new(MockClient {}))),
        );

        capabilities.clients.insert(
            normalize("__cli__ent__".to_string()),
            Arc::new(Mutex::new(Box::new(MockClient {}))),
        );

        capabilities.clients.insert(
            normalize("client 🚀".to_string()),
            Arc::new(Mutex::new(Box::new(MockClient {}))),
        );

        // Test basic case
        assert!(capabilities
            .get_client_for_tool("test_client__tool")
            .is_some());

        // Test leading underscores
        assert!(capabilities.get_client_for_tool("__client__tool").is_some());

        // Test multiple underscores in client name, and ending with __
        assert!(capabilities
            .get_client_for_tool("__cli__ent____tool")
            .is_some());

        // Test unicode in tool name, "client 🚀" should become "client_"
        assert!(capabilities.get_client_for_tool("client___tool").is_some());
    }

    #[tokio::test]
    async fn test_dispatch_tool_call() {
        // test that dispatch_tool_call parses out the sanitized name correctly, and extracts
        // tool_names
        let mock_model_config =
            ModelConfig::new("test-model".to_string()).with_context_limit(200_000.into());

        let mut capabilities = Capabilities::new(Box::new(MockProvider {
            model_config: mock_model_config,
        }));

        // Add some mock clients
        capabilities.clients.insert(
            normalize("test_client".to_string()),
            Arc::new(Mutex::new(Box::new(MockClient {}))),
        );

        capabilities.clients.insert(
            normalize("__cli__ent__".to_string()),
            Arc::new(Mutex::new(Box::new(MockClient {}))),
        );

        capabilities.clients.insert(
            normalize("client 🚀".to_string()),
            Arc::new(Mutex::new(Box::new(MockClient {}))),
        );

        // verify a normal tool call
        let tool_call = ToolCall {
            name: "test_client__tool".to_string(),
            arguments: json!({}),
        };

        let result = capabilities.dispatch_tool_call(tool_call).await;
        assert!(result.is_ok());

        let tool_call = ToolCall {
            name: "test_client__test__tool".to_string(),
            arguments: json!({}),
        };

        let result = capabilities.dispatch_tool_call(tool_call).await;
        assert!(result.is_ok());

        // verify a multiple underscores dispatch
        let tool_call = ToolCall {
            name: "__cli__ent____tool".to_string(),
            arguments: json!({}),
        };

        let result = capabilities.dispatch_tool_call(tool_call).await;
        assert!(result.is_ok());

        // Test unicode in tool name, "client 🚀" should become "client_"
        let tool_call = ToolCall {
            name: "client___tool".to_string(),
            arguments: json!({}),
        };

        let result = capabilities.dispatch_tool_call(tool_call).await;
        assert!(result.is_ok());

        let tool_call = ToolCall {
            name: "client___test__tool".to_string(),
            arguments: json!({}),
        };

        let result = capabilities.dispatch_tool_call(tool_call).await;
        assert!(result.is_ok());

        // this should error out, specifically for an ToolError::ExecutionError
        let invalid_tool_call = ToolCall {
            name: "client___tools".to_string(),
            arguments: json!({}),
        };

        let result = capabilities.dispatch_tool_call(invalid_tool_call).await;
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

        let result = capabilities.dispatch_tool_call(invalid_tool_call).await;
        assert!(matches!(result.err().unwrap(), ToolError::NotFound(_)));
    }
}
