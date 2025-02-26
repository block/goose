use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

type PromptFuture = Pin<Box<dyn Future<Output = Result<String, PromptError>> + Send + 'static>>;

use mcp_core::{
    content::Content,
    handler::{PromptError, ResourceError, ToolError},
    prompt::{Prompt, PromptMessage, PromptMessageRole},
    protocol::{
        CallToolResult, GetPromptResult, Implementation, InitializeResult, JsonRpcRequest,
        JsonRpcResponse, ListPromptsResult, ListResourcesResult, ListToolsResult,
        PromptsCapability, ReadResourceResult, ResourcesCapability, ServerCapabilities,
        ToolsCapability,
    },
    ResourceContents,
};
use serde_json::Value;
use tower_service::Service;

use crate::{BoxError, RouterError};

/// Builder for configuring and constructing capabilities
pub struct CapabilitiesBuilder {
    tools: Option<ToolsCapability>,
    prompts: Option<PromptsCapability>,
    resources: Option<ResourcesCapability>,
}

impl Default for CapabilitiesBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilitiesBuilder {
    pub fn new() -> Self {
        Self {
            tools: None,
            prompts: None,
            resources: None,
        }
    }

    /// Add multiple tools to the router
    pub fn with_tools(mut self, list_changed: bool) -> Self {
        self.tools = Some(ToolsCapability {
            list_changed: Some(list_changed),
        });
        self
    }

    /// Enable prompts capability
    pub fn with_prompts(mut self, list_changed: bool) -> Self {
        self.prompts = Some(PromptsCapability {
            list_changed: Some(list_changed),
        });
        self
    }

    /// Enable resources capability
    pub fn with_resources(mut self, subscribe: bool, list_changed: bool) -> Self {
        self.resources = Some(ResourcesCapability {
            subscribe: Some(subscribe),
            list_changed: Some(list_changed),
        });
        self
    }

    /// Build the router with automatic capability inference
    pub fn build(self) -> ServerCapabilities {
        // Create capabilities based on what's configured
        ServerCapabilities {
            tools: self.tools,
            prompts: self.prompts,
            resources: self.resources,
        }
    }
}

pub trait Router: Send + Sync + 'static {
    fn name(&self) -> String;
    // in the protocol, instructions are optional but we make it required
    fn instructions(&self) -> String;
    fn capabilities(&self) -> ServerCapabilities;
    fn list_tools(&self) -> Vec<mcp_core::tool::Tool>;
    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>>;
    fn list_resources(&self) -> Vec<mcp_core::resource::Resource>;
    fn read_resource(
        &self,
        uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>>;
    fn list_prompts(&self) -> Option<Vec<Prompt>> {
        None
    }
    fn get_prompt(&self, _prompt_name: &str) -> Option<PromptFuture> {
        None
    }

    // Helper method to create base response
    fn create_response(&self, id: Option<u64>) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: None,
        }
    }

    fn handle_initialize(
        &self,
        req: JsonRpcRequest,
    ) -> impl Future<Output = Result<JsonRpcResponse, RouterError>> + Send {
        async move {
            let result = InitializeResult {
                protocol_version: "2024-11-05".to_string(),
                capabilities: self.capabilities().clone(),
                server_info: Implementation {
                    name: self.name(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                },
                instructions: Some(self.instructions()),
            };

            let mut response = self.create_response(req.id);
            response.result =
                Some(serde_json::to_value(result).map_err(|e| {
                    RouterError::Internal(format!("JSON serialization error: {}", e))
                })?);

            Ok(response)
        }
    }

    fn handle_tools_list(
        &self,
        req: JsonRpcRequest,
    ) -> impl Future<Output = Result<JsonRpcResponse, RouterError>> + Send {
        async move {
            let tools = self.list_tools();

            let result = ListToolsResult {
                tools,
                next_cursor: None,
            };
            let mut response = self.create_response(req.id);
            response.result =
                Some(serde_json::to_value(result).map_err(|e| {
                    RouterError::Internal(format!("JSON serialization error: {}", e))
                })?);

            Ok(response)
        }
    }

    fn handle_tools_call(
        &self,
        req: JsonRpcRequest,
    ) -> impl Future<Output = Result<JsonRpcResponse, RouterError>> + Send {
        async move {
            let params = req
                .params
                .ok_or_else(|| RouterError::InvalidParams("Missing parameters".into()))?;

            let name = params
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| RouterError::InvalidParams("Missing tool name".into()))?;

            let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);

            let result = match self.call_tool(name, arguments).await {
                Ok(result) => CallToolResult {
                    content: result,
                    is_error: None,
                },
                Err(err) => CallToolResult {
                    content: vec![Content::text(err.to_string())],
                    is_error: Some(true),
                },
            };

            let mut response = self.create_response(req.id);
            response.result =
                Some(serde_json::to_value(result).map_err(|e| {
                    RouterError::Internal(format!("JSON serialization error: {}", e))
                })?);

            Ok(response)
        }
    }

    fn handle_resources_list(
        &self,
        req: JsonRpcRequest,
    ) -> impl Future<Output = Result<JsonRpcResponse, RouterError>> + Send {
        async move {
            let resources = self.list_resources();

            let result = ListResourcesResult {
                resources,
                next_cursor: None,
            };
            let mut response = self.create_response(req.id);
            response.result =
                Some(serde_json::to_value(result).map_err(|e| {
                    RouterError::Internal(format!("JSON serialization error: {}", e))
                })?);

            Ok(response)
        }
    }

    fn handle_resources_read(
        &self,
        req: JsonRpcRequest,
    ) -> impl Future<Output = Result<JsonRpcResponse, RouterError>> + Send {
        async move {
            let params = req
                .params
                .ok_or_else(|| RouterError::InvalidParams("Missing parameters".into()))?;

            let uri = params
                .get("uri")
                .and_then(Value::as_str)
                .ok_or_else(|| RouterError::InvalidParams("Missing resource URI".into()))?;

            let contents = self.read_resource(uri).await.map_err(RouterError::from)?;

            let result = ReadResourceResult {
                contents: vec![ResourceContents::TextResourceContents {
                    uri: uri.to_string(),
                    mime_type: Some("text/plain".to_string()),
                    text: contents,
                }],
            };

            let mut response = self.create_response(req.id);
            response.result =
                Some(serde_json::to_value(result).map_err(|e| {
                    RouterError::Internal(format!("JSON serialization error: {}", e))
                })?);

            Ok(response)
        }
    }

    fn handle_prompts_list(
        &self,
        req: JsonRpcRequest,
    ) -> impl Future<Output = Result<JsonRpcResponse, RouterError>> + Send {
        async move {
            let prompts = self.list_prompts().unwrap_or_default();

            let result = ListPromptsResult { prompts };

            let mut response = self.create_response(req.id);
            response.result =
                Some(serde_json::to_value(result).map_err(|e| {
                    RouterError::Internal(format!("JSON serialization error: {}", e))
                })?);

            Ok(response)
        }
    }

    fn handle_prompts_get(
        &self,
        req: JsonRpcRequest,
    ) -> impl Future<Output = Result<JsonRpcResponse, RouterError>> + Send {
        async move {
            // Validate and extract parameters
            let params = req
                .params
                .ok_or_else(|| RouterError::InvalidParams("Missing parameters".into()))?;

            // Extract "name" field
            let prompt_name = params
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| RouterError::InvalidParams("Missing prompt name".into()))?;

            // Extract "arguments" field
            let arguments = params
                .get("arguments")
                .and_then(Value::as_object)
                .ok_or_else(|| RouterError::InvalidParams("Missing arguments object".into()))?;

            // Fetch the prompt definition first
            let prompt = match self.list_prompts() {
                Some(prompts) => prompts
                    .into_iter()
                    .find(|p| p.name == prompt_name)
                    .ok_or_else(|| {
                        RouterError::PromptNotFound(format!("Prompt '{}' not found", prompt_name))
                    })?,
                None => return Err(RouterError::PromptNotFound("No prompts available".into())),
            };

            // Validate required arguments
            for arg in &prompt.arguments {
                if arg.required
                    && (!arguments.contains_key(&arg.name)
                        || arguments
                            .get(&arg.name)
                            .and_then(Value::as_str)
                            .is_none_or(str::is_empty))
                {
                    return Err(RouterError::InvalidParams(format!(
                        "Missing required argument: '{}'",
                        arg.name
                    )));
                }
            }

            // Now get the prompt content
            let description = self
                .get_prompt(prompt_name)
                .ok_or_else(|| RouterError::PromptNotFound("Prompt not found".into()))?
                .await
                .map_err(|e| RouterError::Internal(e.to_string()))?;

            // Validate prompt arguments for potential security issues from user text input
            // Checks:
            // - Prompt must be less than 10000 total characters
            // - Argument keys must be less than 1000 characters
            // - Argument values must be less than 1000 characters
            // - Dangerous patterns, eg "../", "//", "\\\\", "<script>", "{{", "}}"
            for (key, value) in arguments.iter() {
                // Check for empty or overly long keys/values
                if key.is_empty() || key.len() > 1000 {
                    return Err(RouterError::InvalidParams(
                        "Argument keys must be between 1-1000 characters".into(),
                    ));
                }

                let value_str = value.as_str().unwrap_or_default();
                if value_str.len() > 1000 {
                    return Err(RouterError::InvalidParams(
                        "Argument values must not exceed 1000 characters".into(),
                    ));
                }

                // Check for potentially dangerous patterns
                let dangerous_patterns = ["../", "//", "\\\\", "<script>", "{{", "}}"];
                for pattern in dangerous_patterns {
                    if key.contains(pattern) || value_str.contains(pattern) {
                        return Err(RouterError::InvalidParams(format!(
                            "Arguments contain potentially unsafe pattern: {}",
                            pattern
                        )));
                    }
                }
            }

            // Validate the prompt description length
            if description.len() > 10000 {
                return Err(RouterError::Internal(
                    "Prompt description exceeds maximum allowed length".into(),
                ));
            }

            // Create a mutable copy of the description to fill in arguments
            let mut description_filled = description.clone();

            // Replace each argument placeholder with its value from the arguments object
            for (key, value) in arguments {
                let placeholder = format!("{{{}}}", key);
                description_filled =
                    description_filled.replace(&placeholder, value.as_str().unwrap_or_default());
            }

            let messages = vec![PromptMessage::new_text(
                PromptMessageRole::User,
                description_filled.to_string(),
            )];

            // Build the final response
            let mut response = self.create_response(req.id);
            response.result = Some(
                serde_json::to_value(GetPromptResult {
                    description: Some(description_filled),
                    messages,
                })
                .map_err(|e| RouterError::Internal(format!("JSON serialization error: {}", e)))?,
            );
            Ok(response)
        }
    }
}

pub struct RouterService<T>(pub T);

impl<T> Service<JsonRpcRequest> for RouterService<T>
where
    T: Router + Clone + Send + Sync + 'static,
{
    type Response = JsonRpcResponse;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: JsonRpcRequest) -> Self::Future {
        let this = self.0.clone();

        Box::pin(async move {
            let result = match req.method.as_str() {
                "initialize" => this.handle_initialize(req).await,
                "tools/list" => this.handle_tools_list(req).await,
                "tools/call" => this.handle_tools_call(req).await,
                "resources/list" => this.handle_resources_list(req).await,
                "resources/read" => this.handle_resources_read(req).await,
                "prompts/list" => this.handle_prompts_list(req).await,
                "prompts/get" => this.handle_prompts_get(req).await,
                _ => {
                    let mut response = this.create_response(req.id);
                    response.error = Some(RouterError::MethodNotFound(req.method).into());
                    Ok(response)
                }
            };

            result.map_err(BoxError::from)
        })
    }
}
