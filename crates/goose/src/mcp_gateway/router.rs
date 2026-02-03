//! MCP Router
//!
//! Routes tool calls to appropriate MCP servers.

use super::errors::GatewayError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// MCP Server connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServerStatus {
    /// Server is connected and healthy
    Connected,
    /// Server is disconnected
    Disconnected,
    /// Server is unhealthy (failed health checks)
    Unhealthy,
    /// Server is initializing
    Initializing,
    /// Server connection is being retried
    Reconnecting,
}

/// Server endpoint type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEndpoint {
    /// Standard I/O transport
    Stdio {
        command: String,
        args: Vec<String>,
        env: Option<HashMap<String, String>>,
    },
    /// Server-Sent Events transport
    Sse { url: String, headers: Option<HashMap<String, String>> },
    /// WebSocket transport
    WebSocket { url: String, headers: Option<HashMap<String, String>> },
}

/// Server capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerCapabilities {
    /// Server supports tools
    pub tools: bool,
    /// Server supports resources
    pub resources: bool,
    /// Server supports prompts
    pub prompts: bool,
    /// Server supports sampling
    pub sampling: bool,
    /// Server supports logging
    pub logging: bool,
    /// Additional capabilities
    pub extensions: HashMap<String, bool>,
}

/// MCP Server connection information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConnection {
    /// Unique server identifier
    pub server_id: String,
    /// Human-readable server name
    pub name: String,
    /// Server endpoint configuration
    pub endpoint: ServerEndpoint,
    /// Current connection status
    pub status: ServerStatus,
    /// Server capabilities
    pub capabilities: ServerCapabilities,
    /// Health check interval in seconds
    #[serde(default = "default_health_check_secs")]
    pub health_check_interval_secs: u64,
    /// Last successful health check
    pub last_health_check: Option<DateTime<Utc>>,
    /// Server version
    pub version: Option<String>,
    /// Connection established at
    pub connected_at: Option<DateTime<Utc>>,
    /// Number of consecutive failures
    pub failure_count: u32,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Tool definition from MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: Option<String>,
    /// Input schema (JSON Schema)
    pub input_schema: serde_json::Value,
    /// Server that provides this tool
    pub server_id: String,
    /// Whether the tool requires confirmation
    pub requires_confirmation: bool,
    /// Tool tags for categorization
    pub tags: Vec<String>,
    /// Tool metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Tool registration in the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolRegistration {
    /// Tool name
    pub tool_name: String,
    /// Server that provides this tool
    pub server_id: String,
    /// Full tool definition
    pub definition: ToolDefinition,
    /// When the tool was registered
    pub registered_at: DateTime<Utc>,
    /// Call count
    pub call_count: u64,
    /// Average execution time in ms
    pub avg_execution_ms: f64,
}

/// Registry mapping tools to servers
pub struct ToolRegistry {
    tools: HashMap<String, ToolRegistration>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Register a tool
    pub fn register(&mut self, definition: ToolDefinition) {
        let registration = ToolRegistration {
            tool_name: definition.name.clone(),
            server_id: definition.server_id.clone(),
            definition,
            registered_at: Utc::now(),
            call_count: 0,
            avg_execution_ms: 0.0,
        };
        self.tools.insert(registration.tool_name.clone(), registration);
    }

    /// Unregister all tools from a server
    pub fn unregister_server(&mut self, server_id: &str) {
        self.tools.retain(|_, reg| reg.server_id != server_id);
    }

    /// Get server for a tool
    pub fn get_server_for_tool(&self, tool_name: &str) -> Option<&str> {
        self.tools.get(tool_name).map(|reg| reg.server_id.as_str())
    }

    /// Get tool definition
    pub fn get_tool(&self, tool_name: &str) -> Option<&ToolRegistration> {
        self.tools.get(tool_name)
    }

    /// Get all tools
    pub fn all_tools(&self) -> Vec<&ToolRegistration> {
        self.tools.values().collect()
    }

    /// Get tools by server
    pub fn tools_by_server(&self, server_id: &str) -> Vec<&ToolRegistration> {
        self.tools
            .values()
            .filter(|reg| reg.server_id == server_id)
            .collect()
    }

    /// Search tools by query
    pub fn search(&self, query: &str) -> Vec<&ToolRegistration> {
        let query_lower = query.to_lowercase();
        self.tools
            .values()
            .filter(|reg| {
                reg.tool_name.to_lowercase().contains(&query_lower)
                    || reg
                        .definition
                        .description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
                    || reg.definition.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// Update tool statistics
    pub fn record_call(&mut self, tool_name: &str, execution_ms: f64) {
        if let Some(reg) = self.tools.get_mut(tool_name) {
            let old_count = reg.call_count as f64;
            let new_count = old_count + 1.0;
            reg.avg_execution_ms = (reg.avg_execution_ms * old_count + execution_ms) / new_count;
            reg.call_count += 1;
        }
    }

    /// Get tool count
    pub fn count(&self) -> usize {
        self.tools.len()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Health report for all servers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// Report timestamp
    pub timestamp: DateTime<Utc>,
    /// Server statuses
    pub servers: HashMap<String, ServerHealthStatus>,
    /// Overall health
    pub overall_healthy: bool,
    /// Total tools available
    pub total_tools: usize,
}

/// Individual server health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerHealthStatus {
    /// Server ID
    pub server_id: String,
    /// Server name
    pub name: String,
    /// Current status
    pub status: ServerStatus,
    /// Last health check time
    pub last_check: Option<DateTime<Utc>>,
    /// Number of tools
    pub tool_count: usize,
    /// Error message if unhealthy
    pub error: Option<String>,
}

/// MCP Server configuration for registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Server ID (auto-generated if not provided)
    pub id: Option<String>,
    /// Server name
    pub name: String,
    /// Server endpoint
    pub endpoint: ServerEndpoint,
    /// Health check interval in seconds
    #[serde(default = "default_health_check_secs")]
    pub health_check_interval_secs: u64,
    /// Connection timeout in seconds
    #[serde(default = "default_connection_timeout_secs")]
    pub connection_timeout_secs: u64,
    /// Auto-reconnect on failure
    #[serde(default = "default_auto_reconnect")]
    pub auto_reconnect: bool,
    /// Max reconnection attempts
    #[serde(default = "default_max_reconnect_attempts")]
    pub max_reconnect_attempts: u32,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

fn default_health_check_secs() -> u64 {
    30
}

fn default_connection_timeout_secs() -> u64 {
    10
}

fn default_auto_reconnect() -> bool {
    true
}

fn default_max_reconnect_attempts() -> u32 {
    3
}

/// Routes tool calls to appropriate MCP servers
pub struct McpRouter {
    servers: RwLock<HashMap<String, McpServerConnection>>,
    tool_registry: RwLock<ToolRegistry>,
}

impl McpRouter {
    /// Create a new router
    pub fn new() -> Self {
        Self {
            servers: RwLock::new(HashMap::new()),
            tool_registry: RwLock::new(ToolRegistry::new()),
        }
    }

    /// Register a new MCP server
    pub async fn register_server(&self, config: McpServerConfig) -> Result<String, GatewayError> {
        let server_id = config
            .id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let connection = McpServerConnection {
            server_id: server_id.clone(),
            name: config.name,
            endpoint: config.endpoint,
            status: ServerStatus::Initializing,
            capabilities: ServerCapabilities::default(),
            health_check_interval_secs: config.health_check_interval_secs,
            last_health_check: None,
            version: None,
            connected_at: None,
            failure_count: 0,
            metadata: config.metadata,
        };

        let mut servers = self.servers.write().await;
        servers.insert(server_id.clone(), connection);

        tracing::info!(server_id = %server_id, "Registered MCP server");

        Ok(server_id)
    }

    /// Unregister a server
    pub async fn unregister_server(&self, server_id: &str) -> Result<(), GatewayError> {
        let mut servers = self.servers.write().await;
        servers.remove(server_id).ok_or_else(|| GatewayError::ServerNotAvailable {
            server_id: server_id.to_string(),
        })?;

        // Remove tools from registry
        let mut registry = self.tool_registry.write().await;
        registry.unregister_server(server_id);

        tracing::info!(server_id = %server_id, "Unregistered MCP server");

        Ok(())
    }

    /// Update server status
    pub async fn update_server_status(&self, server_id: &str, status: ServerStatus) {
        let mut servers = self.servers.write().await;
        if let Some(server) = servers.get_mut(server_id) {
            server.status = status;
            if status == ServerStatus::Connected {
                server.connected_at = Some(Utc::now());
                server.failure_count = 0;
            }
        }
    }

    /// Record server failure
    pub async fn record_server_failure(&self, server_id: &str, error: &str) {
        let mut servers = self.servers.write().await;
        if let Some(server) = servers.get_mut(server_id) {
            server.failure_count += 1;
            server.status = if server.failure_count >= 3 {
                ServerStatus::Unhealthy
            } else {
                ServerStatus::Reconnecting
            };
            tracing::warn!(
                server_id = %server_id,
                failure_count = server.failure_count,
                error = %error,
                "Server failure recorded"
            );
        }
    }

    /// Register tools from a server
    pub async fn register_tools(&self, server_id: &str, tools: Vec<ToolDefinition>) {
        let mut registry = self.tool_registry.write().await;
        for mut tool in tools {
            tool.server_id = server_id.to_string();
            registry.register(tool);
        }
        tracing::info!(server_id = %server_id, tool_count = registry.tools_by_server(server_id).len(), "Registered tools");
    }

    /// Route a tool call to the appropriate server
    pub async fn route(&self, tool_name: &str) -> Result<McpServerConnection, GatewayError> {
        let registry = self.tool_registry.read().await;
        let server_id = registry
            .get_server_for_tool(tool_name)
            .ok_or_else(|| GatewayError::ToolNotFound {
                tool_name: tool_name.to_string(),
            })?;

        let servers = self.servers.read().await;
        let server = servers
            .get(server_id)
            .ok_or_else(|| GatewayError::ServerNotAvailable {
                server_id: server_id.to_string(),
            })?;

        // Check server is available
        if server.status != ServerStatus::Connected {
            return Err(GatewayError::ServerNotAvailable {
                server_id: server_id.to_string(),
            });
        }

        Ok(server.clone())
    }

    /// Get server by ID
    pub async fn get_server(&self, server_id: &str) -> Option<McpServerConnection> {
        let servers = self.servers.read().await;
        servers.get(server_id).cloned()
    }

    /// Get all servers
    pub async fn all_servers(&self) -> Vec<McpServerConnection> {
        let servers = self.servers.read().await;
        servers.values().cloned().collect()
    }

    /// Get tool definition
    pub async fn get_tool(&self, tool_name: &str) -> Option<ToolDefinition> {
        let registry = self.tool_registry.read().await;
        registry.get_tool(tool_name).map(|r| r.definition.clone())
    }

    /// List all tools
    pub async fn list_tools(&self) -> Vec<ToolDefinition> {
        let registry = self.tool_registry.read().await;
        registry.all_tools().iter().map(|r| r.definition.clone()).collect()
    }

    /// Search tools
    pub async fn search_tools(&self, query: &str) -> Vec<ToolDefinition> {
        let registry = self.tool_registry.read().await;
        registry.search(query).iter().map(|r| r.definition.clone()).collect()
    }

    /// Record tool call statistics
    pub async fn record_tool_call(&self, tool_name: &str, execution_ms: f64) {
        let mut registry = self.tool_registry.write().await;
        registry.record_call(tool_name, execution_ms);
    }

    /// Health check all servers
    pub async fn health_check(&self) -> HealthReport {
        let servers = self.servers.read().await;
        let registry = self.tool_registry.read().await;

        let mut server_statuses = HashMap::new();
        let mut all_healthy = true;

        for (server_id, server) in servers.iter() {
            let is_healthy = server.status == ServerStatus::Connected;
            if !is_healthy {
                all_healthy = false;
            }

            server_statuses.insert(
                server_id.clone(),
                ServerHealthStatus {
                    server_id: server_id.clone(),
                    name: server.name.clone(),
                    status: server.status,
                    last_check: server.last_health_check,
                    tool_count: registry.tools_by_server(server_id).len(),
                    error: if !is_healthy {
                        Some(format!("Status: {:?}", server.status))
                    } else {
                        None
                    },
                },
            );
        }

        HealthReport {
            timestamp: Utc::now(),
            servers: server_statuses,
            overall_healthy: all_healthy,
            total_tools: registry.count(),
        }
    }
}

impl Default for McpRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_router_register_server() {
        let router = McpRouter::new();

        let config = McpServerConfig {
            id: Some("test-server".to_string()),
            name: "Test Server".to_string(),
            endpoint: ServerEndpoint::Stdio {
                command: "test".to_string(),
                args: vec![],
                env: None,
            },
            health_check_interval_secs: 30,
            connection_timeout_secs: 10,
            auto_reconnect: true,
            max_reconnect_attempts: 3,
            metadata: HashMap::new(),
        };

        let server_id = router.register_server(config).await.unwrap();
        assert_eq!(server_id, "test-server");

        let server = router.get_server("test-server").await;
        assert!(server.is_some());
        assert_eq!(server.unwrap().name, "Test Server");
    }

    #[tokio::test]
    async fn test_router_register_tools() {
        let router = McpRouter::new();

        let config = McpServerConfig {
            id: Some("test-server".to_string()),
            name: "Test Server".to_string(),
            endpoint: ServerEndpoint::Stdio {
                command: "test".to_string(),
                args: vec![],
                env: None,
            },
            health_check_interval_secs: 30,
            connection_timeout_secs: 10,
            auto_reconnect: true,
            max_reconnect_attempts: 3,
            metadata: HashMap::new(),
        };

        router.register_server(config).await.unwrap();
        router.update_server_status("test-server", ServerStatus::Connected).await;

        let tools = vec![
            ToolDefinition {
                name: "test_tool".to_string(),
                description: Some("A test tool".to_string()),
                input_schema: serde_json::json!({}),
                server_id: "test-server".to_string(),
                requires_confirmation: false,
                tags: vec!["test".to_string()],
                metadata: HashMap::new(),
            },
        ];

        router.register_tools("test-server", tools).await;

        let all_tools = router.list_tools().await;
        assert_eq!(all_tools.len(), 1);
        assert_eq!(all_tools[0].name, "test_tool");
    }

    #[tokio::test]
    async fn test_router_route_tool() {
        let router = McpRouter::new();

        let config = McpServerConfig {
            id: Some("test-server".to_string()),
            name: "Test Server".to_string(),
            endpoint: ServerEndpoint::Stdio {
                command: "test".to_string(),
                args: vec![],
                env: None,
            },
            health_check_interval_secs: 30,
            connection_timeout_secs: 10,
            auto_reconnect: true,
            max_reconnect_attempts: 3,
            metadata: HashMap::new(),
        };

        router.register_server(config).await.unwrap();
        router.update_server_status("test-server", ServerStatus::Connected).await;

        let tools = vec![ToolDefinition {
            name: "test_tool".to_string(),
            description: Some("A test tool".to_string()),
            input_schema: serde_json::json!({}),
            server_id: "test-server".to_string(),
            requires_confirmation: false,
            tags: vec![],
            metadata: HashMap::new(),
        }];

        router.register_tools("test-server", tools).await;

        let server = router.route("test_tool").await.unwrap();
        assert_eq!(server.server_id, "test-server");
    }

    #[tokio::test]
    async fn test_router_tool_not_found() {
        let router = McpRouter::new();

        let result = router.route("nonexistent_tool").await;
        assert!(matches!(result, Err(GatewayError::ToolNotFound { .. })));
    }

    #[tokio::test]
    async fn test_router_search_tools() {
        let router = McpRouter::new();

        let config = McpServerConfig {
            id: Some("test-server".to_string()),
            name: "Test Server".to_string(),
            endpoint: ServerEndpoint::Stdio {
                command: "test".to_string(),
                args: vec![],
                env: None,
            },
            health_check_interval_secs: 30,
            connection_timeout_secs: 10,
            auto_reconnect: true,
            max_reconnect_attempts: 3,
            metadata: HashMap::new(),
        };

        router.register_server(config).await.unwrap();

        let tools = vec![
            ToolDefinition {
                name: "file_read".to_string(),
                description: Some("Read a file".to_string()),
                input_schema: serde_json::json!({}),
                server_id: "test-server".to_string(),
                requires_confirmation: false,
                tags: vec!["file".to_string()],
                metadata: HashMap::new(),
            },
            ToolDefinition {
                name: "file_write".to_string(),
                description: Some("Write a file".to_string()),
                input_schema: serde_json::json!({}),
                server_id: "test-server".to_string(),
                requires_confirmation: true,
                tags: vec!["file".to_string()],
                metadata: HashMap::new(),
            },
            ToolDefinition {
                name: "bash".to_string(),
                description: Some("Execute shell command".to_string()),
                input_schema: serde_json::json!({}),
                server_id: "test-server".to_string(),
                requires_confirmation: true,
                tags: vec!["shell".to_string()],
                metadata: HashMap::new(),
            },
        ];

        router.register_tools("test-server", tools).await;

        let file_tools = router.search_tools("file").await;
        assert_eq!(file_tools.len(), 2);

        let shell_tools = router.search_tools("shell").await;
        assert_eq!(shell_tools.len(), 1);
    }

    #[tokio::test]
    async fn test_router_health_check() {
        let router = McpRouter::new();

        let config = McpServerConfig {
            id: Some("test-server".to_string()),
            name: "Test Server".to_string(),
            endpoint: ServerEndpoint::Stdio {
                command: "test".to_string(),
                args: vec![],
                env: None,
            },
            health_check_interval_secs: 30,
            connection_timeout_secs: 10,
            auto_reconnect: true,
            max_reconnect_attempts: 3,
            metadata: HashMap::new(),
        };

        router.register_server(config).await.unwrap();
        router.update_server_status("test-server", ServerStatus::Connected).await;

        let report = router.health_check().await;
        assert!(report.overall_healthy);
        assert_eq!(report.servers.len(), 1);
    }
}
