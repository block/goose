use etcetera::{choose_app_strategy, AppStrategy};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Process information stored in the PID tracking file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub command: String,
    pub timestamp: u64,
}

/// File-based PID tracker that persists process information across different
/// parts of the application (CLI, session, MCP server)
#[derive(Debug)]
pub struct FilePidTracker {
    file_path: PathBuf,
}

impl FilePidTracker {
    pub fn new() -> Self {
        // Use the same app strategy as the rest of the application
        let file_path = choose_app_strategy(crate::APP_STRATEGY.clone())
            .map(|strategy| strategy.in_data_dir("tracked_pids.json"))
            .unwrap_or_else(|_| {
                PathBuf::from(
                    shellexpand::tilde("~/.local/share/goose/tracked_pids.json").to_string(),
                )
            });

        // Create the directory if it doesn't exist
        if let Some(parent) = file_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        Self { file_path }
    }

    /// Read PIDs from the JSON file
    fn read_pids(&self) -> HashMap<String, ProcessInfo> {
        if !self.file_path.exists() {
            return HashMap::new();
        }

        match fs::read_to_string(&self.file_path) {
            Ok(content) => match serde_json::from_str::<HashMap<String, ProcessInfo>>(&content) {
                Ok(pids) => pids,
                Err(_) => HashMap::new(),
            },
            Err(_) => HashMap::new(),
        }
    }

    /// Write PIDs to the JSON file
    fn write_pids(&self, pids: &HashMap<String, ProcessInfo>) {
        match serde_json::to_string_pretty(pids) {
            Ok(content) => {
                let _ = fs::write(&self.file_path, content);
            }
            Err(_) => {}
        }
    }

    /// Register a process PID with execution ID and command
    pub fn register_process(&self, execution_id: String, pid: u32, command: String) {
        let mut pids = self.read_pids();
        let process_info = ProcessInfo {
            pid,
            command,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        pids.insert(execution_id.clone(), process_info);
        self.write_pids(&pids);
    }

    /// Unregister a process PID by execution ID
    pub fn unregister_process(&self, execution_id: &str) -> Option<u32> {
        let mut pids = self.read_pids();
        let removed = pids.remove(execution_id);
        self.write_pids(&pids);
        if let Some(ref process_info) = removed {
            Some(process_info.pid)
        } else {
            None
        }
    }

    /// Get all currently tracked PIDs
    pub fn get_all_pids(&self) -> Vec<u32> {
        let pids = self.read_pids();
        pids.values().map(|info| info.pid).collect()
    }

    /// Clear all tracked PIDs
    pub fn clear_all(&self) {
        let empty_pids: HashMap<String, ProcessInfo> = HashMap::new();
        self.write_pids(&empty_pids);
    }

    /// Clean up old PIDs (older than 1 hour) to prevent the file from growing indefinitely
    pub fn cleanup_old_pids(&self) {
        let mut pids = self.read_pids();
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let one_hour = 3600; // 1 hour in seconds
        let initial_count = pids.len();

        pids.retain(|_, info| current_time - info.timestamp < one_hour);

        if pids.len() != initial_count {
            self.write_pids(&pids);
        }
    }
}

impl Default for FilePidTracker {
    fn default() -> Self {
        Self::new()
    }
}
