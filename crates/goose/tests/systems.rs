use anyhow::Result as AnyhowResult;
use async_trait::async_trait;
use serde_json::{json, Value};

use goose::errors::{AgentError, AgentResult};
use goose::models::content::Content;
use goose::models::tool::{Tool, ToolCall};
use goose::systems::{Resource, System};

/// A simple system that echoes input back to the caller
pub struct EchoSystem {
    tools: Vec<Tool>,
}

impl Default for EchoSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl EchoSystem {
    pub fn new() -> Self {
        Self {
            tools: vec![Tool::new(
                "echo",
                "reply with the input",
                json!({
                    "type": "object",
                    "properties": {
                        "message": {
                            "type": "string",
                            "description": "The message to echo"
                        }
                    },
                    "required": ["message"]
                }),
            )],
        }
    }

    async fn echo(&self, params: Value) -> AgentResult<Vec<Content>> {
        let message = params
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AgentError::InvalidParameters("message parameter required".into()))?;

        Ok(vec![Content::text(message)])
    }
}

#[async_trait]
impl System for EchoSystem {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> &str {
        "A simple system that echoes input back to the caller"
    }

    fn instructions(&self) -> &str {
        "Use the echo tool to get a response back with your input message"
    }

    fn tools(&self) -> &[Tool] {
        &self.tools
    }

    async fn status(&self) -> AnyhowResult<Vec<Resource>> {
        Ok(Vec::new()) // Echo system has no state to report
    }

    async fn call(&self, tool_call: ToolCall) -> AgentResult<Vec<Content>> {
        match tool_call.name.as_str() {
            "echo" => self.echo(tool_call.arguments).await,
            _ => Err(AgentError::ToolNotFound(tool_call.name)),
        }
    }

    async fn read_resource(&self, _uri: &str) -> AgentResult<String> {
        Ok("".to_string())
    }
}

mod tests {
    use super::*;

    #[tokio::test]
    async fn test_echo_success() {
        let system = EchoSystem::new();

        let tool_call = ToolCall::new("echo", json!({"message": "hello world"}));
        let result = system.call(tool_call).await.unwrap();
        assert_eq!(result.len(), 1);
        if let Content::Text(text) = &result[0] {
            assert_eq!(text.text, "hello world");
        } else {
            panic!("Expected text content");
        }
    }

    #[tokio::test]
    async fn test_echo_missing_message() {
        let system = EchoSystem::new();

        let tool_call = ToolCall::new("echo", json!({}));
        let error = system.call(tool_call).await.unwrap_err();

        assert!(matches!(error, AgentError::InvalidParameters(_)));
    }

    #[tokio::test]
    async fn test_unknown_tool() {
        let system = EchoSystem::new();

        let tool_call = ToolCall::new("unknown", json!({}));
        let error = system.call(tool_call).await.unwrap_err();

        assert!(matches!(error, AgentError::ToolNotFound(_)));
    }
}
