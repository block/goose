use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::transport::Error;
use mcp_core::protocol::{JsonRpcMessage, JsonRpcNotification, JsonRpcRequest};

/// Log levels supported by the MCP protocol
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Notice,
    Warning,
    Error,
    Critical,
    Alert,
    Emergency,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Notice => write!(f, "NOTICE"),
            LogLevel::Warning => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Critical => write!(f, "CRIT"),
            LogLevel::Alert => write!(f, "ALERT"),
            LogLevel::Emergency => write!(f, "EMERG"),
        }
    }
}

/// Logging capability for client/server capability negotiation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LoggingCapability {}

/// Parameters for setting the log level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetLevelParams {
    pub level: LogLevel,
}

/// Parameters for logging message notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingMessageParams {
    pub level: LogLevel,
    pub logger: Option<String>,
    pub data: Value,
}

/// A log message received from the server
#[derive(Debug, Clone)]
pub struct LogMessage {
    pub level: LogLevel,
    pub logger: Option<String>,
    pub message: String,
}

/// Handler type for log messages
pub type LogHandler = Box<dyn Fn(LogMessage) + Send + Sync>;

/// Manages logging state and handlers
#[derive(Clone)]
pub struct LoggingManager {
    handlers: Arc<RwLock<Vec<LogHandler>>>,
}

impl Default for LoggingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl LoggingManager {
    pub fn new() -> Self {
        Self {
            handlers: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Register a new handler for log messages
    pub async fn add_handler<F>(&self, handler: F)
    where
        F: Fn(LogMessage) + Send + Sync + 'static,
    {
        println!("Adding new log handler");
        self.handlers.write().await.push(Box::new(handler));
        let count = self.handlers.read().await.len();
        println!("Now have {} log handlers registered", count);
    }

    /// Handle a logging notification message
    pub async fn handle_notification(&self, notification: JsonRpcNotification) -> Result<(), Error> {
        println!("LoggingManager handling notification: {:?}", notification);
        // Parse notification parameters
        let params: LoggingMessageParams = serde_json::from_value(
            notification.params.ok_or_else(|| Error::UnsupportedMessage)?,
        )
        .map_err(|e| Error::Serialization(e))?;

        // Convert data to string - handle both string and structured data
        let message = match params.data {
            Value::String(s) => s,
            _ => serde_json::to_string(&params.data)
                .map_err(|e| Error::Serialization(e))?,
        };

        let log_message = LogMessage {
            level: params.level,
            logger: params.logger.clone(),
            message: message.clone(),
        };

        println!(
            "Created log message: level={:?}, logger={:?}, message={}",
            log_message.level,
            log_message.logger,
            log_message.message
        );

        let handler_count = self.handlers.read().await.len();
        println!("About to notify {} registered handlers", handler_count);

        // Notify all registered handlers
        for (idx, handler) in self.handlers.read().await.iter().enumerate() {
            println!("Calling handler {}", idx);
            handler(log_message.clone());
            println!("Handler {} completed", idx);
        }

        Ok(())
    }
}

/// Helper function to handle incoming notifications in the transport
pub async fn handle_notification(
    notification: JsonRpcNotification,
    logging_manager: &LoggingManager,
) -> Result<(), Error> {
    match notification.method.as_str() {
        "notifications/message" => {
            logging_manager.handle_notification(notification).await?;
        }
        _ => {
            // Ignore other notification types
            println!("Ignoring unknown notification: {}", notification.method);
        }
    }
    Ok(())
}

/// Helper function to create a setLevel request
pub fn create_set_level_request(level: LogLevel) -> JsonRpcMessage {
    JsonRpcMessage::Request(JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: Some(0), // The transport will set the actual ID
        method: "logging/setLevel".to_string(),
        params: Some(serde_json::to_value(SetLevelParams { level }).unwrap()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn test_logging_manager() {
        let manager = LoggingManager::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        // Add a test handler
        manager
            .add_handler(move |msg| {
                assert_eq!(msg.level, LogLevel::Info);
                assert_eq!(msg.message, "test message");
                counter_clone.fetch_add(1, Ordering::SeqCst);
            })
            .await;

        // Create a test notification
        let notification = JsonRpcNotification {
            jsonrpc: "2.0".to_string(),
            method: "notifications/message".to_string(),
            params: Some(serde_json::json!({
                "level": "info",
                "data": "test message"
            })),
        };

        // Handle the notification
        manager.handle_notification(notification).await.unwrap();

        // Verify the handler was called
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_create_set_level_request() {
        let request = create_set_level_request(LogLevel::Debug);
        match request {
            JsonRpcMessage::Request(req) => {
                assert_eq!(req.method, "logging/setLevel");
                assert!(req.params.is_some());
            }
            _ => panic!("Expected Request"),
        }
    }
}