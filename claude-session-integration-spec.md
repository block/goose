# Claude Desktop Session Integration Spec

## Goal
Show Claude Desktop sessions alongside goose native sessions in the session list, allowing users to view their Claude Desktop conversation history within goose.

## Architecture

### Session Source Types

Add a new `SessionSource` concept to distinguish where sessions come from:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SessionSource {
    /// Native goose session (stored in SQLite)
    Goose,
    /// External Claude Desktop session (read from JSONL files)
    ClaudeDesktop,
}
```

### Extended Session Model

Extend the `Session` struct to include source information:

```rust
pub struct Session {
    pub id: String,
    pub working_dir: PathBuf,
    pub name: String,
    pub session_type: SessionType,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub conversation: Option<Conversation>,
    pub message_count: usize,
    
    // NEW: Session source
    #[serde(default = "default_session_source")]
    pub source: SessionSource,
    
    // NEW: Original session ID for external sessions
    pub external_id: Option<String>,
    
    // ... other fields
}

fn default_session_source() -> SessionSource {
    SessionSource::Goose
}
```

## Implementation Strategy

### Phase 1: Claude Session Parser Module

Create `crates/goose/src/session/claude_desktop.rs`:

```rust
pub struct ClaudeDesktopSessionReader {
    projects_dir: PathBuf,
}

impl ClaudeDesktopSessionReader {
    pub fn new() -> Result<Self> {
        let projects_dir = dirs::home_dir()
            .ok_or_else(|| anyhow!("Could not determine home directory"))?
            .join(".claude")
            .join("projects");
        
        Ok(Self { projects_dir })
    }
    
    /// List all Claude Desktop sessions
    pub fn list_sessions(&self) -> Result<Vec<ClaudeSession>> {
        // Scan ~/.claude/projects/ directories
        // Find *.jsonl files (not agent-* files)
        // Parse and return session metadata
    }
    
    /// Load a specific Claude session with messages
    pub fn load_session(&self, session_id: &str) -> Result<ClaudeSession> {
        // Load full JSONL file and parse messages
    }
}

#[derive(Debug, Clone)]
pub struct ClaudeSession {
    pub session_id: String,
    pub working_dir: PathBuf,
    pub file_path: PathBuf,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub messages: Vec<ClaudeMessage>,
}

#[derive(Debug, Clone)]
pub struct ClaudeMessage {
    pub uuid: String,
    pub role: Role,
    pub timestamp: DateTime<Utc>,
    pub content: Vec<ClaudeMessageContent>,
}

#[derive(Debug, Clone)]
pub enum ClaudeMessageContent {
    Text { text: String },
    Thinking { thinking: String },
    ToolUse { id: String, name: String, input: serde_json::Value },
    ToolResult { tool_use_id: String, content: String, is_error: bool },
}
```

### Phase 2: Session Manager Integration

Modify `SessionManager` to aggregate sessions from multiple sources:

```rust
impl SessionManager {
    /// List all sessions (goose + external)
    pub async fn list_all_sessions() -> Result<Vec<Session>> {
        let mut all_sessions = Vec::new();
        
        // Get native goose sessions
        let goose_sessions = Self::list_sessions().await?;
        all_sessions.extend(goose_sessions);
        
        // Get Claude Desktop sessions (if available)
        if let Ok(reader) = ClaudeDesktopSessionReader::new() {
            if let Ok(claude_sessions) = reader.list_sessions() {
                all_sessions.extend(
                    claude_sessions.into_iter()
                        .map(|cs| convert_claude_to_session(cs))
                );
            }
        }
        
        // Sort by updated_at (most recent first)
        all_sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        
        Ok(all_sessions)
    }
    
    /// Get a session (from any source)
    pub async fn get_session_any_source(id: &str) -> Result<Session> {
        // Try to parse the ID to determine source
        if id.starts_with("claude_") {
            // Load from Claude Desktop
            let session_id = id.strip_prefix("claude_").unwrap();
            let reader = ClaudeDesktopSessionReader::new()?;
            let claude_session = reader.load_session(session_id)?;
            Ok(convert_claude_to_session_full(claude_session))
        } else {
            // Load from goose database
            Self::get_session(id, true).await
        }
    }
}

fn convert_claude_to_session(claude: ClaudeSession) -> Session {
    Session {
        id: format!("claude_{}", claude.session_id),
        working_dir: claude.working_dir,
        name: generate_name_from_messages(&claude.messages),
        session_type: SessionType::User,
        source: SessionSource::ClaudeDesktop,
        external_id: Some(claude.session_id),
        created_at: claude.created_at,
        updated_at: claude.updated_at,
        message_count: claude.messages.len(),
        conversation: None,  // Don't load full conversation for list
        // Defaults for goose-specific fields
        user_set_name: false,
        extension_data: ExtensionData::default(),
        total_tokens: None,
        // ... etc
    }
}

fn convert_claude_to_session_full(claude: ClaudeSession) -> Session {
    // Convert Claude messages to goose Message format
    let messages = claude.messages.into_iter()
        .map(|cm| convert_claude_message(cm))
        .collect();
    
    let mut session = convert_claude_to_session(claude);
    session.conversation = Some(Conversation::new_unvalidated(messages));
    session
}

fn convert_claude_message(msg: ClaudeMessage) -> Message {
    // Map Claude message types to goose MessageContent
    let content: Vec<MessageContent> = msg.content.into_iter()
        .filter_map(|c| match c {
            ClaudeMessageContent::Text { text } => {
                Some(MessageContent::text(text))
            },
            ClaudeMessageContent::ToolUse { name, input, .. } => {
                Some(MessageContent::ToolRequest {
                    name,
                    params: input,
                })
            },
            ClaudeMessageContent::ToolResult { content, is_error, .. } => {
                Some(MessageContent::ToolResponse { content })
            },
            // Skip thinking blocks for now (could add as metadata)
            ClaudeMessageContent::Thinking { .. } => None,
        })
        .collect();
    
    Message::new(msg.role, msg.timestamp.timestamp_millis(), content)
        .with_id(format!("claude_{}", msg.uuid))
}
```

### Phase 3: API Route Updates

Update routes to handle both session types:

```rust
// In routes/session.rs

#[utoipa::path(
    get,
    path = "/sessions",
    responses(
        (status = 200, description = "List of sessions from all sources", body = SessionListResponse),
    ),
    tag = "Session Management"
)]
async fn list_sessions() -> Result<Json<SessionListResponse>, StatusCode> {
    // NEW: Use list_all_sessions instead
    let sessions = SessionManager::list_all_sessions()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(SessionListResponse { sessions }))
}

#[utoipa::path(
    get,
    path = "/sessions/{session_id}",
    responses(
        (status = 200, description = "Session retrieved successfully", body = Session),
    ),
    tag = "Session Management"
)]
async fn get_session(Path(session_id): Path<String>) -> Result<Json<Session>, StatusCode> {
    // NEW: Use get_session_any_source
    let session = SessionManager::get_session_any_source(&session_id)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(session))
}
```

### Phase 4: Read-Only Operations

Claude Desktop sessions should be read-only from goose:

```rust
impl SessionManager {
    pub async fn update_session(id: &str) -> Result<SessionUpdateBuilder, Error> {
        // Check if this is a Claude session
        if id.starts_with("claude_") {
            return Err(anyhow!("Cannot modify external Claude Desktop sessions"));
        }
        
        Ok(SessionUpdateBuilder::new(id.to_string()))
    }
    
    pub async fn delete_session(id: &str) -> Result<()> {
        // Check if this is a Claude session
        if id.starts_with("claude_") {
            return Err(anyhow!("Cannot delete external Claude Desktop sessions"));
        }
        
        Self::instance().await?.delete_session(id).await
    }
}
```

## UI Considerations

### Session List Display

In the UI, show sessions with visual distinction:

```typescript
interface Session {
  id: string;
  name: string;
  workingDir: string;
  source: 'goose' | 'claude_desktop';  // NEW
  createdAt: string;
  updatedAt: string;
  messageCount: number;
  // ... other fields
}

// In the session list component
{sessions.map(session => (
  <SessionCard 
    key={session.id}
    session={session}
    readOnly={session.source === 'claude_desktop'}
    icon={session.source === 'claude_desktop' ? <ClaudeIcon /> : <GooseIcon />}
  />
))}
```

### Session Detail View

Show sessions in read-only mode:
- Display messages in conversation format
- No ability to continue the conversation
- No ability to rename or delete
- Show a badge/indicator: "Claude Desktop Session (Read-Only)"

## ID Mapping Strategy

To avoid conflicts and clearly distinguish session sources:

| Source | ID Format | Example |
|--------|-----------|---------|
| Goose Native | `YYYYMMDD_N` | `20251119_1` |
| Claude Desktop | `claude_{uuid}` | `claude_b0c8fba6-0600-4013-bdcf-2d6d41bb48d6` |

The `claude_` prefix allows:
1. Easy identification in code
2. No conflicts with goose IDs
3. Can extract original UUID when needed

## Working Directory Mapping

Claude stores working directories as encoded folder names:
- `/Users/micn/Documents` → `-Users-micn-Documents`

Mapping logic:
```rust
fn decode_claude_working_dir(project_dir_name: &str) -> PathBuf {
    let path_str = project_dir_name
        .strip_prefix('-')
        .unwrap_or(project_dir_name)
        .replace('-', "/");
    PathBuf::from(format!("/{}", path_str))
}
```

## Handling Agent Sidechains

Claude Desktop has agent sidechain files (`agent-*.jsonl`):

**Option 1: Ignore for now**
- Only show main sessions
- Agent sidechains are internal implementation detail

**Option 2: Include as metadata**
- Parse agent files
- Show as sub-conversations or linked sessions
- Display in session detail view

**Recommendation: Option 1** for initial implementation, can add Option 2 later if valuable.

## Error Handling

Graceful degradation if Claude Desktop not installed or no sessions:

```rust
pub fn list_sessions(&self) -> Result<Vec<ClaudeSession>> {
    if !self.projects_dir.exists() {
        return Ok(Vec::new());  // No Claude Desktop sessions, return empty
    }
    
    // ... rest of implementation
}
```

Never fail the entire session list if Claude parsing fails:
```rust
pub async fn list_all_sessions() -> Result<Vec<Session>> {
    let mut all_sessions = Vec::new();
    
    // Always get goose sessions
    let goose_sessions = Self::list_sessions().await?;
    all_sessions.extend(goose_sessions);
    
    // Optionally add Claude sessions (don't fail if unavailable)
    match ClaudeDesktopSessionReader::new() {
        Ok(reader) => {
            if let Ok(claude_sessions) = reader.list_sessions() {
                all_sessions.extend(
                    claude_sessions.into_iter()
                        .map(|cs| convert_claude_to_session(cs))
                );
            }
        }
        Err(e) => {
            // Log but don't fail
            tracing::debug!("Could not read Claude Desktop sessions: {}", e);
        }
    }
    
    all_sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(all_sessions)
}
```

## Migration Path

This is additive, no migration needed:
1. Existing goose sessions continue to work as before
2. New `source` field defaults to `SessionSource::Goose`
3. Claude sessions appear as new entries with `source = SessionSource::ClaudeDesktop`
4. OpenAPI schema updated automatically via codegen

## Testing Strategy

### Unit Tests
- `claude_desktop.rs`: Test JSONL parsing
- Test working directory decoding
- Test message conversion

### Integration Tests
- Mock Claude session files
- Test session listing with mixed sources
- Test read-only enforcement

### Manual Testing
- Test with real Claude Desktop installation
- Verify sessions appear in list
- Verify read-only behavior
- Test error handling when Claude not installed

## Open Questions

1. **Session Names**: How to generate good names for Claude sessions?
   - Option A: Use first user message (truncated)
   - Option B: Generate from conversation content (like goose does)
   - **Recommendation**: Option A initially, can enhance later

2. **Refresh Strategy**: Claude sessions are external files that can change
   - Option A: Re-scan on every list request (simple, always fresh)
   - Option B: Cache with TTL (faster, might be stale)
   - **Recommendation**: Option A initially (performance impact likely minimal)

3. **Message Metadata**: Claude has additional fields (thinking, token usage, model)
   - Store in Message.metadata?
   - Display in UI?
   - **Recommendation**: Store what we can in metadata, display later if useful

4. **Search Integration**: Should Claude sessions be searchable via chat_history_search?
   - Probably yes, but needs separate implementation
   - Could be a follow-up feature

## Implementation Checklist

- [ ] Create `claude_desktop.rs` module with parser
- [ ] Add `SessionSource` enum to Session struct
- [ ] Implement `ClaudeDesktopSessionReader`
- [ ] Add `list_all_sessions()` to SessionManager
- [ ] Add `get_session_any_source()` to SessionManager
- [ ] Update API routes to use new methods
- [ ] Add read-only checks for update/delete
- [ ] Update OpenAPI schema (via generate-openapi)
- [ ] Add UI indicator for external sessions
- [ ] Add tests
- [ ] Documentation

## Benefits

1. **Unified View**: See all your AI conversations in one place
2. **Context Awareness**: Reference past Claude conversations from goose
3. **Non-Invasive**: Doesn't modify Claude Desktop sessions
4. **Graceful**: Works with or without Claude Desktop installed
5. **Extensible**: Pattern can be used for other external session sources (Cursor, etc.)
