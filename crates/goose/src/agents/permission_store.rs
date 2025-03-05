use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use anyhow::Result;
use etcetera::{choose_app_strategy, AppStrategy, AppStrategyArgs};
use std::{
    path::PathBuf,
    fs::File,
};
use crate::message::ToolRequest;
use chrono::Utc;
use std::time::Duration;
use blake3::Hasher;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolPermissionRecord {
    tool_name: String,
    allowed: bool,
    context_hash: String,  // Hash of the tool's arguments/context to differentiate similar calls
    timestamp: i64,
    expiry: Option<i64>,  // Optional expiry timestamp
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolPermissionStore {
    permissions: HashMap<String, Vec<ToolPermissionRecord>>,
    version: u32,  // For future schema migrations
    #[serde(skip)]  // Don't serialize this field
    config_dir: PathBuf,
}

impl ToolPermissionStore {
    pub fn new() -> Self {
        let config_dir = choose_app_strategy(crate::config::APP_STRATEGY.clone())
            .map(|strategy| strategy.config_dir())
            .unwrap_or_else(|_| PathBuf::from(".config/goose"));

        Self {
            permissions: HashMap::new(),
            version: 1,
            config_dir,
        }
    }

    pub fn load() -> Result<Self> {
        let store = Self::new();
        let file_path = store.config_dir.join("tool_permissions.json");

        if !file_path.exists() {
            return Ok(store);
        }

        let file = File::open(file_path)?;
        let mut permissions: ToolPermissionStore = serde_json::from_reader(file)?;
        permissions.config_dir = store.config_dir;
        Ok(permissions)
    }
    
    pub fn save(&self) -> anyhow::Result<()> {
        let config_dir = self.config_dir.clone();
        std::fs::create_dir_all(&config_dir)?;
        
        let path = config_dir.join("tool_permissions.json");
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn check_permission(&self, tool_request: &ToolRequest) -> Option<bool> {
        let context_hash = self.hash_tool_context(tool_request);
        let tool_call = tool_request.tool_call.as_ref().unwrap();
        let key = format!("{}:{}", tool_call.name, context_hash);
        
        self.permissions.get(&key).and_then(|records| {
            records
                .iter()
                .filter(|record| {
                    record.expiry.map_or(true, |exp| exp > Utc::now().timestamp())
                })
                .last()
                .map(|record| record.allowed)
        })
    }

    pub fn record_permission(
        &mut self, 
        tool_request: &ToolRequest, 
        allowed: bool,
        expiry_duration: Option<Duration>
    ) -> anyhow::Result<()> {
        let context_hash = self.hash_tool_context(tool_request);
        let tool_call = tool_request.tool_call.as_ref().unwrap();
        let key = format!("{}:{}", tool_call.name, context_hash);
        
        let record = ToolPermissionRecord {
            tool_name: tool_call.name.clone(),
            allowed,
            context_hash,
            timestamp: Utc::now().timestamp(),
            expiry: expiry_duration.map(|d| Utc::now().timestamp() + d.as_secs() as i64),
        };

        self.permissions
            .entry(key)
            .or_insert_with(Vec::new)
            .push(record);
            
        self.save()?;
        Ok(())
    }

    fn hash_tool_context(&self, tool_request: &ToolRequest) -> String {
        // Create a hash of the tool's arguments to differentiate similar calls
        // This helps identify when the same tool is being used in a different context
        let mut hasher = Hasher::new();
        hasher.update(serde_json::to_string(&tool_request.tool_call.as_ref().unwrap().arguments).unwrap_or_default().as_bytes());
        hasher.finalize().to_hex().to_string()
    }
}

#[cfg(test)]
mod tests {
    use crate::message::ToolRequest;
    use mcp_core::tool::ToolCall;
    use crate::agents::permission_store::ToolPermissionStore;
    use chrono::Utc;
    use std::time::Duration;

    fn create_test_tool_request(name: &str, args: serde_json::Value) -> ToolRequest {
        ToolRequest {
            id: "test-id".to_string(),
            tool_call: Ok(ToolCall {
                name: name.to_string(),
                arguments: args,
            }),
        }
    }

    #[test]
    fn test_permission_store_basic() {
        let mut store = ToolPermissionStore::new();
        let tool_request = create_test_tool_request(
            "test_tool",
            serde_json::json!({"arg1": "value1"}),
        );

        // Initially no permission recorded
        assert!(store.check_permission(&tool_request).is_none());

        // Record a permission
        store.record_permission(&tool_request, true, None).unwrap();

        // Should now find the recorded permission
        assert_eq!(store.check_permission(&tool_request), Some(true));
    }

    #[test]
    fn test_permission_expiry() {
        let mut store = ToolPermissionStore::new();
        let tool_request = create_test_tool_request(
            "test_tool",
            serde_json::json!({"arg1": "value1"}),
        );

        // Record a permission that expires in 1 second
        store.record_permission(&tool_request, true, Some(Duration::from_secs(1))).unwrap();

        // Should initially be allowed
        assert_eq!(store.check_permission(&tool_request), Some(true));

        // Manually set expiry to the past
        if let Some(records) = store.permissions.get_mut(&format!("{}:{}", 
            tool_request.tool_call.as_ref().unwrap().name,
            store.hash_tool_context(&tool_request))) {
            if let Some(record) = records.last_mut() {
                record.expiry = Some(Utc::now().timestamp() - 2);
            }
        }

        // Should now be expired (no permission found)
        assert!(store.check_permission(&tool_request).is_none());
    }

    #[test]
    fn test_different_arguments() {
        let mut store = ToolPermissionStore::new();
        
        // Create two requests with same tool but different args
        let request1 = create_test_tool_request(
            "test_tool",
            serde_json::json!({"arg": "value1"}),
        );
        let request2 = create_test_tool_request(
            "test_tool", 
            serde_json::json!({"arg": "value2"}),
        );

        // Record permission for first request
        store.record_permission(&request1, true, None).unwrap();

        // Should only find permission for first request
        assert_eq!(store.check_permission(&request1), Some(true));
        assert!(store.check_permission(&request2).is_none());
    }
}


