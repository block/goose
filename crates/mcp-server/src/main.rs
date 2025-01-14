use anyhow::Result;
use mcp_core::content::Content;
use mcp_core::handler::ResourceError;
use mcp_core::{handler::ToolError, protocol::ServerCapabilities, resource::Resource, tool::Tool};
use mcp_server::router::{CapabilitiesBuilder, RouterService};
use mcp_server::{ByteTransport, Router, Server};
use serde_json::Value;
use std::{future::Future, pin::Pin, sync::Arc};
use tokio::{
    io::{stdin, stdout},
    sync::Mutex,
};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{self, EnvFilter};

// A simple counter service that demonstrates the Router trait
#[derive(Clone)]
struct CounterRouter {
    counter: Arc<Mutex<i32>>,
}

impl CounterRouter {
    fn new() -> Self {
        Self {
            counter: Arc::new(Mutex::new(0)),
        }
    }

    async fn increment(&self) -> Result<i32, ToolError> {
        let mut counter = self.counter.lock().await;
        *counter += 1;
        Ok(*counter)
    }

    async fn decrement(&self) -> Result<i32, ToolError> {
        let mut counter = self.counter.lock().await;
        *counter -= 1;
        Ok(*counter)
    }

    async fn get_value(&self) -> Result<i32, ToolError> {
        let counter = self.counter.lock().await;
        Ok(*counter)
    }

    fn _create_resource_text(&self, uri: &str, name: &str) -> Resource {
        Resource::new(uri, Some("text/plain".to_string()), Some(name.to_string())).unwrap()
    }
}

impl Router for CounterRouter {
    fn name(&self) -> String {
        "counter".to_string()
    }

    fn instructions(&self) -> String {
        "This server provides a counter tool that can increment and decrement values. The counter starts at 0 and can be modified using the 'increment' and 'decrement' tools. Use 'get_value' to check the current count.".to_string()
    }

    fn capabilities(&self) -> ServerCapabilities {
        CapabilitiesBuilder::new()
            .with_tools(false)
            .with_resources(false, false)
            .build()
    }

    fn list_tools(&self) -> Vec<Tool> {
        vec![
            Tool::new(
                "increment".to_string(),
                "Increment the counter by 1".to_string(),
                serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            ),
            Tool::new(
                "decrement".to_string(),
                "Decrement the counter by 1".to_string(),
                serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            ),
            Tool::new(
                "get_value".to_string(),
                "Get the current counter value".to_string(),
                serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            ),
        ]
    }

    fn call_tool(
        &self,
        tool_name: &str,
        _arguments: Value,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<Content>, ToolError>> + Send + 'static>> {
        let this = self.clone();
        let tool_name = tool_name.to_string();

        Box::pin(async move {
            match tool_name.as_str() {
                "increment" => {
                    let value = this.increment().await?;
                    Ok(vec![Content::text(value.to_string())])
                }
                "decrement" => {
                    let value = this.decrement().await?;
                    Ok(vec![Content::text(value.to_string())])
                }
                "get_value" => {
                    let value = this.get_value().await?;
                    Ok(vec![Content::text(value.to_string())])
                }
                _ => Err(ToolError::NotFound(format!("Tool {} not found", tool_name))),
            }
        })
    }

    fn list_resources(&self) -> Vec<Resource> {
        vec![
            self._create_resource_text("str:////Users/to/some/path/", "cwd"),
            self._create_resource_text("memo://insights", "memo-name"),
        ]
    }

    fn read_resource(
        &self,
        uri: &str,
    ) -> Pin<Box<dyn Future<Output = Result<String, ResourceError>> + Send + 'static>> {
        let uri = uri.to_string();
        Box::pin(async move {
            match uri.as_str() {
                "str:////Users/to/some/path/" => {
                    let cwd = "/Users/to/some/path/";
                    Ok(cwd.to_string())
                }
                "memo://insights" => {
                    let memo =
                        "Business Intelligence Memo\n\nAnalysis has revealed 5 key insights ...";
                    Ok(memo.to_string())
                }
                _ => Err(ResourceError::NotFound(format!(
                    "Resource {} not found",
                    uri
                ))),
            }
        })
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
    let router = RouterService(CounterRouter::new());

    // Create and run the server
    let server = Server::new(router);
    let transport = ByteTransport::new(stdin(), stdout());

    tracing::info!("Server initialized and ready to handle requests");
    Ok(server.run(transport).await?)
}
