---
date: 2025-12-29T12:37:16-05:00
git_commit: f12744e22a74529d541e1cd78d50605933b76219
branch: remove-advent-of-ai-banner
repository: goose
topic: "LLM Tool Discovery Implementation"
tags: [research, codebase, tools, mcp, extensions, providers]
status: complete
---

# Research: LLM Tool Discovery Implementation

## Research Question
How is LLM Tool Discovery implemented in goose? This covers how tools are defined, registered, discovered from extensions, presented to LLMs, and invoked.

## Summary

Goose implements tool discovery through a layered architecture built on the Model Context Protocol (MCP). Tools flow through this pipeline:

```
MCP Extensions → McpClientTrait → ExtensionManager → Agent → Provider → LLM API
```

Key components:
1. **Tool Definition**: Tools use JSON Schema via `rmcp::model::Tool` struct
2. **Tool Registration**: Three patterns - MCP Server (`#[tool]` macro), Platform Extensions (`McpClientTrait`), Frontend tools
3. **Tool Discovery**: `ExtensionManager.get_prefixed_tools()` aggregates tools from all extensions concurrently
4. **Tool Presentation**: Provider-specific formatters serialize tools for OpenAI/Anthropic/etc APIs
5. **Tool Invocation**: LLM responses trigger `call_tool()` on the appropriate extension client

## Detailed Findings

### 1. Tool Definition & Schema

**Core Type**: `rmcp::model::Tool`

Tools are defined with three fields:
- `name`: String identifier (prefixed with extension name, e.g., `developer__shell`)
- `description`: Human-readable description for the LLM
- `input_schema`: JSON Schema object defining parameters

**File References**:
- `crates/mcp-core/src/tool.rs` - Core Tool type from rmcp crate
- `crates/goose/src/agents/todo_extension.rs:140-165` - Example tool schema generation using `schema_for!` macro

```rust
fn get_tools() -> Vec<Tool> {
    let schema = schema_for!(TodoWriteParams);
    let schema_value = serde_json::to_value(schema).expect("Failed to serialize schema");
    vec![Tool::new(
        "todo_write".to_string(),
        "Overwrite the entire TODO content...".to_string(),
        schema_value.as_object().unwrap().clone(),
    )]
}
```

---

### 2. Tool Registration Patterns

#### Pattern A: MCP Server Extensions (Built-in/External)

Uses `rmcp` crate's `#[tool]` and `#[tool_router]` macros.

**File References**:
- `crates/goose-mcp/src/developer/rmcp_developer.rs:175-190` - DeveloperServer with ToolRouter
- `crates/goose-mcp/src/memory/mod.rs:66` - MemoryServer
- `crates/goose-mcp/src/computercontroller/mod.rs:281` - ComputerControllerServer

```rust
#[tool_router(router = tool_router)]
impl DeveloperServer {
    #[tool(
        name = "shell",
        description = "Execute a command in the shell..."
    )]
    pub async fn shell(&self, params: Parameters<ShellParams>) -> Result<CallToolResult, ErrorData> {
        // implementation
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for DeveloperServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            // ...
        }
    }
}
```

#### Pattern B: Platform Extensions (In-Process)

Implement `McpClientTrait` directly with `list_tools()` and `call_tool()` methods.

**File References**:
- `crates/goose/src/agents/mcp_client.rs:41-76` - McpClientTrait definition
- `crates/goose/src/agents/todo_extension.rs:140-196` - TodoClient
- `crates/goose/src/agents/extension_manager_extension.rs:277-413` - ExtensionManagerClient
- `crates/goose/src/agents/chatrecall_extension.rs:247-306` - ChatRecallClient

```rust
#[async_trait::async_trait]
pub trait McpClientTrait: Send + Sync {
    async fn list_tools(
        &self,
        next_cursor: Option<String>,
        cancel_token: CancellationToken,
    ) -> Result<ListToolsResult, Error>;
    
    async fn call_tool(
        &self,
        name: &str,
        arguments: Option<JsonObject>,
        cancel_token: CancellationToken,
    ) -> Result<CallToolResult, Error>;
    // ... other methods
}
```

#### Pattern C: Frontend Tools

Tools provided by the UI/frontend, stored in `Agent.frontend_tools`.

**File References**:
- `crates/goose/src/agents/agent.rs:638-670` - Frontend tool aggregation
- `crates/goose/src/agents/reply_parts.rs:117-160` - Frontend tools added during preparation

---

### 3. Extension Configuration Types

**File**: `crates/goose/src/agents/extension.rs:40-95`

```rust
pub enum ExtensionConfig {
    Sse { name, uri, timeout, ... },           // Server-Sent Events
    StreamableHttp { name, uri, headers, ... }, // MCP Streaming HTTP
    Stdio { name, cmd, args, envs, ... },      // Command-line subprocess
    Builtin { name, display_name, ... },       // Built-in goose MCP servers
    Platform { name, ... },                     // In-process extensions
    Frontend { name, tools, ... },              // Frontend-provided tools
    InlinePython { name, code, ... },          // Python code execution
}
```

Platform extensions are registered in a static HashMap:

**File**: `crates/goose/src/agents/extension.rs:40-95`

```rust
pub static PLATFORM_EXTENSIONS: Lazy<HashMap<&'static str, PlatformExtensionDef>> = Lazy::new(|| {
    let mut map = HashMap::new();
    map.insert("todo", PlatformExtensionDef { 
        client_factory: |ctx| Box::new(todo_extension::TodoClient::new(ctx).unwrap()) 
    });
    map.insert("extensionmanager", ...);
    map.insert("skills", ...);
    // etc.
    map
});
```

---

### 4. Tool Discovery Flow

#### 4.1 ExtensionManager.get_prefixed_tools()

**File**: `crates/goose/src/agents/extension_manager.rs:680-750`

The central tool aggregation method:

```rust
pub async fn get_prefixed_tools(&self, extension_name: Option<String>) -> ExtensionResult<Vec<Tool>> {
    // 1. Filter clients based on extension_name
    let filtered_clients: Vec<_> = self.extensions.lock().await
        .iter()
        .filter(|(name, _)| extension_name.as_ref().map_or(true, |n| *name == n))
        .map(|(name, ext)| (name.clone(), ext.config.clone(), ext.get_client()))
        .collect();

    // 2. Spawn concurrent tasks to fetch tools from each extension
    for (name, config, client) in filtered_clients {
        let mut client_tools = client.lock().await.list_tools(None, cancel_token).await?;
        
        // 3. Handle pagination
        loop {
            for tool in client_tools.tools {
                // 4. Check tool availability filter
                if config.is_tool_available(&tool.name) {
                    // 5. Prefix tool name with extension name
                    tools.push(Tool {
                        name: format!("{}__{}", name, tool.name).into(),
                        description: tool.description,
                        input_schema: tool.input_schema,
                    });
                }
            }
            if client_tools.next_cursor.is_none() { break; }
            client_tools = client.list_tools(client_tools.next_cursor, ...).await?;
        }
    }
    Ok(tools)
}
```

Key behaviors:
- **Concurrent fetching**: Uses `FuturesUnordered` to fetch from all extensions in parallel
- **Pagination support**: Handles `next_cursor` for large tool sets
- **Name prefixing**: Tools are prefixed as `{extension_name}__{tool_name}` (e.g., `developer__shell`)
- **Filtering**: Respects `available_tools` config to limit exposed tools

#### 4.2 Agent.list_tools()

**File**: `crates/goose/src/agents/agent.rs:638-670`

Aggregates tools from multiple sources:

```rust
pub async fn list_tools(&self, extension_name: Option<String>) -> Vec<Tool> {
    // 1. Get prefixed tools from ExtensionManager
    let mut prefixed_tools = self.extension_manager
        .get_prefixed_tools(extension_name.clone()).await.unwrap_or_default();

    // 2. Add platform tools (schedule management)
    if extension_name.is_none() || extension_name.as_deref() == Some("platform") {
        prefixed_tools.push(platform_tools::manage_schedule_tool());
    }

    // 3. Add final output tool if configured
    if let Some(final_output_tool) = self.final_output_tool.lock().await.as_ref() {
        prefixed_tools.push(final_output_tool.tool());
    }

    // 4. Add subagent tool if enabled
    if subagents_enabled {
        prefixed_tools.push(create_subagent_tool(&sub_recipes_vec));
    }

    prefixed_tools
}
```

#### 4.3 Tool Preparation for LLM

**File**: `crates/goose/src/agents/reply_parts.rs:112-175`

```rust
pub async fn prepare_tools_and_prompt(&self, working_dir: &Path) -> Result<(Vec<Tool>, Vec<Tool>, String)> {
    // Get tools from extension manager
    let mut tools = self.list_tools(None).await;

    // Add frontend tools
    let frontend_tools = self.frontend_tools.lock().await;
    for frontend_tool in frontend_tools.values() {
        tools.push(frontend_tool.tool.clone());
    }

    // Stable tool ordering for prompt caching
    tools.sort_by(|a, b| a.name.cmp(&b.name));

    // Build system prompt with extension info
    Ok((tools, toolshim_tools, system_prompt))
}
```

---

### 5. Tool Presentation to LLM (Provider Serialization)

#### 5.1 Provider Interface

**File**: `crates/goose/src/providers/base.rs:359-376`

```rust
async fn complete_with_model(
    &self, 
    model_config: &ModelConfig, 
    system: &str, 
    messages: &[Message], 
    tools: &[Tool]
) -> Result<(Message, ProviderUsage), ProviderError>;
```

#### 5.2 OpenAI Format

**File**: `crates/goose/src/providers/formats/openai.rs:259-280`

```rust
pub fn format_tools(tools: &[Tool]) -> anyhow::Result<Vec<Value>> {
    let mut tool_names = std::collections::HashSet::new();
    let mut result = Vec::new();

    for tool in tools {
        if !tool_names.insert(&tool.name) {
            return Err(anyhow!("Duplicate tool name: {}", tool.name));
        }

        result.push(json!({
            "type": "function",
            "function": {
                "name": tool.name,
                "description": tool.description,
                "parameters": tool.input_schema,
            }
        }));
    }
    Ok(result)
}
```

**Resulting JSON**:
```json
{
  "tools": [
    {
      "type": "function",
      "function": {
        "name": "developer__shell",
        "description": "Execute a command in the shell",
        "parameters": {
          "type": "object",
          "properties": { "command": { "type": "string" } },
          "required": ["command"]
        }
      }
    }
  ]
}
```

#### 5.3 Anthropic Format

**File**: `crates/goose/src/providers/formats/anthropic.rs:175-202`

```rust
pub fn format_tools(tools: &[Tool]) -> Vec<Value> {
    for tool in tools {
        if unique_tools.insert(tool.name.clone()) {
            tool_specs.push(json!({
                "name": tool.name,
                "description": tool.description,
                "input_schema": anthropic_flavored_input_schema(tool.input_schema.clone())
            }));
        }
    }
    // Adds cache_control to last tool for prompt caching
}
```

**Resulting JSON**:
```json
{
  "tools": [
    {
      "name": "developer__shell",
      "description": "Execute a command in the shell",
      "input_schema": {
        "type": "object",
        "properties": { "command": { "type": "string" } }
      },
      "cache_control": { "type": "ephemeral" }
    }
  ]
}
```

#### 5.4 Provider-Specific Variations

| Provider | Format Module | Key Differences |
|----------|--------------|-----------------|
| OpenAI | `formats/openai.rs` | Uses `"type": "function"` wrapper, `parameters` field |
| Anthropic | `formats/anthropic.rs` | Uses `input_schema`, adds `cache_control` for caching |
| Google/Vertex | `formats/google.rs`, `formats/gcpvertexai.rs` | Similar to OpenAI |
| Databricks | `formats/databricks.rs` | OpenAI-compatible |
| Bedrock | `formats/bedrock.rs` | AWS-specific format |

---

### 6. Extension Startup and MCP Client Creation

**File**: `crates/goose/src/agents/extension_manager.rs:355-545`

```rust
pub async fn add_extension(&self, config: ExtensionConfig) -> ExtensionResult<()> {
    match config {
        // Stdio: Spawn subprocess, connect via stdin/stdout
        ExtensionConfig::Stdio { cmd, args, envs, .. } => {
            let command = Command::new(cmd).configure(|c| c.args(args).envs(all_envs));
            let client = child_process_client(command, timeout, provider).await?;
        }
        
        // Builtin: Run goose binary with "mcp <name>" args
        ExtensionConfig::Builtin { name, .. } => {
            let cmd = std::env::current_exe()?;
            let command = Command::new(cmd).configure(|c| c.arg("mcp").arg(name));
            let client = child_process_client(command, timeout, provider).await?;
        }
        
        // Platform: Create in-process client from factory
        ExtensionConfig::Platform { name, .. } => {
            let def = PLATFORM_EXTENSIONS.get(normalized_key)?;
            let context = self.get_context().await;
            (def.client_factory)(context)
        }
        
        // SSE/StreamableHttp: Connect via HTTP transport
        ExtensionConfig::StreamableHttp { uri, .. } => {
            let transport = StreamableHttpClientTransport::with_client(client, config);
            McpClient::connect(transport, timeout, provider).await?
        }
    }
}
```

---

### 7. Server Routes for Tool Discovery

**File**: `crates/goose-server/src/routes/agent.rs`

HTTP endpoint for fetching tools (used by desktop app):

```rust
#[utoipa::path(get, path = "/agent/tools")]
async fn get_tools(
    State(state): State<Arc<AppState>>,
    Query(query): Query<GetToolsQuery>,
) -> Result<Json<Vec<ToolInfo>>, StatusCode> {
    let agent = state.get_agent_for_route(query.session_id).await?;
    
    let tools: Vec<ToolInfo> = agent
        .list_tools(query.extension_name)
        .await
        .into_iter()
        .map(|tool| ToolInfo::new(
            &tool.name,
            tool.description.as_ref().map(|d| d.as_ref()).unwrap_or_default(),
            get_parameter_names(&tool),
            permission,
        ))
        .collect();
    
    Ok(Json(tools))
}
```

---

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              User Request                                    │
│                         (CLI, Desktop, Server)                               │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                                Agent                                         │
│                         list_tools(extension_name)                           │
│                      prepare_tools_and_prompt()                              │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                          ExtensionManager                                    │
│                        get_prefixed_tools()                                  │
│   - Iterates over all extensions concurrently                                │
│   - Prefixes tools with extension name (developer__shell)                    │
│   - Handles pagination via next_cursor                                       │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                    ┌───────────────┼───────────────┐
                    ▼               ▼               ▼
            ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
            │  Extension  │ │  Extension  │ │  Extension  │
            │   Client    │ │   Client    │ │   Client    │
            │ (McpClient) │ │ (McpClient) │ │ (Platform)  │
            └─────────────┘ └─────────────┘ └─────────────┘
                    │               │               │
                    ▼               ▼               ▼
            ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
            │ MCP Server  │ │ MCP Server  │ │ In-Process  │
            │  (stdio)    │ │  (http)     │ │  Extension  │
            └─────────────┘ └─────────────┘ └─────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Provider                                        │
│                    complete_with_model(..., tools)                           │
│                                                                              │
│   ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐             │
│   │ formats/openai  │  │formats/anthropic│  │ formats/google  │             │
│   │  format_tools() │  │  format_tools() │  │  format_tools() │             │
│   └─────────────────┘  └─────────────────┘  └─────────────────┘             │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                            LLM API Request                                   │
│                    POST /chat/completions { tools: [...] }                   │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Code References

| File | Line(s) | Description |
|------|---------|-------------|
| `crates/goose/src/agents/mcp_client.rs` | 41-76 | `McpClientTrait` definition |
| `crates/goose/src/agents/extension.rs` | 40-95 | `ExtensionConfig` enum, `PLATFORM_EXTENSIONS` |
| `crates/goose/src/agents/extension_manager.rs` | 355-545 | `add_extension()` - client creation |
| `crates/goose/src/agents/extension_manager.rs` | 680-750 | `get_prefixed_tools()` - tool aggregation |
| `crates/goose/src/agents/agent.rs` | 638-670 | `list_tools()` - final aggregation |
| `crates/goose/src/agents/reply_parts.rs` | 112-175 | `prepare_tools_and_prompt()` |
| `crates/goose/src/providers/base.rs` | 359-376 | Provider trait with tools parameter |
| `crates/goose/src/providers/formats/openai.rs` | 259-280 | OpenAI tool serialization |
| `crates/goose/src/providers/formats/anthropic.rs` | 175-202 | Anthropic tool serialization |
| `crates/goose-mcp/src/developer/rmcp_developer.rs` | 175-190 | `#[tool]` macro usage example |
| `crates/goose/src/agents/todo_extension.rs` | 140-196 | Platform extension example |
| `crates/goose-server/src/routes/agent.rs` | - | `/agent/tools` HTTP endpoint |

---

## Open Questions

1. **Tool Caching**: How are tool definitions cached between requests? The sorting suggests prompt caching optimization, but the caching mechanism itself isn't fully documented.

2. **Dynamic Tool Updates**: How does the system handle tools being added/removed from running MCP servers mid-session?

3. **Tool Permissions**: The `ToolInfo` struct includes `permission` field - how is this enforced at runtime?

4. **Error Handling**: What happens when an extension fails to respond to `list_tools()`? How does this affect other extensions?

5. **Tool Schema Validation**: `validate_tool_schemas()` in openai.rs modifies schemas - what transformations are applied and why?
