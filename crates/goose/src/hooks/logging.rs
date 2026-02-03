//! Hook Logging - Per-hook logging with correlation IDs

use super::events::HookEvent;
use super::handlers::HookResult;
use super::manager::HookStats;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;

/// Entry in the hook log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookLogEntry {
    pub timestamp: DateTime<Utc>,
    pub run_id: String,
    pub event_id: String,
    pub hook_event_name: String,
    pub phase: LogPhase,
    pub session_id: String,
    pub tool_name: Option<String>,
    pub duration_ms: Option<u64>,
    pub exit_code: Option<i32>,
    pub blocked: Option<bool>,
    pub error: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Phase of the hook execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogPhase {
    Start,
    End,
    Handler,
    Error,
}

/// Hook logger for writing per-hook log files
pub struct HookLogger {
    run_id: String,
    log_dir: PathBuf,
    stats: Arc<RwLock<HookStats>>,
}

impl HookLogger {
    pub fn new(run_id: impl Into<String>, log_dir: impl Into<PathBuf>) -> Self {
        let run_id = run_id.into();
        let log_dir = log_dir.into();
        Self {
            run_id,
            log_dir,
            stats: Arc::new(RwLock::new(HookStats::default())),
        }
    }

    fn get_log_path(&self, hook_name: &str) -> PathBuf {
        self.log_dir
            .join("runs")
            .join(&self.run_id)
            .join("hooks")
            .join(format!("{}.jsonl", hook_name.to_lowercase()))
    }

    fn get_human_log_path(&self, hook_name: &str) -> PathBuf {
        self.log_dir
            .join("runs")
            .join(&self.run_id)
            .join("hooks")
            .join(format!("{}.log", hook_name.to_lowercase()))
    }

    async fn ensure_log_dir(&self, path: &PathBuf) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        Ok(())
    }

    async fn append_jsonl(&self, path: &PathBuf, entry: &HookLogEntry) -> std::io::Result<()> {
        self.ensure_log_dir(path).await?;

        let json = serde_json::to_string(entry)?;
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;

        file.write_all(json.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;

        Ok(())
    }

    async fn append_human_log(&self, path: &PathBuf, message: &str) -> std::io::Result<()> {
        self.ensure_log_dir(path).await?;

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;

        file.write_all(message.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;

        Ok(())
    }

    /// Log the start of a hook event
    pub async fn log_event_start(&self, event: &HookEvent) -> std::io::Result<()> {
        let event_name = event.event_name();
        let event_id = uuid::Uuid::new_v4().to_string();

        let entry = HookLogEntry {
            timestamp: Utc::now(),
            run_id: self.run_id.clone(),
            event_id: event_id.clone(),
            hook_event_name: event_name.to_string(),
            phase: LogPhase::Start,
            session_id: event.session_id().to_string(),
            tool_name: event.tool_name().map(String::from),
            duration_ms: None,
            exit_code: None,
            blocked: None,
            error: None,
            metadata: HashMap::new(),
        };

        let path = self.get_log_path(event_name);
        self.append_jsonl(&path, &entry).await?;

        // Human-readable log
        let human_path = self.get_human_log_path(event_name);
        let human_msg = format!(
            "[{}] {} START event_id={} session={}{}",
            entry.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
            event_name,
            &event_id[..8],
            &entry.session_id[..8.min(entry.session_id.len())],
            entry
                .tool_name
                .as_ref()
                .map(|t| format!(" tool={}", t))
                .unwrap_or_default()
        );
        self.append_human_log(&human_path, &human_msg).await?;

        // Update stats
        let mut stats = self.stats.write().await;
        stats.total_executions += 1;
        *stats.by_event.entry(event_name.to_string()).or_insert(0) += 1;

        Ok(())
    }

    /// Log a handler result
    pub async fn log_handler_result(
        &self,
        event: &HookEvent,
        result: &HookResult,
    ) -> std::io::Result<()> {
        let event_name = event.event_name();
        let event_id = uuid::Uuid::new_v4().to_string();

        let entry = HookLogEntry {
            timestamp: Utc::now(),
            run_id: self.run_id.clone(),
            event_id: event_id.clone(),
            hook_event_name: event_name.to_string(),
            phase: LogPhase::Handler,
            session_id: event.session_id().to_string(),
            tool_name: event.tool_name().map(String::from),
            duration_ms: Some(result.duration_ms),
            exit_code: Some(result.exit_code),
            blocked: Some(result.should_block()),
            error: if result.stderr.is_empty() {
                None
            } else {
                Some(result.stderr.clone())
            },
            metadata: HashMap::new(),
        };

        let path = self.get_log_path(event_name);
        self.append_jsonl(&path, &entry).await?;

        // Human-readable log
        let human_path = self.get_human_log_path(event_name);
        let status = if result.should_block() {
            "BLOCKED"
        } else if result.is_success() {
            "OK"
        } else {
            "ERROR"
        };
        let human_msg = format!(
            "[{}] {} HANDLER {} exit={} duration={}ms{}",
            entry.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
            event_name,
            status,
            result.exit_code,
            result.duration_ms,
            if result.timed_out { " TIMEOUT" } else { "" }
        );
        self.append_human_log(&human_path, &human_msg).await?;

        // Update stats
        let mut stats = self.stats.write().await;
        if result.is_success() {
            stats.successful += 1;
        } else if result.should_block() {
            stats.blocked += 1;
        } else {
            stats.failed += 1;
        }
        if result.timed_out {
            stats.timed_out += 1;
        }

        Ok(())
    }

    /// Log the end of a hook event
    pub async fn log_event_end(
        &self,
        event: &HookEvent,
        results: &[HookResult],
    ) -> std::io::Result<()> {
        let event_name = event.event_name();
        let event_id = uuid::Uuid::new_v4().to_string();

        let total_duration: u64 = results.iter().map(|r| r.duration_ms).sum();
        let any_blocked = results.iter().any(|r| r.should_block());

        let entry = HookLogEntry {
            timestamp: Utc::now(),
            run_id: self.run_id.clone(),
            event_id: event_id.clone(),
            hook_event_name: event_name.to_string(),
            phase: LogPhase::End,
            session_id: event.session_id().to_string(),
            tool_name: event.tool_name().map(String::from),
            duration_ms: Some(total_duration),
            exit_code: None,
            blocked: Some(any_blocked),
            error: None,
            metadata: {
                let mut m = HashMap::new();
                m.insert("handler_count".to_string(), results.len().to_string());
                m
            },
        };

        let path = self.get_log_path(event_name);
        self.append_jsonl(&path, &entry).await?;

        // Human-readable log
        let human_path = self.get_human_log_path(event_name);
        let human_msg = format!(
            "[{}] {} END handlers={} total_duration={}ms blocked={}",
            entry.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
            event_name,
            results.len(),
            total_duration,
            any_blocked
        );
        self.append_human_log(&human_path, &human_msg).await?;

        Ok(())
    }

    /// Log an error
    pub async fn log_error(&self, event_name: &str, error: &str) -> std::io::Result<()> {
        let entry = HookLogEntry {
            timestamp: Utc::now(),
            run_id: self.run_id.clone(),
            event_id: uuid::Uuid::new_v4().to_string(),
            hook_event_name: event_name.to_string(),
            phase: LogPhase::Error,
            session_id: String::new(),
            tool_name: None,
            duration_ms: None,
            exit_code: None,
            blocked: None,
            error: Some(error.to_string()),
            metadata: HashMap::new(),
        };

        let path = self.get_log_path(event_name);
        self.append_jsonl(&path, &entry).await?;

        let human_path = self.get_human_log_path(event_name);
        let human_msg = format!(
            "[{}] {} ERROR: {}",
            entry.timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
            event_name,
            error
        );
        self.append_human_log(&human_path, &human_msg).await?;

        Ok(())
    }

    /// Get current statistics
    pub async fn get_stats(&self) -> HookStats {
        self.stats.read().await.clone()
    }

    /// Get the log directory for this run
    pub fn log_dir(&self) -> PathBuf {
        self.log_dir.join("runs").join(&self.run_id).join("hooks")
    }

    /// Create a run index file listing all hook logs
    pub async fn create_run_index(&self) -> std::io::Result<()> {
        let index_path = self
            .log_dir
            .join("runs")
            .join(&self.run_id)
            .join("run_index.json");

        self.ensure_log_dir(&index_path).await?;

        let stats = self.stats.read().await;
        let index = serde_json::json!({
            "run_id": self.run_id,
            "timestamp": Utc::now().to_rfc3339(),
            "stats": *stats,
            "log_dir": self.log_dir().display().to_string(),
        });

        let content = serde_json::to_string_pretty(&index)?;
        tokio::fs::write(&index_path, content).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_log_entry_serialization() {
        let entry = HookLogEntry {
            timestamp: Utc::now(),
            run_id: "run-1".to_string(),
            event_id: "event-1".to_string(),
            hook_event_name: "PreToolUse".to_string(),
            phase: LogPhase::Start,
            session_id: "session-1".to_string(),
            tool_name: Some("Bash".to_string()),
            duration_ms: None,
            exit_code: None,
            blocked: None,
            error: None,
            metadata: HashMap::new(),
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("PreToolUse"));
        assert!(json.contains("start"));
    }

    #[test]
    fn test_log_phase_serialization() {
        let json = serde_json::to_string(&LogPhase::Start).unwrap();
        assert_eq!(json, "\"start\"");

        let phase: LogPhase = serde_json::from_str("\"end\"").unwrap();
        assert_eq!(phase, LogPhase::End);
    }

    #[tokio::test]
    async fn test_hook_logger_stats() {
        let logger = HookLogger::new("test-run", std::env::temp_dir());

        let stats = logger.get_stats().await;
        assert_eq!(stats.total_executions, 0);
    }
}
