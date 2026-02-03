//! MCP Gateway Audit Logging
//!
//! Comprehensive audit logging for MCP operations.

use super::permissions::UserContextSnapshot;
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;

/// Audit event type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    /// Tool execution started
    ToolExecutionStart,
    /// Tool execution completed successfully
    ToolExecutionSuccess,
    /// Tool execution failed
    ToolExecutionFailure,
    /// Permission was denied
    PermissionDenied,
    /// Server connection error
    ServerConnectionError,
    /// Credential was accessed
    CredentialAccess,
    /// Policy was evaluated
    PolicyEvaluation,
    /// Server registered
    ServerRegistered,
    /// Server unregistered
    ServerUnregistered,
    /// Configuration changed
    ConfigurationChanged,
}

/// Audit request information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRequest {
    /// Tool name
    pub tool_name: String,
    /// Tool arguments (may be redacted)
    pub arguments: serde_json::Value,
    /// SHA-256 hash of arguments for privacy-preserving audit
    pub argument_hash: String,
}

impl AuditRequest {
    /// Create from tool call
    pub fn new(tool_name: &str, arguments: &serde_json::Value, redact: bool) -> Self {
        let argument_hash = Self::hash_arguments(arguments);
        let arguments = if redact {
            Self::redact_arguments(arguments)
        } else {
            arguments.clone()
        };

        Self {
            tool_name: tool_name.to_string(),
            arguments,
            argument_hash,
        }
    }

    /// Hash arguments for privacy-preserving audit
    fn hash_arguments(arguments: &serde_json::Value) -> String {
        let mut hasher = Sha256::new();
        hasher.update(arguments.to_string().as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Redact sensitive fields from arguments
    fn redact_arguments(arguments: &serde_json::Value) -> serde_json::Value {
        match arguments {
            serde_json::Value::Object(map) => {
                let mut redacted = serde_json::Map::new();
                for (key, value) in map {
                    let key_lower = key.to_lowercase();
                    if key_lower.contains("password")
                        || key_lower.contains("secret")
                        || key_lower.contains("token")
                        || key_lower.contains("key")
                        || key_lower.contains("credential")
                    {
                        redacted.insert(
                            key.clone(),
                            serde_json::Value::String("[REDACTED]".to_string()),
                        );
                    } else {
                        redacted.insert(key.clone(), Self::redact_arguments(value));
                    }
                }
                serde_json::Value::Object(redacted)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(Self::redact_arguments).collect())
            }
            other => other.clone(),
        }
    }
}

/// Audit response information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditResponse {
    /// Whether execution was successful
    pub success: bool,
    /// Result size in bytes
    pub result_size_bytes: usize,
    /// Error type if failed
    pub error_type: Option<String>,
    /// Error message if failed
    pub error_message: Option<String>,
}

/// Complete audit entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Unique entry identifier
    pub id: Uuid,
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Event type
    pub event_type: AuditEventType,
    /// User context snapshot
    pub user_context: UserContextSnapshot,
    /// Tool name
    pub tool_name: String,
    /// Server ID
    pub server_id: String,
    /// Request information
    pub request: Option<AuditRequest>,
    /// Response information
    pub response: Option<AuditResponse>,
    /// Duration in milliseconds
    pub duration_ms: Option<u64>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl AuditEntry {
    /// Create a new audit entry
    pub fn new(
        event_type: AuditEventType,
        user_context: UserContextSnapshot,
        tool_name: &str,
        server_id: &str,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type,
            user_context,
            tool_name: tool_name.to_string(),
            server_id: server_id.to_string(),
            request: None,
            response: None,
            duration_ms: None,
            metadata: HashMap::new(),
        }
    }

    /// Add request information
    pub fn with_request(mut self, request: AuditRequest) -> Self {
        self.request = Some(request);
        self
    }

    /// Add response information
    pub fn with_response(mut self, response: AuditResponse) -> Self {
        self.response = Some(response);
        self
    }

    /// Add duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }
}

/// Audit storage trait
#[async_trait]
pub trait AuditStorage: Send + Sync {
    /// Store an audit entry
    async fn store(&self, entry: AuditEntry) -> Result<()>;

    /// Query audit entries
    async fn query(&self, query: AuditQuery) -> Result<Vec<AuditEntry>>;

    /// Get entry by ID
    async fn get(&self, id: Uuid) -> Result<Option<AuditEntry>>;

    /// Delete old entries (retention)
    async fn cleanup(&self, before: DateTime<Utc>) -> Result<usize>;
}

/// Audit query parameters
#[derive(Debug, Clone, Default)]
pub struct AuditQuery {
    /// Filter by user ID
    pub user_id: Option<String>,
    /// Filter by tool name
    pub tool_name: Option<String>,
    /// Filter by server ID
    pub server_id: Option<String>,
    /// Filter by event type
    pub event_type: Option<AuditEventType>,
    /// Start time
    pub start_time: Option<DateTime<Utc>>,
    /// End time
    pub end_time: Option<DateTime<Utc>>,
    /// Limit results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

impl AuditQuery {
    /// Create new query
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by user
    pub fn user(mut self, user_id: &str) -> Self {
        self.user_id = Some(user_id.to_string());
        self
    }

    /// Filter by tool
    pub fn tool(mut self, tool_name: &str) -> Self {
        self.tool_name = Some(tool_name.to_string());
        self
    }

    /// Filter by server
    pub fn server(mut self, server_id: &str) -> Self {
        self.server_id = Some(server_id.to_string());
        self
    }

    /// Filter by event type
    pub fn event_type(mut self, event_type: AuditEventType) -> Self {
        self.event_type = Some(event_type);
        self
    }

    /// Filter by time range
    pub fn time_range(mut self, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        self.start_time = Some(start);
        self.end_time = Some(end);
        self
    }

    /// Limit results
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set offset
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
}

/// In-memory audit storage
pub struct MemoryAuditStorage {
    entries: Mutex<Vec<AuditEntry>>,
    max_entries: usize,
}

impl MemoryAuditStorage {
    /// Create new in-memory storage
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
            max_entries,
        }
    }
}

impl Default for MemoryAuditStorage {
    fn default() -> Self {
        Self::new(10000)
    }
}

#[async_trait]
impl AuditStorage for MemoryAuditStorage {
    async fn store(&self, entry: AuditEntry) -> Result<()> {
        let mut entries = self.entries.lock().await;
        entries.push(entry);

        // Trim if over limit
        if entries.len() > self.max_entries {
            let drain_count = entries.len() - self.max_entries;
            entries.drain(0..drain_count);
        }

        Ok(())
    }

    async fn query(&self, query: AuditQuery) -> Result<Vec<AuditEntry>> {
        let entries = self.entries.lock().await;

        let filtered: Vec<AuditEntry> = entries
            .iter()
            .filter(|e| {
                if let Some(ref user_id) = query.user_id {
                    if e.user_context.user_id != *user_id {
                        return false;
                    }
                }
                if let Some(ref tool_name) = query.tool_name {
                    if e.tool_name != *tool_name {
                        return false;
                    }
                }
                if let Some(ref server_id) = query.server_id {
                    if e.server_id != *server_id {
                        return false;
                    }
                }
                if let Some(event_type) = query.event_type {
                    if e.event_type != event_type {
                        return false;
                    }
                }
                if let Some(start_time) = query.start_time {
                    if e.timestamp < start_time {
                        return false;
                    }
                }
                if let Some(end_time) = query.end_time {
                    if e.timestamp > end_time {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        // Apply pagination
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(usize::MAX);

        Ok(filtered.into_iter().skip(offset).take(limit).collect())
    }

    async fn get(&self, id: Uuid) -> Result<Option<AuditEntry>> {
        let entries = self.entries.lock().await;
        Ok(entries.iter().find(|e| e.id == id).cloned())
    }

    async fn cleanup(&self, before: DateTime<Utc>) -> Result<usize> {
        let mut entries = self.entries.lock().await;
        let initial_len = entries.len();
        entries.retain(|e| e.timestamp >= before);
        Ok(initial_len - entries.len())
    }
}

/// Audit logger
pub struct AuditLogger {
    storage: Arc<dyn AuditStorage>,
    buffer: Mutex<Vec<AuditEntry>>,
    buffer_size: usize,
    redact_arguments: bool,
}

impl AuditLogger {
    /// Create new audit logger
    pub fn new(storage: Arc<dyn AuditStorage>) -> Self {
        Self {
            storage,
            buffer: Mutex::new(Vec::new()),
            buffer_size: 100,
            redact_arguments: true,
        }
    }

    /// Create with in-memory storage
    pub fn memory() -> Self {
        Self::new(Arc::new(MemoryAuditStorage::default()))
    }

    /// Set buffer size
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    /// Set argument redaction
    pub fn with_redaction(mut self, redact: bool) -> Self {
        self.redact_arguments = redact;
        self
    }

    /// Log an audit entry
    pub async fn log(&self, entry: AuditEntry) -> Result<()> {
        let mut buffer = self.buffer.lock().await;
        buffer.push(entry);

        if buffer.len() >= self.buffer_size {
            self.flush_buffer(&mut buffer).await?;
        }

        Ok(())
    }

    /// Flush buffer to storage
    async fn flush_buffer(&self, buffer: &mut Vec<AuditEntry>) -> Result<()> {
        for entry in buffer.drain(..) {
            self.storage.store(entry).await?;
        }
        Ok(())
    }

    /// Force flush
    pub async fn flush(&self) -> Result<()> {
        let mut buffer = self.buffer.lock().await;
        self.flush_buffer(&mut buffer).await
    }

    /// Create entry for tool execution start
    pub fn start_execution(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
        user_context: &UserContextSnapshot,
        server_id: &str,
    ) -> AuditEntry {
        let request = AuditRequest::new(tool_name, arguments, self.redact_arguments);

        AuditEntry::new(
            AuditEventType::ToolExecutionStart,
            user_context.clone(),
            tool_name,
            server_id,
        )
        .with_request(request)
    }

    /// Complete entry with success
    pub fn complete_success(
        &self,
        mut entry: AuditEntry,
        result_size: usize,
        duration_ms: u64,
    ) -> AuditEntry {
        entry.event_type = AuditEventType::ToolExecutionSuccess;
        entry.response = Some(AuditResponse {
            success: true,
            result_size_bytes: result_size,
            error_type: None,
            error_message: None,
        });
        entry.duration_ms = Some(duration_ms);
        entry
    }

    /// Complete entry with failure
    pub fn complete_failure(
        &self,
        mut entry: AuditEntry,
        error_type: &str,
        error_message: &str,
        duration_ms: u64,
    ) -> AuditEntry {
        entry.event_type = AuditEventType::ToolExecutionFailure;
        entry.response = Some(AuditResponse {
            success: false,
            result_size_bytes: 0,
            error_type: Some(error_type.to_string()),
            error_message: Some(error_message.to_string()),
        });
        entry.duration_ms = Some(duration_ms);
        entry
    }

    /// Log permission denied
    pub async fn log_permission_denied(
        &self,
        tool_name: &str,
        user_context: &UserContextSnapshot,
        reason: &str,
    ) -> Result<()> {
        let entry = AuditEntry::new(
            AuditEventType::PermissionDenied,
            user_context.clone(),
            tool_name,
            "",
        )
        .with_metadata("reason", serde_json::json!(reason));

        self.log(entry).await
    }

    /// Query audit log
    pub async fn query(&self, query: AuditQuery) -> Result<Vec<AuditEntry>> {
        // Flush buffer first
        self.flush().await?;
        self.storage.query(query).await
    }

    /// Get entry by ID
    pub async fn get(&self, id: Uuid) -> Result<Option<AuditEntry>> {
        self.flush().await?;
        self.storage.get(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp_gateway::permissions::UserContext;

    fn test_user_context() -> UserContextSnapshot {
        UserContext::new("test_user")
            .with_group("developers")
            .snapshot()
    }

    #[tokio::test]
    async fn test_audit_logger_basic() {
        let logger = AuditLogger::memory().with_buffer_size(1);

        let entry = AuditEntry::new(
            AuditEventType::ToolExecutionSuccess,
            test_user_context(),
            "test_tool",
            "server1",
        );

        logger.log(entry).await.unwrap();

        let entries = logger.query(AuditQuery::new()).await.unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[tokio::test]
    async fn test_audit_query_filters() {
        let logger = AuditLogger::memory().with_buffer_size(1);

        // Add entries for different users
        let entry1 = AuditEntry::new(
            AuditEventType::ToolExecutionSuccess,
            UserContext::new("user1").snapshot(),
            "tool1",
            "server1",
        );
        let entry2 = AuditEntry::new(
            AuditEventType::ToolExecutionSuccess,
            UserContext::new("user2").snapshot(),
            "tool2",
            "server1",
        );

        logger.log(entry1).await.unwrap();
        logger.log(entry2).await.unwrap();

        // Query for user1
        let entries = logger.query(AuditQuery::new().user("user1")).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].user_context.user_id, "user1");

        // Query for tool2
        let entries = logger.query(AuditQuery::new().tool("tool2")).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].tool_name, "tool2");
    }

    #[tokio::test]
    async fn test_audit_execution_flow() {
        let logger = AuditLogger::memory().with_buffer_size(1);
        let user_context = test_user_context();
        let arguments = serde_json::json!({"path": "/test"});

        // Start execution
        let entry = logger.start_execution("file_read", &arguments, &user_context, "server1");

        assert_eq!(entry.event_type, AuditEventType::ToolExecutionStart);
        assert!(entry.request.is_some());

        // Complete with success
        let completed = logger.complete_success(entry, 1024, 50);

        assert_eq!(completed.event_type, AuditEventType::ToolExecutionSuccess);
        assert!(completed.response.is_some());
        assert!(completed.response.as_ref().unwrap().success);
        assert_eq!(completed.duration_ms, Some(50));

        logger.log(completed).await.unwrap();

        let entries = logger.query(AuditQuery::new()).await.unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[tokio::test]
    async fn test_audit_permission_denied() {
        let logger = AuditLogger::memory().with_buffer_size(1);
        let user_context = test_user_context();

        logger
            .log_permission_denied("dangerous_tool", &user_context, "User not authorized")
            .await
            .unwrap();

        let entries = logger
            .query(AuditQuery::new().event_type(AuditEventType::PermissionDenied))
            .await
            .unwrap();

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].tool_name, "dangerous_tool");
    }

    #[test]
    fn test_argument_redaction() {
        let args = serde_json::json!({
            "path": "/test",
            "password": "secret123",
            "config": {
                "api_key": "key123",
                "timeout": 30
            }
        });

        let request = AuditRequest::new("test_tool", &args, true);

        // Check redaction
        let redacted = &request.arguments;
        assert_eq!(redacted["path"], "/test");
        assert_eq!(redacted["password"], "[REDACTED]");
        assert_eq!(redacted["config"]["api_key"], "[REDACTED]");
        assert_eq!(redacted["config"]["timeout"], 30);
    }

    #[test]
    fn test_argument_hash() {
        let args1 = serde_json::json!({"a": 1, "b": 2});
        let args2 = serde_json::json!({"a": 1, "b": 2});
        let args3 = serde_json::json!({"a": 1, "b": 3});

        let request1 = AuditRequest::new("tool", &args1, false);
        let request2 = AuditRequest::new("tool", &args2, false);
        let request3 = AuditRequest::new("tool", &args3, false);

        assert_eq!(request1.argument_hash, request2.argument_hash);
        assert_ne!(request1.argument_hash, request3.argument_hash);
    }

    #[tokio::test]
    async fn test_memory_storage_cleanup() {
        let storage = MemoryAuditStorage::new(100);

        // Add entries at different times
        let old_entry = AuditEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now() - chrono::Duration::days(30),
            event_type: AuditEventType::ToolExecutionSuccess,
            user_context: test_user_context(),
            tool_name: "old_tool".to_string(),
            server_id: "server1".to_string(),
            request: None,
            response: None,
            duration_ms: None,
            metadata: HashMap::new(),
        };

        let new_entry = AuditEntry {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type: AuditEventType::ToolExecutionSuccess,
            user_context: test_user_context(),
            tool_name: "new_tool".to_string(),
            server_id: "server1".to_string(),
            request: None,
            response: None,
            duration_ms: None,
            metadata: HashMap::new(),
        };

        storage.store(old_entry).await.unwrap();
        storage.store(new_entry).await.unwrap();

        // Cleanup entries older than 7 days
        let cutoff = Utc::now() - chrono::Duration::days(7);
        let cleaned = storage.cleanup(cutoff).await.unwrap();

        assert_eq!(cleaned, 1);

        let remaining = storage.query(AuditQuery::new()).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].tool_name, "new_tool");
    }
}
