use anyhow::Result;
use async_trait::async_trait;

use crate::conversation::message::{Message, ToolRequest};
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
        let action = if security_result.is_malicious && security_result.should_ask_user {
            // High confidence threat - require user approval with warning
            // Create a user-friendly explanation without BERT model details
            let user_explanation = self.create_user_friendly_explanation(&security_result.explanation);

            InspectionAction::RequireApproval(Some(format!(
                "🔒 Security Alert: This tool call has been flagged as potentially dangerous.\n\
                Confidence: {:.1}%\n\
                Explanation: {}\n\
                Finding ID: {}",
                security_result.confidence * 100.0,
                user_explanation,
                security_result.finding_id
            )))
        } else {
            // Either not malicious, or below threshold (already logged) - allow
            InspectionAction::Allow
        };

        InspectionResult {
            tool_request_id,
            action,
            reason: format!("{}, threshold: {}", security_result.explanation, security_result.threshold),
            confidence: security_result.confidence,
            inspector_name: self.name().to_string(),
            finding_id: Some(security_result.finding_id.clone()),
        }
    }

    /// Create a user-friendly explanation by removing BERT model details
    fn create_user_friendly_explanation(&self, full_explanation: &str) -> String {
        // Remove BERT model information from user-facing messages
        // Keep only the pattern analysis details for the user

        if full_explanation.starts_with("Detected by pattern analysis (BERT model found no injection") {
            // Extract just the pattern analysis part after the BERT model note
            if let Some(pattern_start) = full_explanation.find("):\n") {
                return full_explanation[pattern_start + 3..].to_string();
            }
        }

        if full_explanation.starts_with("Detected by both BERT model") {
            // Extract just the pattern analysis part
            if let Some(pattern_start) = full_explanation.find("and pattern analysis:\n") {
                return full_explanation[pattern_start + 22..].to_string();
            }
        }

        if full_explanation.starts_with("Detected by BERT model") {
            // For BERT-only detections, provide a generic message
            return "Potential prompt injection detected".to_string();
        }

        // For other cases (pattern-only, no threats), return as-is
        full_explanation.to_string()
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

        // Convert security results to inspection results
        // The SecurityManager already handles the correlation between tool requests and results
        let inspection_results = security_results
            .into_iter()
            .map(|security_result| {
                let tool_request_id = security_result.tool_request_id.clone();
                self.convert_security_result(&security_result, tool_request_id)
            })
            .collect();

        Ok(inspection_results)
    }

    fn is_enabled(&self) -> bool {
        self.security_manager
            .is_prompt_injection_detection_enabled()
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
    use rmcp::model::CallToolRequestParam;
    use rmcp::object;

    #[tokio::test]
    async fn test_security_inspector() {
        let inspector = SecurityInspector::new();

        // Test with a potentially dangerous tool call
        let tool_requests = vec![ToolRequest {
            id: "test_req".to_string(),
            tool_call: Ok(CallToolRequestParam {
                name: "shell".into(),
                arguments: Some(object!({"command": "rm -rf /"})),
            }),
        }];

        let results = inspector.inspect(&tool_requests, &[]).await.unwrap();

        // Results depend on whether security is enabled in config
        if inspector.is_enabled() {
            // If security is enabled, should detect the dangerous command
            assert!(
                !results.is_empty(),
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
