# Claude Desktop Session Integration - Simple Approach

## Goal
Show Claude Desktop sessions in goose session list as if they are goose sessions. Keep it super simple - just detect, parse, display, and allow resume.

## Core Principle
Treat Claude sessions **exactly like goose sessions** from the user's perspective. No special handling, no read-only mode, just show them and let users resume/continue.

## Minimal Changes

### 1. Add Session Source (Optional Field)
```rust
// In Session struct
#[serde(default)]
pub source: Option<String>,  // "claude" or None (for goose)
```

This is just for tracking internally, not exposed in UI initially.

### 2. Create Simple Parser
```rust
// crates/goose/src/session/claude_desktop.rs

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct ClaudeJsonLine {
    #[serde(rename = "type")]
    entry_type: Option<String>,
    #[serde(rename = "sessionId")]
    session_id: Option<String>,
    cwd: Option<String>,
    timestamp: Option<String>,
    message: Option<ClaudeMessageWrapper>,
}

#[derive(Debug, Deserialize)]
struct ClaudeMessageWrapper {
    role: Option<String>,
    content: serde_json::Value,
}

pub fn list_claude_sessions() -> Result<Vec<(String, PathBuf, DateTime<Utc>)>> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home dir"))?;
    let projects_dir = home.join(".claude").join("projects");
    
    if !projects_dir.exists() {
        return Ok(Vec::new());
    }
    
    let mut sessions = Vec::new();
    
    // Scan project directories
    for entry in std::fs::read_dir(projects_dir)? {
        let entry = entry?;
        let project_path = entry.path();
        
        if !project_path.is_dir() {
            continue;
        }
        
        // Find main session files (not agent-*)
        for file_entry in std::fs::read_dir(&project_path)? {
            let file_entry = file_entry?;
            let file_path = file_entry.path();
            
            if !file_path.is_file() {
                continue;
            }
            
            let file_name = file_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            
            // Skip agent files
            if file_name.starts_with("agent-") || !file_name.ends_with(".jsonl") {
                continue;
            }
            
            // Parse first few lines to get session info
            if let Ok((session_id, working_dir, updated_at)) = parse_session_metadata(&file_path) {
                sessions.push((session_id, working_dir, updated_at));
            }
        }
    }
    
    Ok(sessions)
}

fn parse_session_metadata(file_path: &PathBuf) -> Result<(String, PathBuf, DateTime<Utc>)> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    
    let mut session_id = None;
    let mut working_dir = None;
    let mut latest_timestamp = None;
    
    for line in reader.lines() {
        let line = line?;
        if let Ok(entry) = serde_json::from_str::<ClaudeJsonLine>(&line) {
            if session_id.is_none() {
                session_id = entry.session_id;
            }
            if working_dir.is_none() {
                working_dir = entry.cwd;
            }
            if let Some(ts) = entry.timestamp {
                if let Ok(dt) = DateTime::parse_from_rfc3339(&ts) {
                    latest_timestamp = Some(dt.with_timezone(&Utc));
                }
            }
            
            // Break early if we have what we need
            if session_id.is_some() && working_dir.is_some() {
                // Keep going to get latest timestamp
            }
        }
    }
    
    let session_id = session_id.ok_or_else(|| anyhow::anyhow!("No session ID found"))?;
    let working_dir = working_dir.ok_or_else(|| anyhow::anyhow!("No working dir found"))?;
    let updated_at = latest_timestamp.unwrap_or_else(Utc::now);
    
    Ok((session_id, PathBuf::from(working_dir), updated_at))
}

pub fn load_claude_session(session_id: &str) -> Result<crate::conversation::Conversation> {
    // Find the session file
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home dir"))?;
    let projects_dir = home.join(".claude").join("projects");
    
    // Search for the session file
    for entry in std::fs::read_dir(projects_dir)? {
        let entry = entry?;
        let project_path = entry.path();
        
        if !project_path.is_dir() {
            continue;
        }
        
        for file_entry in std::fs::read_dir(&project_path)? {
            let file_entry = file_entry?;
            let file_path = file_entry.path();
            
            if !file_path.is_file() {
                continue;
            }
            
            let file_name = file_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            
            // Check if this file contains our session
            if file_name.contains(session_id) && file_name.ends_with(".jsonl") {
                return parse_conversation(&file_path);
            }
        }
    }
    
    Err(anyhow::anyhow!("Session not found"))
}

fn parse_conversation(file_path: &PathBuf) -> Result<crate::conversation::Conversation> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};
    use crate::conversation::message::{Message, MessageContent};
    use rmcp::model::Role;
    
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    
    let mut messages = Vec::new();
    
    for line in reader.lines() {
        let line = line?;
        if let Ok(entry) = serde_json::from_str::<ClaudeJsonLine>(&line) {
            // Only process user and assistant messages
            if entry.entry_type.as_deref() == Some("user") {
                if let Some(msg) = entry.message {
                    if let Some(role_str) = msg.role {
                        if role_str == "user" {
                            // Parse user message
                            let content = parse_message_content(&msg.content);
                            if !content.is_empty() {
                                let timestamp = entry.timestamp
                                    .and_then(|ts| DateTime::parse_from_rfc3339(&ts).ok())
                                    .map(|dt| dt.with_timezone(&Utc).timestamp_millis())
                                    .unwrap_or_else(|| Utc::now().timestamp_millis());
                                
                                messages.push(Message::new(Role::User, timestamp, content));
                            }
                        }
                    }
                }
            } else if entry.entry_type.as_deref() == Some("assistant") {
                if let Some(msg) = entry.message {
                    if let Some(role_str) = msg.role {
                        if role_str == "assistant" {
                            // Parse assistant message
                            let content = parse_message_content(&msg.content);
                            if !content.is_empty() {
                                let timestamp = entry.timestamp
                                    .and_then(|ts| DateTime::parse_from_rfc3339(&ts).ok())
                                    .map(|dt| dt.with_timezone(&Utc).timestamp_millis())
                                    .unwrap_or_else(|| Utc::now().timestamp_millis());
                                
                                messages.push(Message::new(Role::Assistant, timestamp, content));
                            }
                        }
                    }
                }
            }
        }
    }
    
    Ok(crate::conversation::Conversation::new_unvalidated(messages))
}

fn parse_message_content(content_value: &serde_json::Value) -> Vec<MessageContent> {
    use crate::conversation::message::MessageContent;
    
    let mut result = Vec::new();
    
    // Handle string content (simple message)
    if let Some(text) = content_value.as_str() {
        result.push(MessageContent::text(text.to_string()));
        return result;
    }
    
    // Handle array content (structured message)
    if let Some(content_array) = content_value.as_array() {
        for item in content_array {
            if let Some(obj) = item.as_object() {
                if let Some(content_type) = obj.get("type").and_then(|v| v.as_str()) {
                    match content_type {
                        "text" => {
                            if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                                result.push(MessageContent::text(text.to_string()));
                            }
                        }
                        "tool_result" => {
                            if let Some(content) = obj.get("content").and_then(|v| v.as_str()) {
                                result.push(MessageContent::text(format!("[Tool Result]\n{}", content)));
                            }
                        }
                        // Skip thinking, tool_use for now - just get the conversational content
                        _ => {}
                    }
                }
            }
        }
    }
    
    result
}
```

### 3. Integrate into SessionManager

```rust
// In session_manager.rs

impl SessionStorage {
    async fn list_sessions(&self) -> Result<Vec<Session>> {
        // Get native goose sessions
        let mut sessions = sqlx::query_as::<_, Session>(
            r#"
            SELECT s.id, s.working_dir, s.name, s.description, s.user_set_name, s.session_type, 
                   s.created_at, s.updated_at, s.extension_data,
                   s.total_tokens, s.input_tokens, s.output_tokens,
                   s.accumulated_total_tokens, s.accumulated_input_tokens, s.accumulated_output_tokens,
                   s.schedule_id, s.recipe_json, s.user_recipe_values_json,
                   COUNT(m.id) as message_count
            FROM sessions s
            INNER JOIN messages m ON s.id = m.session_id
            WHERE s.session_type = 'user' OR s.session_type = 'scheduled'
            GROUP BY s.id
            ORDER BY s.updated_at DESC
        "#,
        )
        .fetch_all(&self.pool)
        .await?;
        
        // Add Claude sessions (if available)
        if let Ok(claude_sessions) = crate::session::claude_desktop::list_claude_sessions() {
            for (session_id, working_dir, updated_at) in claude_sessions {
                // Create a session object that looks like a goose session
                let session = Session {
                    id: session_id.clone(),
                    working_dir,
                    name: format!("Claude Session {}", &session_id[..8]), // Use first 8 chars of UUID
                    user_set_name: false,
                    session_type: SessionType::User,
                    created_at: updated_at, // Use updated as created
                    updated_at,
                    extension_data: ExtensionData::default(),
                    total_tokens: None,
                    input_tokens: None,
                    output_tokens: None,
                    accumulated_total_tokens: None,
                    accumulated_input_tokens: None,
                    accumulated_output_tokens: None,
                    schedule_id: None,
                    recipe: None,
                    user_recipe_values: None,
                    conversation: None,
                    message_count: 0, // We'd need to count, but skip for listing
                    source: Some("claude".to_string()),
                    external_id: None,
                };
                sessions.push(session);
            }
        }
        
        // Re-sort by updated_at
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        
        Ok(sessions)
    }
    
    async fn get_session(&self, id: &str, include_messages: bool) -> Result<Session> {
        // Try to get from database first
        let mut session = sqlx::query_as::<_, Session>(
            r#"
            SELECT id, working_dir, name, description, user_set_name, session_type, created_at, updated_at, extension_data,
                   total_tokens, input_tokens, output_tokens,
                   accumulated_total_tokens, accumulated_input_tokens, accumulated_output_tokens,
                   schedule_id, recipe_json, user_recipe_values_json
            FROM sessions
            WHERE id = ?
        "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        
        // If not found in database, try Claude sessions
        if session.is_none() {
            // This is a Claude session
            if let Ok(claude_sessions) = crate::session::claude_desktop::list_claude_sessions() {
                for (session_id, working_dir, updated_at) in claude_sessions {
                    if session_id == id {
                        let conversation = if include_messages {
                            Some(crate::session::claude_desktop::load_claude_session(id)?)
                        } else {
                            None
                        };
                        
                        let message_count = conversation.as_ref()
                            .map(|c| c.messages().len())
                            .unwrap_or(0);
                        
                        session = Some(Session {
                            id: session_id.clone(),
                            working_dir,
                            name: format!("Claude Session {}", &session_id[..8]),
                            user_set_name: false,
                            session_type: SessionType::User,
                            created_at: updated_at,
                            updated_at,
                            extension_data: ExtensionData::default(),
                            total_tokens: None,
                            input_tokens: None,
                            output_tokens: None,
                            accumulated_total_tokens: None,
                            accumulated_input_tokens: None,
                            accumulated_output_tokens: None,
                            schedule_id: None,
                            recipe: None,
                            user_recipe_values: None,
                            conversation,
                            message_count,
                            source: Some("claude".to_string()),
                            external_id: None,
                        });
                        break;
                    }
                }
            }
        }
        
        let mut session = session.ok_or_else(|| anyhow::anyhow!("Session not found"))?;
        
        // Load messages for goose sessions
        if session.source.is_none() {
            if include_messages {
                let conv = self.get_conversation(&session.id).await?;
                session.message_count = conv.messages().len();
                session.conversation = Some(conv);
            } else {
                let count = sqlx::query_scalar::<_, i64>(
                    "SELECT COUNT(*) FROM messages WHERE session_id = ?"
                )
                .bind(&session.id)
                .fetch_one(&self.pool)
                .await? as usize;
                session.message_count = count;
            }
        }
        
        Ok(session)
    }
}
```

### 4. Add to mod.rs

```rust
// In crates/goose/src/session/mod.rs
mod claude_desktop;
```

### 5. Resume Works Automatically!

When user selects a Claude session and sends a message:
1. Session loads with full conversation history
2. Agent gets the conversation context
3. New message appended via normal goose flow
4. **New messages saved to goose database, not Claude files**

This means:
- Claude sessions become "live" goose sessions once you resume
- Original Claude files never modified
- Going forward it's just a regular goose session

## Benefits of This Simple Approach

1. **No UI changes needed** - sessions just appear in the list
2. **Resume works out of the box** - goose doesn't care where conversation came from
3. **No import/export** - just read on demand
4. **No special read-only mode** - everything just works
5. **Graceful** - if Claude Desktop not installed, nothing breaks
6. **~200 lines of code** - mostly parsing JSONL

## Edge Cases Handled

1. **No Claude Desktop**: `list_claude_sessions()` returns empty vec, no error
2. **Invalid JSONL**: Skip that file, continue with others
3. **Resume overwrites**: Once you resume, new messages go to goose DB
4. **ID conflicts**: Claude uses UUIDs, goose uses `YYYYMMDD_N` - no overlap

## What We Skip (For Now)

- Thinking blocks (just show text messages)
- Tool use details (show as text)
- Token counting (leave empty)
- Agent sidechains (ignore agent-*.jsonl files)
- Session naming (use simple "Claude Session {id}" until first message)

## Testing

```bash
# Ensure it compiles
cargo build

# List sessions (should show Claude + goose)
# Via CLI or server endpoint

# Open a Claude session
# Send a message - should work like normal goose session
```

## Migration/Schema Changes

**None needed!** 
- `source` and `external_id` fields can be added as optional to Session struct
- Serde handles missing fields gracefully
- No database migrations required

## Implementation Time

Estimate: **2-4 hours**
- 1 hour: claude_desktop.rs parser
- 1 hour: SessionStorage integration  
- 1 hour: Testing and debugging
- 1 hour: Buffer

This is dead simple compared to the full spec!
