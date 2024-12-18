use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};
use tokio::time::sleep;

use mcp_core::{
    handler::ToolHandler,
    protocol::{
        JsonRpcRequest, JsonRpcResponse, ServerCapabilities, InitializeResult, Implementation,
        PromptsCapability, ResourcesCapability, ToolsCapability, ListToolsResult, CallToolResult,
    },
    content::Content,
};
use tower_service::Service;
use serde_json::Value;

use crate::{RouterError, BoxError};

/// Builder for configuring and constructing a Router
pub struct RouterBuilder {
    tools: HashMap<String, Box<dyn ToolHandler>>,
    prompts: Option<PromptsCapability>,
    resources: Option<ResourcesCapability>,
}

impl RouterBuilder {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            prompts: None,
            resources: None,
        }
    }

    /// Add a tool to the router
    pub fn with_tool<T: ToolHandler>(mut self, tool: T) -> Self {
        self.tools.insert(tool.name().to_string(), Box::new(tool));
        self
    }

    /// Add multiple tools to the router
    pub fn with_tools<I>(mut self, tools: I) -> Self 
    where 
        I: IntoIterator,
        I::Item: ToolHandler,
    {
        for tool in tools {
            self.tools.insert(tool.name().to_string(), Box::new(tool));
        }
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
    pub fn build(self) -> Router {
        // Create capabilities based on what's configured
        let capabilities = ServerCapabilities {
            // Add tools capability if we have any tools
            tools: (!self.tools.is_empty()).then(|| ToolsCapability {
                list_changed: Some(false),
            }),
            // Add other capabilities that were explicitly set
            prompts: self.prompts,
            resources: self.resources,
        };

        Router {
            capabilities,
            tools: Arc::new(self.tools),
        }
    }
}


/// Router service that implements the JSON-RPC protocol
#[derive(Clone)]
pub struct Router {
    capabilities: ServerCapabilities,
    tools: Arc<HashMap<String, Box<dyn ToolHandler>>>,
}

impl Router {
    pub fn builder() -> RouterBuilder {
        RouterBuilder::new()
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

    async fn handle_initialize(&self, req: JsonRpcRequest) -> Result<JsonRpcResponse, RouterError> {
        let result = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: self.capabilities.clone(),
            server_info: Implementation {
                name: "mcp-server".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        let mut response = self.create_response(req.id);
        response.result = Some(serde_json::to_value(result)
            .map_err(|e| RouterError::Internal(format!("JSON serialization error: {}", e)))?);

        Ok(response)
    }

    async fn handle_tools_list(&self, req: JsonRpcRequest) -> Result<JsonRpcResponse, RouterError> {
        let tools = self.tools.values()
            .map(|tool| mcp_core::tool::Tool {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                input_schema: tool.schema(),
            })
            .collect();

        let result = ListToolsResult { tools };
        let mut response = self.create_response(req.id);
        response.result = Some(serde_json::to_value(result)
            .map_err(|e| RouterError::Internal(format!("JSON serialization error: {}", e)))?);

        Ok(response)
    }

    async fn handle_tools_call(&self, req: JsonRpcRequest) -> Result<JsonRpcResponse, RouterError> {
        let params = req.params.ok_or_else(|| RouterError::InvalidParams("Missing parameters".into()))?;

        let name = params.get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| RouterError::InvalidParams("Missing tool name".into()))?;

        let arguments = params.get("arguments")
            .cloned()
            .unwrap_or(Value::Null);

        let tool = self.tools.get(name)
            .ok_or_else(|| RouterError::ToolNotFound(name.to_string()))?;

        let result = match tool.call(arguments).await {
            Ok(result) => CallToolResult {
                content: vec![Content::text(result.to_string())],
                is_error: false,
            },
            Err(err) => CallToolResult {
                content: vec![Content::text(err.to_string())],
                is_error: true,
            }
        };

        let mut response = self.create_response(req.id);
        response.result = Some(serde_json::to_value(result)
            .map_err(|e| RouterError::Internal(format!("JSON serialization error: {}", e)))?);

        Ok(response)
    }
}

impl Service<JsonRpcRequest> for Router {
    type Response = JsonRpcResponse;
    type Error = BoxError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: JsonRpcRequest) -> Self::Future {
        // Create owned copy of self for the async block
        // This is safe because Router contains Arc'd data
        let this = self.clone();

        Box::pin(async move {
            let result = match req.method.as_str() {
                "slow" => {
                    sleep(Duration::from_secs(60)).await;
                    let mut response = this.create_response(req.id);
                    response.result = Some(Value::String("This should never be seen".to_string()));
                    Ok(response)
                },
                "initialize" => this.handle_initialize(req).await,
                "tools/list" => this.handle_tools_list(req).await,
                "tools/call" => this.handle_tools_call(req).await,
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


#[cfg(test)]
mod tests {
    use super::*;
    use mcp_core::handler::{ToolError, ToolHandler};
    use async_trait::async_trait;

    struct TestTool;

    #[async_trait]
    impl ToolHandler for TestTool {
        fn name(&self) -> &'static str {
            "test"
        }

        fn description(&self) -> &'static str {
            "A test tool"
        }

        fn schema(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "echo": {
                        "type": "string"
                    }
                }
            })
        }

        async fn call(&self, params: Value) -> Result<Value, ToolError> {
            Ok(params)
        }
    }

    #[tokio::test]
    async fn test_router_builder() {
        let router = Router::builder()
            .with_tool(TestTool)
            .with_prompts(true)
            .with_resources(true, true)
            .build();

        assert!(router.capabilities.tools.is_some());
        assert!(router.capabilities.prompts.is_some());
        assert!(router.capabilities.resources.is_some());
        assert!(router.tools.contains_key("test"));
    }

    #[tokio::test]
    async fn test_tools_list() {
        let router = Router::builder()
            .with_tool(TestTool)
            .build();

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(1),
            method: "tools/list".to_string(),
            params: None,
        };

        let mut router_service = router;
        let response = router_service.call(req).await.unwrap();
        
        assert!(response.error.is_none());
        assert!(response.result.is_some());
    }

    #[tokio::test]
    async fn test_tools_call() {
        let router = Router::builder()
            .with_tool(TestTool)
            .build();

        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(1),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "name": "test",
                "arguments": {
                    "echo": "hello"
                }
            })),
        };

        let mut router_service = router;
        let response = router_service.call(req).await.unwrap();
        
        assert!(response.error.is_none());
        assert!(response.result.is_some());
    }
}
