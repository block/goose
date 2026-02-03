//! # MCP Gateway Module
//!
//! Enterprise-grade MCP gateway for multi-server routing, permissions, and audit.
//! Inspired by Gate22 architecture with unified endpoint for multiple MCP servers.
//!
//! ## Features
//!
//! - Multi-server routing with tool discovery
//! - Function-level permissions with policies and allow lists
//! - Credential management (org-shared / per-user)
//! - Comprehensive audit logging
//! - User bundle management
//!
//! ## Usage
//!
//! ```rust,ignore
//! use goose::mcp_gateway::{McpGateway, GatewayConfig};
//!
//! let config = GatewayConfig::default();
//! let gateway = McpGateway::new(config)?;
//!
//! // Register MCP servers
//! gateway.register_server(server_config).await?;
//!
//! // Execute tools with permission checks and audit logging
//! let result = gateway.execute_tool("tool_name", args, &user_context).await?;
//! ```

pub mod audit;
pub mod bundles;
pub mod credentials;
pub mod errors;
pub mod permissions;
pub mod router;

pub use audit::{AuditEntry, AuditEventType, AuditLogger, AuditQuery};
pub use bundles::{Bundle, BundleManager, BundleStatus};
pub use credentials::{CredentialManager, CredentialScope, CredentialStore, Credentials};
pub use errors::GatewayError;
pub use permissions::{
    DefaultPolicy, PermissionCheckResult, PermissionDecision, PermissionManager,
    PermissionPolicy, PermissionRule, Subject, UserContext,
};
pub use router::{
    HealthReport, McpRouter, McpServerConfig, McpServerConnection, ServerEndpoint, ServerStatus,
    ToolDefinition,
};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

/// Gateway configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Gateway enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Default permission policy
    #[serde(default)]
    pub default_policy: DefaultPolicy,

    /// Tool execution timeout in seconds
    #[serde(default = "default_timeout_secs")]
    pub execution_timeout_secs: u64,

    /// Enable audit logging
    #[serde(default = "default_audit_enabled")]
    pub audit_enabled: bool,

    /// Redact arguments in audit logs
    #[serde(default = "default_redact_arguments")]
    pub redact_arguments: bool,

    /// Maximum concurrent executions
    #[serde(default)]
    pub max_concurrent_executions: Option<u32>,

    /// Health check interval in seconds
    #[serde(default = "default_health_check_secs")]
    pub health_check_interval_secs: u64,
}

fn default_enabled() -> bool {
    true
}

fn default_timeout_secs() -> u64 {
    30
}

fn default_audit_enabled() -> bool {
    true
}

fn default_redact_arguments() -> bool {
    true
}

fn default_health_check_secs() -> u64 {
    30
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_policy: DefaultPolicy::RequireApproval,
            execution_timeout_secs: 30,
            audit_enabled: true,
            redact_arguments: true,
            max_concurrent_executions: None,
            health_check_interval_secs: 30,
        }
    }
}

/// Tool execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Tool name
    pub tool_name: String,
    /// Execution success
    pub success: bool,
    /// Result content
    pub content: serde_json::Value,
    /// Execution time in milliseconds
    pub execution_ms: u64,
    /// Server that executed the tool
    pub server_id: String,
    /// Error message if failed
    pub error: Option<String>,
}

/// Tool match from search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMatch {
    /// Tool definition
    pub tool: ToolDefinition,
    /// Relevance score (0-1)
    pub score: f64,
}

/// MCP Gateway - unified endpoint for multiple MCP servers
pub struct McpGateway {
    config: GatewayConfig,
    router: Arc<McpRouter>,
    permission_manager: Arc<PermissionManager>,
    credential_manager: Arc<CredentialManager>,
    audit_logger: Arc<AuditLogger>,
    bundle_manager: Arc<BundleManager>,
}

impl McpGateway {
    /// Create gateway with configuration
    pub fn new(config: GatewayConfig) -> Self {
        let router = Arc::new(McpRouter::new());
        let permission_manager = Arc::new(PermissionManager::new(config.default_policy));
        let credential_manager = Arc::new(CredentialManager::memory());
        let audit_logger = Arc::new(
            AuditLogger::memory()
                .with_redaction(config.redact_arguments)
                .with_buffer_size(100),
        );
        let bundle_manager = Arc::new(BundleManager::new());

        Self {
            config,
            router,
            permission_manager,
            credential_manager,
            audit_logger,
            bundle_manager,
        }
    }

    /// Create gateway with custom components
    pub fn with_components(
        config: GatewayConfig,
        router: Arc<McpRouter>,
        permission_manager: Arc<PermissionManager>,
        credential_manager: Arc<CredentialManager>,
        audit_logger: Arc<AuditLogger>,
        bundle_manager: Arc<BundleManager>,
    ) -> Self {
        Self {
            config,
            router,
            permission_manager,
            credential_manager,
            audit_logger,
            bundle_manager,
        }
    }

    /// Register an MCP server
    pub async fn register_server(&self, config: McpServerConfig) -> Result<String, GatewayError> {
        let server_id = self.router.register_server(config).await?;

        if self.config.audit_enabled {
            let entry = AuditEntry::new(
                AuditEventType::ServerRegistered,
                UserContext::new("system").snapshot(),
                "",
                &server_id,
            );
            self.audit_logger.log(entry).await.map_err(|e| {
                GatewayError::AuditError(e.to_string())
            })?;
        }

        Ok(server_id)
    }

    /// Unregister an MCP server
    pub async fn unregister_server(&self, server_id: &str) -> Result<(), GatewayError> {
        self.router.unregister_server(server_id).await?;

        if self.config.audit_enabled {
            let entry = AuditEntry::new(
                AuditEventType::ServerUnregistered,
                UserContext::new("system").snapshot(),
                "",
                server_id,
            );
            self.audit_logger.log(entry).await.map_err(|e| {
                GatewayError::AuditError(e.to_string())
            })?;
        }

        Ok(())
    }

    /// List all available tools across all registered servers
    pub async fn list_tools(&self, user_context: &UserContext) -> Result<Vec<ToolDefinition>, GatewayError> {
        let all_tools = self.router.list_tools().await;

        // Filter by permissions
        let mut allowed_tools = Vec::new();
        for tool in all_tools {
            let result = self.permission_manager.check_permission(&tool.name, user_context).await;
            if result.is_allowed() {
                allowed_tools.push(tool);
            }
        }

        Ok(allowed_tools)
    }

    /// Search for tools matching a query
    pub async fn search_tools(
        &self,
        query: &str,
        user_context: &UserContext,
    ) -> Result<Vec<ToolMatch>, GatewayError> {
        let matching_tools = self.router.search_tools(query).await;

        // Filter by permissions and calculate scores
        let mut results = Vec::new();
        for tool in matching_tools {
            let result = self.permission_manager.check_permission(&tool.name, user_context).await;
            if result.is_allowed() {
                // Simple relevance score based on name match
                let score = if tool.name.to_lowercase() == query.to_lowercase() {
                    1.0
                } else if tool.name.to_lowercase().contains(&query.to_lowercase()) {
                    0.8
                } else {
                    0.5
                };

                results.push(ToolMatch { tool, score });
            }
        }

        // Sort by score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        Ok(results)
    }

    /// Execute a tool with permission checks and audit logging
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        arguments: serde_json::Value,
        user_context: &UserContext,
    ) -> Result<ToolResult, GatewayError> {
        if !self.config.enabled {
            return Err(GatewayError::Internal("Gateway is disabled".to_string()));
        }

        let start = Instant::now();

        // 1. Check permissions
        let permission_result = self.permission_manager.check_permission(tool_name, user_context).await;

        match permission_result {
            PermissionCheckResult::Denied { reason } => {
                if self.config.audit_enabled {
                    self.audit_logger
                        .log_permission_denied(tool_name, &user_context.snapshot(), &reason)
                        .await
                        .ok();
                }
                return Err(GatewayError::PermissionDenied { reason });
            }
            PermissionCheckResult::RequiresApproval { .. } => {
                return Err(GatewayError::PermissionDenied {
                    reason: "Tool requires approval".to_string(),
                });
            }
            PermissionCheckResult::Allowed => {}
        }

        // 2. Route to server
        let server = self.router.route(tool_name).await?;

        // 3. Start audit entry
        let audit_entry = if self.config.audit_enabled {
            Some(self.audit_logger.start_execution(
                tool_name,
                &arguments,
                &user_context.snapshot(),
                &server.server_id,
            ))
        } else {
            None
        };

        // 4. Get credentials (if needed)
        let _credentials = self
            .credential_manager
            .get_credentials(&server.server_id, user_context)
            .await
            .ok();

        // 5. Execute tool (simulated - actual MCP transport would happen here)
        let execution_result = self.simulate_tool_execution(tool_name, &arguments).await;

        let duration_ms = start.elapsed().as_millis() as u64;

        // 6. Record statistics
        self.router.record_tool_call(tool_name, duration_ms as f64).await;

        // 7. Complete audit entry
        if let Some(entry) = audit_entry {
            let completed_entry = match &execution_result {
                Ok(result) => {
                    let result_size = serde_json::to_string(&result.content)
                        .map(|s| s.len())
                        .unwrap_or(0);
                    self.audit_logger.complete_success(entry, result_size, duration_ms)
                }
                Err(e) => {
                    self.audit_logger.complete_failure(
                        entry,
                        "execution_error",
                        &e.to_string(),
                        duration_ms,
                    )
                }
            };
            self.audit_logger.log(completed_entry).await.ok();
        }

        execution_result
    }

    /// Simulate tool execution (placeholder for actual MCP transport)
    async fn simulate_tool_execution(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, GatewayError> {
        // In a real implementation, this would:
        // 1. Connect to the MCP server via the appropriate transport
        // 2. Send the tool call request
        // 3. Wait for response
        // 4. Return the result

        // For now, return a simulated success
        Ok(ToolResult {
            tool_name: tool_name.to_string(),
            success: true,
            content: serde_json::json!({
                "status": "simulated",
                "arguments_received": arguments,
            }),
            execution_ms: 10,
            server_id: "simulated".to_string(),
            error: None,
        })
    }

    /// Health check all servers
    pub async fn health_check(&self) -> HealthReport {
        self.router.health_check().await
    }

    /// Get gateway configuration
    pub fn config(&self) -> &GatewayConfig {
        &self.config
    }

    /// Get router reference
    pub fn router(&self) -> &Arc<McpRouter> {
        &self.router
    }

    /// Get permission manager reference
    pub fn permission_manager(&self) -> &Arc<PermissionManager> {
        &self.permission_manager
    }

    /// Get credential manager reference
    pub fn credential_manager(&self) -> &Arc<CredentialManager> {
        &self.credential_manager
    }

    /// Get audit logger reference
    pub fn audit_logger(&self) -> &Arc<AuditLogger> {
        &self.audit_logger
    }

    /// Get bundle manager reference
    pub fn bundle_manager(&self) -> &Arc<BundleManager> {
        &self.bundle_manager
    }

    /// Check if gateway is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get server count
    pub async fn server_count(&self) -> usize {
        self.router.all_servers().await.len()
    }

    /// Get tool count
    pub async fn tool_count(&self) -> usize {
        self.router.list_tools().await.len()
    }
}

impl Default for McpGateway {
    fn default() -> Self {
        Self::new(GatewayConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gateway_creation() {
        let gateway = McpGateway::default();
        assert!(gateway.is_enabled());
        assert_eq!(gateway.server_count().await, 0);
    }

    #[tokio::test]
    async fn test_gateway_register_server() {
        let gateway = McpGateway::default();

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
            metadata: std::collections::HashMap::new(),
        };

        let server_id = gateway.register_server(config).await.unwrap();
        assert_eq!(server_id, "test-server");
        assert_eq!(gateway.server_count().await, 1);
    }

    #[tokio::test]
    async fn test_gateway_permission_denied() {
        let config = GatewayConfig {
            default_policy: DefaultPolicy::Deny,
            ..Default::default()
        };
        let gateway = McpGateway::new(config);
        let user_context = UserContext::new("user1");

        let result = gateway
            .execute_tool("some_tool", serde_json::json!({}), &user_context)
            .await;

        assert!(matches!(result, Err(GatewayError::PermissionDenied { .. })));
    }

    #[tokio::test]
    async fn test_gateway_tool_execution() {
        let config = GatewayConfig {
            default_policy: DefaultPolicy::Allow,
            ..Default::default()
        };
        let gateway = McpGateway::new(config);

        // Register server and tool
        let server_config = McpServerConfig {
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
            metadata: std::collections::HashMap::new(),
        };
        gateway.register_server(server_config).await.unwrap();
        gateway
            .router
            .update_server_status("test-server", ServerStatus::Connected)
            .await;

        let tool = ToolDefinition {
            name: "test_tool".to_string(),
            description: Some("Test tool".to_string()),
            input_schema: serde_json::json!({}),
            server_id: "test-server".to_string(),
            requires_confirmation: false,
            tags: vec![],
            metadata: std::collections::HashMap::new(),
        };
        gateway.router.register_tools("test-server", vec![tool]).await;

        // Execute tool
        let user_context = UserContext::new("user1");
        let result = gateway
            .execute_tool("test_tool", serde_json::json!({"arg": "value"}), &user_context)
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.tool_name, "test_tool");
    }

    #[tokio::test]
    async fn test_gateway_list_tools_filtered() {
        let config = GatewayConfig {
            default_policy: DefaultPolicy::Deny,
            ..Default::default()
        };
        let gateway = McpGateway::new(config);

        // Register server
        let server_config = McpServerConfig {
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
            metadata: std::collections::HashMap::new(),
        };
        gateway.register_server(server_config).await.unwrap();

        // Register tools
        let tools = vec![
            ToolDefinition {
                name: "allowed_tool".to_string(),
                description: None,
                input_schema: serde_json::json!({}),
                server_id: "test-server".to_string(),
                requires_confirmation: false,
                tags: vec![],
                metadata: std::collections::HashMap::new(),
            },
            ToolDefinition {
                name: "denied_tool".to_string(),
                description: None,
                input_schema: serde_json::json!({}),
                server_id: "test-server".to_string(),
                requires_confirmation: false,
                tags: vec![],
                metadata: std::collections::HashMap::new(),
            },
        ];
        gateway.router.register_tools("test-server", tools).await;

        // Add policy to allow only allowed_tool
        let policy = PermissionPolicy {
            id: "test-policy".to_string(),
            name: "Test Policy".to_string(),
            description: None,
            rules: vec![PermissionRule {
                id: "rule1".to_string(),
                tool_pattern: "allowed_tool".to_string(),
                subject: Subject::All,
                decision: PermissionDecision::Allow,
                conditions: vec![],
                description: None,
            }],
            priority: 100,
            enabled: true,
        };
        gateway.permission_manager.add_policy(policy).await;

        let user_context = UserContext::new("user1");
        let listed_tools = gateway.list_tools(&user_context).await.unwrap();

        // Only allowed_tool should be listed (denied_tool uses default deny)
        assert_eq!(listed_tools.len(), 1);
        assert_eq!(listed_tools[0].name, "allowed_tool");
    }

    #[tokio::test]
    async fn test_gateway_search_tools() {
        let config = GatewayConfig {
            default_policy: DefaultPolicy::Allow,
            ..Default::default()
        };
        let gateway = McpGateway::new(config);

        // Register server
        let server_config = McpServerConfig {
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
            metadata: std::collections::HashMap::new(),
        };
        gateway.register_server(server_config).await.unwrap();

        // Register tools
        let tools = vec![
            ToolDefinition {
                name: "file_read".to_string(),
                description: Some("Read files".to_string()),
                input_schema: serde_json::json!({}),
                server_id: "test-server".to_string(),
                requires_confirmation: false,
                tags: vec!["file".to_string()],
                metadata: std::collections::HashMap::new(),
            },
            ToolDefinition {
                name: "file_write".to_string(),
                description: Some("Write files".to_string()),
                input_schema: serde_json::json!({}),
                server_id: "test-server".to_string(),
                requires_confirmation: true,
                tags: vec!["file".to_string()],
                metadata: std::collections::HashMap::new(),
            },
        ];
        gateway.router.register_tools("test-server", tools).await;

        let user_context = UserContext::new("user1");
        let results = gateway.search_tools("file", &user_context).await.unwrap();

        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_gateway_health_check() {
        let gateway = McpGateway::default();

        // Register server
        let server_config = McpServerConfig {
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
            metadata: std::collections::HashMap::new(),
        };
        gateway.register_server(server_config).await.unwrap();
        gateway
            .router
            .update_server_status("test-server", ServerStatus::Connected)
            .await;

        let report = gateway.health_check().await;
        assert!(report.overall_healthy);
        assert_eq!(report.servers.len(), 1);
    }
}
