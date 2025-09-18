use anyhow::Result;
use async_trait::async_trait;

use crate::conversation::message::{Message, ToolRequest};
use crate::security::{context::SecurityContext, SecurityManager, SecurityResult};
use crate::tool_inspection::{InspectionAction, InspectionResult, ToolInspector};

/// Security inspector that uses pattern matching to detect malicious tool calls
pub struct SecurityInspector {
    security_manager: SecurityManager,
    security_context: SecurityContext,
}

impl SecurityInspector {
    pub fn new() -> Self {
        Self {
            security_manager: SecurityManager::new(),
            security_context: SecurityContext::new(),
        }
    }

    /// Get the security context for accessing finding IDs
    pub fn get_security_context(&self) -> &SecurityContext {
        &self.security_context
    }

    /// Get security finding ID for a tool request ID
    pub async fn get_security_finding_id(&self, request_id: &str) -> Option<String> {
        self.security_context.get_finding_id(request_id).await
    }

    /// Convert SecurityResult to InspectionResult
    fn convert_security_result(
        &self,
        security_result: &SecurityResult,
        tool_request_id: String,
    ) -> InspectionResult {
        let action = if security_result.is_malicious && security_result.should_ask_user {
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
            // Either not malicious, or below threshold (already logged) - allow
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

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn inspect(
        &self,
        tool_requests: &[ToolRequest],
        messages: &[Message],
    ) -> Result<Vec<InspectionResult>> {
        let security_results = self
            .security_manager
            .analyze_tool_requests(tool_requests, messages)
            .await?;

        // Store security finding IDs in context for later retrieval
        for security_result in &security_results {
            self.security_context
                .store_finding_id(
                    &security_result.tool_request_id,
                    &security_result.finding_id,
                )
                .await;
        }

        // Convert security results to inspection results
        // The SecurityManager already handles the correlation between tool requests and results
        let inspection_results = security_results
            .into_iter()
            .map(|security_result| {
                // Extract the tool request ID from the security result's context
                // The SecurityManager should provide this information
                let tool_request_id = security_result.tool_request_id.clone();
                self.convert_security_result(&security_result, tool_request_id)
            })
            .collect();

        Ok(inspection_results)
    }

    fn is_enabled(&self) -> bool {
        use crate::config::Config;
        let config = Config::global();
        config
            .get_param::<bool>("security_enabled")
            .unwrap_or(false)
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

        let results = inspector.inspect(&tool_requests, &[]).await.unwrap();

        // Results depend on whether security is enabled in config
        if inspector.is_enabled() {
            // If security is enabled, should detect the dangerous command
            assert!(
                results.len() >= 1,
                "Security inspector should detect dangerous command when enabled"
            );
            if !results.is_empty() {
                assert_eq!(results[0].inspector_name, "security");
                assert!(results[0].confidence > 0.0);
            }
        } else {
            // If security is disabled, should return no results
            assert_eq!(
                results.len(),
                0,
                "Security inspector should return no results when disabled"
            );
        }
    }

    #[test]
    fn test_security_inspector_name() {
        let inspector = SecurityInspector::new();
        assert_eq!(inspector.name(), "security");
    }
}
