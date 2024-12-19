use futures::TryFutureExt;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::errors::{AgentError, AgentResult};
use crate::providers::base::{Provider, ProviderUsage};
use mcp_client::client::McpClient;
use mcp_core::resource::ResourceContents;
use mcp_core::{Content, Resource, Tool, ToolCall};

/// McpAgent manages multiple MCP clients and dispatches requests to appropriate servers.
pub struct McpAgent {
    // Using Box<dyn McpClient> to allow for different implementations of the McpClient trait
    clients: HashMap<String, Arc<Mutex<Box<dyn McpClient + Send>>>>,
    provider: Box<dyn Provider>,
    provider_usage: Mutex<Vec<ProviderUsage>>,
}

impl McpAgent {
    /// Create new McpAgent with specified provider
    pub fn new(provider: Box<dyn Provider>) -> Self {
        Self {
            clients: HashMap::new(),
            provider,
            provider_usage: Mutex::new(Vec::new()),
        }
    }

    /// Get the context limit from the provider's configuration
    fn get_context_limit(&self) -> usize {
        self.provider.get_model_config().context_limit()
    }

    /// Add a named McpClient implementation to the agent
    pub fn add_mcp_client(&mut self, name: String, mcp_client: Box<dyn McpClient + Send>) {
        //TODO: initialize the client here and verify we have connectivity before
        //inserting into the map, probably return an error too
        self.clients.insert(name, Arc::new(Mutex::new(mcp_client)));
    }

    /// Get all tools from all servers, and prefix with the configured name
    async fn get_prefixed_tools(&mut self) -> Vec<Tool> {
        let results =
            futures::future::join_all(self.clients.iter_mut().map(|(name, client)| async move {
                let name = name.clone();
                let mut client_guard = client.lock().await;
                match client_guard.list_tools().await {
                    Ok(tools) => (name, Ok(tools)),
                    Err(e) => (name, Err(e)),
                }
            }))
            .await;

        //TODO: do something with _errors
        let (tools, _errors): (Vec<_>, Vec<_>) =
            results.into_iter().partition(|(_, result)| result.is_ok());

        tools
            .into_iter()
            .flat_map(|(name, result)| {
                result.unwrap().tools.into_iter().map(move |t| {
                    Tool::new(
                        format!("{}__{}", name, t.name),
                        &t.description,
                        t.input_schema,
                    )
                })
            })
            .collect()
    }

    /// Find and return a reference to the appropriate client for a tool call
    fn get_client_for_tool(
        &self,
        prefixed_name: &str,
    ) -> Option<Arc<Mutex<Box<dyn McpClient + Send>>>> {
        prefixed_name
            .split_once("__")
            .and_then(|(client_name, _)| self.clients.get(client_name))
            .map(Arc::clone)
    }

}
