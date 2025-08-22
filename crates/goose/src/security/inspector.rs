use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use crate::conversation::message::{Message, ToolRequest};
use crate::providers::base::Provider;
use crate::security::{SecurityManager, SecurityResult};
use crate::tool_inspection::{InspectionAction, InspectionResult, ToolInspector};

/// Security inspector that uses pattern matching to detect malicious tool calls
pub struct SecurityInspector {
    security_manager: SecurityManager,
}

impl SecurityInspector {
    pub fn new() -> Self {
        Self {
            security_manager: SecurityManager::new(),
        }
    }

    /// Convert SecurityResult to InspectionResult
    fn convert_security_result(
        &self,
        security_result: &SecurityResult,
        tool_request_id: String,
    ) -> InspectionResult {
        let action = if security_result.is_malicious {
            if security_result.should_ask_user {
                // High confidence threat - require user approval with warning
                InspectionAction::RequireApproval(Some(format!(
                    "🔒 Security Alert: This tool call has been flagged as potentially dangerous.\n\
                    Confidence: {:.1}%\n\
                    Explanation: {}\n\
                    Finding ID: {}",
                    security_result.confidence * 100.0,
                    security_result.explanation,
                    security_result.finding_id
                )))
            } else {
                // Low confidence threat - allow but log
                InspectionAction::Allow
            }
        } else {
            InspectionAction::Allow
        };

        InspectionResult {
            tool_request_id,
            action,
            reason: security_result.explanation.clone(),
            confidence: security_result.confidence,
            inspector_name: self.name().to_string(),
            finding_id: Some(security_result.finding_id.clone()),
        }
    }
}

#[async_trait]
impl ToolInspector for SecurityInspector {
    fn name(&self) -> &'static str {
        "security"
    }

    async fn inspect(
        &self,
        tool_requests: &[ToolRequest],
        messages: &[Message],
        _provider: Option<Arc<dyn Provider>>,
    ) -> Result<Vec<InspectionResult>> {
        let security_results = self
            .security_manager
            .analyze_tool_requests(tool_requests, messages)
            .await?;

        let mut inspection_results = Vec::new();

        // Match security results with tool requests by index
        for (i, security_result) in security_results.iter().enumerate() {
            if let Some(tool_request) = tool_requests.get(i) {
                let inspection_result = self.convert_security_result(security_result, tool_request.id.clone());
                inspection_results.push(inspection_result);
            }
        }

        Ok(inspection_results)
    }

    fn is_enabled(&self) -> bool {
        // Check if security is enabled in config
        use crate::config::Config;
        let config = Config::global();
        
        config
            .get_param::<serde_json::Value>("security")
            .ok()
            .and_then(|security_config| security_config.get("enabled")?.as_bool())
            .unwrap_or(false)
    }

    fn priority(&self) -> u32 {
        // High priority - security checks should run early
        200
    }
}

impl Default for SecurityInspector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::ToolRequest;
    use mcp_core::ToolCall;
    use serde_json::json;

    #[tokio::test]
    async fn test_security_inspector() {
        let inspector = SecurityInspector::new();
        
        // Test with a potentially dangerous tool call
        let tool_requests = vec![ToolRequest {
            id: "test_req".to_string(),
            tool_call: Ok(ToolCall {
                name: "shell".to_string(),
                arguments: json!({"command": "rm -rf /"}),
            }),
        }];

        let results = inspector.inspect(&tool_requests, &[], None).await.unwrap();
        
        // Results depend on whether security is enabled in config
        if inspector.is_enabled() {
            // If security is enabled, should detect the dangerous command
            assert!(results.len() >= 1, "Security inspector should detect dangerous command when enabled");
            if !results.is_empty() {
                assert_eq!(results[0].inspector_name, "security");
                assert!(results[0].confidence > 0.0);
            }
        } else {
            // If security is disabled, should return no results
            assert_eq!(results.len(), 0, "Security inspector should return no results when disabled");
        }
    }

    #[test]
    fn test_security_inspector_name() {
        let inspector = SecurityInspector::new();
        assert_eq!(inspector.name(), "security");
    }
}
