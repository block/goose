use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use crate::conversation::message::{Message, ToolRequest};
use crate::providers::base::Provider;

/// Result of inspecting a tool call
#[derive(Debug, Clone)]
pub struct InspectionResult {
    pub tool_request_id: String,
    pub action: InspectionAction,
    pub reason: String,
    pub confidence: f32,
    pub inspector_name: String,
    pub finding_id: Option<String>,
}

/// Action to take based on inspection result
#[derive(Debug, Clone, PartialEq)]
pub enum InspectionAction {
    /// Allow the tool to execute without user intervention
    Allow,
    /// Deny the tool execution completely
    Deny,
    /// Require user approval before execution (with optional warning message)
    RequireApproval(Option<String>),
}

/// Trait for all tool inspectors
#[async_trait]
pub trait ToolInspector: Send + Sync {
    /// Name of this inspector (for logging/debugging)
    fn name(&self) -> &'static str;

    /// Inspect tool requests and return results
    async fn inspect(
        &self,
        tool_requests: &[ToolRequest],
        messages: &[Message],
        provider: Option<Arc<dyn Provider>>,
    ) -> Result<Vec<InspectionResult>>;

    /// Whether this inspector is enabled
    fn is_enabled(&self) -> bool {
        true
    }

    /// Priority of this inspector (higher = runs first)
    fn priority(&self) -> u32 {
        100
    }
}

/// Manages all tool inspectors and coordinates their results
pub struct ToolInspectionManager {
    inspectors: Vec<Box<dyn ToolInspector>>,
}

impl ToolInspectionManager {
    pub fn new() -> Self {
        Self {
            inspectors: Vec::new(),
        }
    }

    /// Add an inspector to the manager
    pub fn add_inspector(&mut self, inspector: Box<dyn ToolInspector>) {
        self.inspectors.push(inspector);
        // Sort by priority (highest first)
        self.inspectors.sort_by(|a, b| b.priority().cmp(&a.priority()));
    }

    /// Run all inspectors on the tool requests
    pub async fn inspect_tools(
        &self,
        tool_requests: &[ToolRequest],
        messages: &[Message],
        provider: Option<Arc<dyn Provider>>,
    ) -> Result<Vec<InspectionResult>> {
        let mut all_results = Vec::new();

        for inspector in &self.inspectors {
            if !inspector.is_enabled() {
                continue;
            }

            tracing::debug!(
                inspector_name = inspector.name(),
                tool_count = tool_requests.len(),
                "Running tool inspector"
            );

            match inspector.inspect(tool_requests, messages, provider.clone()).await {
                Ok(results) => {
                    tracing::debug!(
                        inspector_name = inspector.name(),
                        result_count = results.len(),
                        "Tool inspector completed"
                    );
                    all_results.extend(results);
                }
                Err(e) => {
                    tracing::error!(
                        inspector_name = inspector.name(),
                        error = %e,
                        "Tool inspector failed"
                    );
                    // Continue with other inspectors even if one fails
                }
            }
        }

        Ok(all_results)
    }

    /// Get list of registered inspector names
    pub fn inspector_names(&self) -> Vec<&'static str> {
        self.inspectors.iter().map(|i| i.name()).collect()
    }
}

impl Default for ToolInspectionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Apply inspection results to permission check results
/// This is the generic permission-mixing logic that works for all inspector types
pub fn apply_inspection_results_to_permissions(
    mut permission_result: crate::permission::permission_judge::PermissionCheckResult,
    inspection_results: &[InspectionResult],
) -> crate::permission::permission_judge::PermissionCheckResult {
    if inspection_results.is_empty() {
        return permission_result;
    }

    // Create a map of tool requests by ID for easy lookup
    let mut all_requests: HashMap<String, ToolRequest> = HashMap::new();

    // Collect all tool requests
    for req in &permission_result.approved {
        all_requests.insert(req.id.clone(), req.clone());
    }
    for req in &permission_result.needs_approval {
        all_requests.insert(req.id.clone(), req.clone());
    }
    for req in &permission_result.denied {
        all_requests.insert(req.id.clone(), req.clone());
    }

    // Process inspection results
    for result in inspection_results {
        let request_id = &result.tool_request_id;

        tracing::info!(
            inspector_name = result.inspector_name,
            tool_request_id = %request_id,
            action = ?result.action,
            confidence = result.confidence,
            reason = %result.reason,
            finding_id = ?result.finding_id,
            "Applying inspection result"
        );

        match result.action {
            InspectionAction::Deny => {
                // Remove from approved and needs_approval, add to denied
                permission_result.approved.retain(|req| req.id != *request_id);
                permission_result.needs_approval.retain(|req| req.id != *request_id);
                
                if let Some(request) = all_requests.get(request_id) {
                    if !permission_result.denied.iter().any(|req| req.id == *request_id) {
                        permission_result.denied.push(request.clone());
                    }
                }
            }
            InspectionAction::RequireApproval(_) => {
                // Remove from approved, add to needs_approval if not already there
                permission_result.approved.retain(|req| req.id != *request_id);
                
                if let Some(request) = all_requests.get(request_id) {
                    if !permission_result.needs_approval.iter().any(|req| req.id == *request_id) {
                        permission_result.needs_approval.push(request.clone());
                    }
                }
            }
            InspectionAction::Allow => {
                // This inspector allows it, but don't override other inspectors' decisions
                // If it's already denied or needs approval, leave it that way
            }
        }
    }

    permission_result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::ToolRequest;
    use crate::permission::permission_judge::PermissionCheckResult;
    use mcp_core::ToolCall;
    use serde_json::json;

    struct MockInspector {
        name: &'static str,
        results: Vec<InspectionResult>,
    }

    #[async_trait]
    impl ToolInspector for MockInspector {
        fn name(&self) -> &'static str {
            self.name
        }

        async fn inspect(
            &self,
            _tool_requests: &[ToolRequest],
            _messages: &[Message],
            _provider: Option<Arc<dyn Provider>>,
        ) -> Result<Vec<InspectionResult>> {
            Ok(self.results.clone())
        }
    }

    #[tokio::test]
    async fn test_inspection_manager() {
        let mut manager = ToolInspectionManager::new();
        
        let inspector = MockInspector {
            name: "test_inspector",
            results: vec![InspectionResult {
                tool_request_id: "req_1".to_string(),
                action: InspectionAction::Deny,
                reason: "Test denial".to_string(),
                confidence: 0.9,
                inspector_name: "test_inspector".to_string(),
                finding_id: Some("TEST-001".to_string()),
            }],
        };

        manager.add_inspector(Box::new(inspector));

        let tool_requests = vec![ToolRequest {
            id: "req_1".to_string(),
            tool_call: Ok(ToolCall {
                name: "test_tool".to_string(),
                arguments: json!({}),
            }),
        }];

        let results = manager.inspect_tools(&tool_requests, &[], None).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].action, InspectionAction::Deny);
    }

    #[test]
    fn test_apply_inspection_results() {
        let tool_request = ToolRequest {
            id: "req_1".to_string(),
            tool_call: Ok(ToolCall {
                name: "test_tool".to_string(),
                arguments: json!({}),
            }),
        };

        let permission_result = PermissionCheckResult {
            approved: vec![tool_request.clone()],
            needs_approval: vec![],
            denied: vec![],
        };

        let inspection_results = vec![InspectionResult {
            tool_request_id: "req_1".to_string(),
            action: InspectionAction::Deny,
            reason: "Test denial".to_string(),
            confidence: 0.9,
            inspector_name: "test_inspector".to_string(),
            finding_id: Some("TEST-001".to_string()),
        }];

        let updated_result = apply_inspection_results_to_permissions(
            permission_result,
            &inspection_results,
        );

        assert_eq!(updated_result.approved.len(), 0);
        assert_eq!(updated_result.denied.len(), 1);
        assert_eq!(updated_result.denied[0].id, "req_1");
    }
}
