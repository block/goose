# Goose MCP Gateway

## Overview

The MCP Gateway provides a unified interface for managing multiple MCP (Model Context Protocol) servers. It handles routing, permissions, credential management, audit logging, and user bundles.

## Features

- **Multi-Server Routing**: Connect and manage multiple MCP servers
- **Function-Level Permissions**: Fine-grained access control for tools
- **Credential Management**: Secure storage with organization and user scopes
- **Audit Logging**: Comprehensive logging of all operations
- **User Bundles**: Package tools and permissions for user groups

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     MCP Gateway                             │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
│  │   Router    │  │ Permission  │  │    Credential       │ │
│  │             │  │  Manager    │  │      Store          │ │
│  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘ │
│         │                │                     │            │
│  ┌──────┴──────┐  ┌──────┴──────┐  ┌──────────┴──────────┐ │
│  │   Audit     │  │   Bundle    │  │      Tool           │ │
│  │   Logger    │  │   Manager   │  │    Registry         │ │
│  └─────────────┘  └─────────────┘  └─────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
         │                 │                    │
    ┌────┴────┐      ┌────┴────┐         ┌────┴────┐
    │ MCP     │      │ MCP     │         │ MCP     │
    │Server 1 │      │Server 2 │         │Server N │
    └─────────┘      └─────────┘         └─────────┘
```

## Quick Start

```rust
use goose::mcp_gateway::{McpGateway, GatewayConfig, UserContext};

// Create gateway
let config = GatewayConfig::default();
let gateway = McpGateway::new(config)?;

// Register MCP servers
gateway.register_server(McpServerConfig {
    server_id: "files".to_string(),
    name: "File System".to_string(),
    endpoint: ServerEndpoint::Stdio {
        command: "npx".to_string(),
        args: vec!["@anthropic/mcp-server-filesystem".to_string()],
    },
    ..Default::default()
}).await?;

// Execute a tool
let user_context = UserContext {
    user_id: "user-123".to_string(),
    groups: vec!["developers".to_string()],
    ..Default::default()
};

let result = gateway.execute_tool(
    "read_file",
    json!({"path": "/etc/hosts"}),
    &user_context
).await?;
```

## Router

The router manages connections to multiple MCP servers and routes tool calls appropriately.

### Registering Servers

```rust
use goose::mcp_gateway::router::{McpRouter, McpServerConfig, ServerEndpoint};

let router = McpRouter::new();

// Register a stdio server
router.register_server(McpServerConfig {
    server_id: "my-server".to_string(),
    name: "My MCP Server".to_string(),
    endpoint: ServerEndpoint::Stdio {
        command: "node".to_string(),
        args: vec!["server.js".to_string()],
    },
    ..Default::default()
}).await?;

// Register an SSE server
router.register_server(McpServerConfig {
    server_id: "remote-server".to_string(),
    name: "Remote Server".to_string(),
    endpoint: ServerEndpoint::Sse {
        url: "https://mcp.example.com/sse".to_string(),
    },
    ..Default::default()
}).await?;
```

### Tool Discovery

```rust
// List all tools
let tools = router.list_tools().await?;

// Search for tools
let matches = router.search_tools("file").await?;

// Route a tool call
let server = router.route("read_file")?;
```

### Health Checking

```rust
let health = router.health_check().await;

for (server_id, status) in health.servers {
    println!("{}: {:?}", server_id, status);
}
```

## Permissions

The permission system provides fine-grained access control for tools.

### Permission Policies

```rust
use goose::mcp_gateway::permissions::{
    PermissionManager, PermissionPolicy, PermissionRule,
    Subject, PermissionDecision
};

let manager = PermissionManager::new(DefaultPolicy::Deny);

// Add a policy
manager.add_policy(PermissionPolicy {
    id: "developer-access".to_string(),
    name: "Developer Access".to_string(),
    priority: 10,
    rules: vec![
        PermissionRule {
            tool_pattern: "read_*".to_string(),
            subject: Subject::Group("developers".to_string()),
            decision: PermissionDecision::Allow,
            conditions: vec![],
        },
        PermissionRule {
            tool_pattern: "write_*".to_string(),
            subject: Subject::Group("developers".to_string()),
            decision: PermissionDecision::RequireApproval,
            conditions: vec![],
        },
    ],
}).await?;
```

### Allow Lists

```rust
// Create an allow list for a bundle
let allow_list = manager.create_allow_list(
    "dev-bundle",
    vec![
        "read_file".to_string(),
        "write_file".to_string(),
        "list_directory".to_string(),
    ]
).await?;

// Check permission
let result = manager.check_permission("read_file", &user_context).await?;
match result {
    PermissionCheckResult::Allowed => { /* proceed */ },
    PermissionCheckResult::Denied { reason } => { /* block */ },
    PermissionCheckResult::RequireApproval { approvers } => { /* request approval */ },
}
```

## Credentials

Secure credential management with multiple scopes.

### Credential Scopes

```rust
use goose::mcp_gateway::credentials::{
    CredentialStore, Credentials, CredentialScope, CredentialType
};

// Organization-wide credentials
store.store_credentials(
    "github-server",
    Credentials {
        credential_type: CredentialType::BearerToken,
        value: SecretString::new("ghp_xxxx"),
        expires_at: None,
        ..Default::default()
    },
    CredentialScope::Organization
).await?;

// User-specific credentials
store.store_credentials(
    "github-server",
    Credentials {
        credential_type: CredentialType::BearerToken,
        value: SecretString::new("ghp_user_token"),
        expires_at: None,
        ..Default::default()
    },
    CredentialScope::User("user-123".to_string())
).await?;
```

### Credential Retrieval

```rust
// Gets user credentials if available, falls back to org credentials
let creds = store.get_credentials("github-server", &user_context).await?;

if let Some(creds) = creds {
    let header = creds.to_header_value()?;
}
```

## Audit Logging

Comprehensive logging of all gateway operations.

### Audit Events

```rust
use goose::mcp_gateway::audit::{AuditLogger, AuditEventType};

let logger = AuditLogger::new(MemoryAuditStorage::new());

// Automatic logging via gateway
let result = gateway.execute_tool("read_file", args, &user_context).await?;

// Query audit logs
let entries = logger.query(AuditQuery {
    user_id: Some("user-123".to_string()),
    event_type: Some(AuditEventType::ToolExecutionSuccess),
    start_time: Some(Utc::now() - Duration::hours(24)),
    ..Default::default()
}).await?;
```

### Audit Entry Structure

```rust
pub struct AuditEntry {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub event_type: AuditEventType,
    pub user_context: UserContextSnapshot,
    pub tool_name: String,
    pub server_id: String,
    pub request: AuditRequest,
    pub response: Option<AuditResponse>,
    pub duration_ms: Option<u64>,
    pub metadata: HashMap<String, Value>,
}
```

## User Bundles

Package tools and permissions for user groups.

### Creating Bundles

```rust
use goose::mcp_gateway::bundles::{BundleManager, Bundle, BundleStatus};

let manager = BundleManager::new();

// Create a bundle
let bundle = manager.create_bundle(Bundle {
    id: "developer-tools".to_string(),
    name: "Developer Tools".to_string(),
    description: "Tools for software developers".to_string(),
    tools: vec![
        "read_file".to_string(),
        "write_file".to_string(),
        "run_command".to_string(),
    ],
    allowed_users: vec!["user-123".to_string()],
    allowed_groups: vec!["developers".to_string()],
    status: BundleStatus::Active,
    ..Default::default()
}).await?;
```

### Bundle Access Control

```rust
// Check if user can access tool via bundle
let has_access = manager.check_tool_access(
    "developer-tools",
    "read_file",
    &user_context
).await?;

// Assign user to bundle
manager.assign_user("developer-tools", "user-456").await?;
```

## Configuration

### GatewayConfig

```rust
pub struct GatewayConfig {
    /// Default permission policy
    pub default_policy: DefaultPolicy,

    /// Enable audit logging
    pub audit_enabled: bool,

    /// Health check interval (seconds)
    pub health_check_interval_secs: u64,

    /// Request timeout (milliseconds)
    pub request_timeout_ms: u64,

    /// Enable credential caching
    pub credential_cache_enabled: bool,
}
```

## Testing

```bash
# Run MCP Gateway unit tests
cargo test --package goose mcp_gateway::

# Run specific component tests
cargo test --package goose mcp_gateway::router::
cargo test --package goose mcp_gateway::permissions::
cargo test --package goose mcp_gateway::credentials::
cargo test --package goose mcp_gateway::audit::
cargo test --package goose mcp_gateway::bundles::
```

## See Also

- [Enterprise Integration Action Plan](07_ENTERPRISE_INTEGRATION_ACTION_PLAN.md)
- [Comprehensive Audit Report](08_COMPREHENSIVE_AUDIT_REPORT.md)
