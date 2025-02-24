mod proxy;

use anyhow::Result;
use mcp_core::{
    content::Content,
    handler::{PromptError, ResourceError, ToolError},
    prompt::Prompt,
    protocol::ServerCapabilities,
    resource::Resource,
    role::Role,
    tool::Tool,
};
use mcp_server::router::CapabilitiesBuilder;
use mcp_server::Router;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tracing::error;

use self::proxy::JetBrainsProxy;

pub struct JetBrainsRouter {
    tools: Arc<Mutex<Vec<Tool>>>,
    proxy: Arc<JetBrainsProxy>,
    instructions: String,
}

impl Default for JetBrainsRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl JetBrainsRouter {
    pub fn new() -> Self {
        let tools = Arc::new(Mutex::new(Vec::new()));
        let proxy = Arc::new(JetBrainsProxy::new());
        let instructions = "JetBrains IDE integration".to_string();

        // Initialize the proxy
        let proxy_clone = Arc::clone(&proxy);
        tokio::spawn(async move {
            if let Err(e) = proxy_clone.start().await {
                error!("Failed to start JetBrains proxy: {}", e);
            }
        });

        // Start the background task to update tools
        let tools_clone = Arc::clone(&tools);
        let proxy_clone = Arc::clone(&proxy);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                match proxy_clone.list_tools().await {
                    Ok(new_tools) => {
                        let mut tools = tools_clone.lock().await;
                        *tools = new_tools;
                    }
                    Err(e) => {
                        error!("Failed to update tools: {}", e);
                    }
                }
            }
        });

        Self {
            tools,
            proxy,
            instructions,
        }
    }

    async fn call_proxy_tool(
        &self,
        tool_name: String,
        arguments: Value,
    ) -> Result<Vec<Content>, ToolError> {
        let result = self
            .proxy
            .call_tool(&tool_name, arguments)
            .await
            .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

        // Create a success message for the assistant
        let mut contents = vec![
            Content::text(format!("Tool {} executed successfully", tool_name))
                .with_audience(vec![Role::Assistant]),
        ];

        // Add the tool's result contents
        contents.extend(result.content);

        Ok(contents)
    }

    async fn ensure_tools(&self) -> Result<(), ToolError> {
        let mut retry_count = 0;
        let max_retries = 50; // 5 second total wait time
        let retry_delay = Duration::from_millis(100);

        while retry_count < max_retries {
            let tools = self.tools.lock().await;
            if !tools.is_empty() {
                return Ok(());
            }
            drop(tools); // Release the lock before sleeping

            sleep(retry_delay).await;
            retry_count += 1;
        }

        Err(ToolError::ExecutionError("Failed to get tools list from IDE. Make sure the IDE is running and the plugin is installed.".to_string()))
    }
}

impl Router for JetBrainsRouter {
    fn name(&self) -> String {
        "jetbrains".to_string()
    }

    fn instructions(&self) -> String {
        self.instructions.clone()
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new().with_tools(true).build()
    }

    fn list_tools(&self) -> Vec<Tool> {
        // Use block_in_place to avoid blocking the runtime
        tokio::task::block_in_place(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                let tools = self.tools.lock().await;
                if tools.is_empty() {
                    drop(tools);
                    if let Err(e) = self.ensure_tools().await {
                        error!("Failed to ensure tools: {}", e);
                        vec![]
                    } else {
                        self.tools.lock().await.clone()
                    }
                } else {
                    tools.clone()
                }
            })
        })
    }

    fn call_tool(
        &self,
        tool_name: &str,
        arguments: Value,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>> {
        let this = self.clone();
        let tool_name = tool_name.to_string();
        Box::pin(async move {
            this.ensure_tools().await?;
            this.call_proxy_tool(tool_name, arguments).await
        })
    }

    fn list_resources(&self) -> Vec<Resource> {
        vec![]
    }

    fn read_resource(
        &self,
        _uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        Box::pin(async { Err(ResourceError::NotFound("Resource not found".into())) })
    }

    fn list_prompts(&self) -> Vec<Prompt> {
        vec![]
    }

    fn get_prompt(
        &self,
        prompt_name: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, PromptError>> + Send + 'static>> {
        let prompt_name = prompt_name.to_string();
        Box::pin(async move {
            Err(PromptError::NotFound(format!(
                "Prompt {} not found",
                prompt_name
            )))
        })
    }
}

impl Clone for JetBrainsRouter {
    fn clone(&self) -> Self {
        Self {
            tools: Arc::clone(&self.tools),
            proxy: Arc::clone(&self.proxy),
            instructions: self.instructions.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::OnceCell;

    static JETBRAINS_ROUTER: OnceCell<JetBrainsRouter> = OnceCell::const_new();

    async fn get_router() -> &'static JetBrainsRouter {
        JETBRAINS_ROUTER
            .get_or_init(|| async { JetBrainsRouter::new() })
            .await
    }

    #[tokio::test]
    async fn test_router_creation() {
        let router = get_router().await;
        assert_eq!(router.name(), "jetbrains");
        assert!(!router.instructions().is_empty());
    }

    #[tokio::test]
    async fn test_capabilities() {
        let router = get_router().await;
        let capabilities = router.capabilities();
        assert!(capabilities.tools.is_some());
    }
}
