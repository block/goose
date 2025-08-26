use crate::conversation::message::{Message, ToolRequest};
use crate::providers::base::Provider;
use crate::tool_inspection::{InspectionAction, InspectionResult, ToolInspector};
use crate::config::PermissionManager;
use crate::permission::permission_judge::{check_tool_permissions, PermissionCheckResult};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    name: String,
    parameters: serde_json::Value,
}

impl ToolCall {
    pub fn new(name: String, parameters: serde_json::Value) -> Self {
        Self { name, parameters }
    }

    fn matches(&self, other: &ToolCall) -> bool {
        self.name == other.name && self.parameters == other.parameters
    }
}

#[derive(Debug)]
pub struct ToolMonitor {
    max_repetitions: Option<u32>,
    last_call: Option<ToolCall>,
    repeat_count: u32,
    call_counts: HashMap<String, u32>,
    // Permission checking fields
    mode: String,
    readonly_tools: HashSet<String>,
    regular_tools: HashSet<String>,
}

impl ToolMonitor {
    pub fn new(max_repetitions: Option<u32>) -> Self {
        Self {
            max_repetitions,
            last_call: None,
            repeat_count: 0,
            call_counts: HashMap::new(),
            mode: "smart_approve".to_string(),
            readonly_tools: HashSet::new(),
            regular_tools: HashSet::new(),
        }
    }



    pub fn check_tool_call(&mut self, tool_call: ToolCall) -> bool {
        let total_calls = self.call_counts.entry(tool_call.name.clone()).or_insert(0);
        *total_calls += 1;

        if self.max_repetitions.is_none() {
            self.last_call = Some(tool_call);
            self.repeat_count = 1;
            return true;
        }

        if let Some(last) = &self.last_call {
            if last.matches(&tool_call) {
                self.repeat_count += 1;
                if self.repeat_count > self.max_repetitions.unwrap() {
                    return false;
                }
            } else {
                self.repeat_count = 1;
            }
        } else {
            self.repeat_count = 1;
        }

        self.last_call = Some(tool_call);
        true
    }

    pub fn reset(&mut self) {
        self.last_call = None;
        self.repeat_count = 0;
        self.call_counts.clear();
    }

    /// Convert PermissionCheckResult to InspectionResults
    fn convert_permission_result(
        &self,
        permission_result: PermissionCheckResult,
        extension_request_ids: Vec<String>,
    ) -> Vec<InspectionResult> {
        let mut results = Vec::new();

        // Approved tools - allow them
        for request in permission_result.approved {
            results.push(InspectionResult {
                tool_request_id: request.id,
                action: InspectionAction::Allow,
                reason: "Tool approved by permission system".to_string(),
                confidence: 1.0,
                inspector_name: "tool_monitor".to_string(),
                finding_id: None,
            });
        }

        // Tools needing approval - require approval
        for request in permission_result.needs_approval {
            let warning_message = if extension_request_ids.contains(&request.id) {
                Some("This tool will install or manage extensions. Please review carefully.".to_string())
            } else {
                Some("This tool requires user approval before execution.".to_string())
            };

            results.push(InspectionResult {
                tool_request_id: request.id,
                action: InspectionAction::RequireApproval(warning_message),
                reason: "Tool requires user approval".to_string(),
                confidence: 1.0,
                inspector_name: "tool_monitor".to_string(),
                finding_id: None,
            });
        }

        // Denied tools - deny them
        for request in permission_result.denied {
            results.push(InspectionResult {
                tool_request_id: request.id,
                action: InspectionAction::Deny,
                reason: "Tool denied by permission system".to_string(),
                confidence: 1.0,
                inspector_name: "tool_monitor".to_string(),
                finding_id: None,
            });
        }

        results
    }
}

#[async_trait]
impl ToolInspector for ToolMonitor {
    fn name(&self) -> &'static str {
        "tool_monitor"
    }

    async fn inspect(
        &self,
        tool_requests: &[ToolRequest],
        _messages: &[Message],
        provider: Option<Arc<dyn Provider>>,
    ) -> Result<Vec<InspectionResult>> {
        let mut results = Vec::new();

        // 1. First check repetition limits
        for tool_request in tool_requests {
            if let Ok(tool_call) = &tool_request.tool_call {
                let tool_call_info =
                    ToolCall::new(tool_call.name.clone(), tool_call.arguments.clone());

                // Create a temporary clone to check without modifying state
                let mut temp_monitor = ToolMonitor::new(self.max_repetitions);
                temp_monitor.last_call = self.last_call.clone();
                temp_monitor.repeat_count = self.repeat_count;
                temp_monitor.call_counts = self.call_counts.clone();

                if !temp_monitor.check_tool_call(tool_call_info) {
                    results.push(InspectionResult {
                        tool_request_id: tool_request.id.clone(),
                        action: InspectionAction::Deny,
                        reason: format!(
                            "Tool '{}' has exceeded maximum repetitions",
                            tool_call.name
                        ),
                        confidence: 1.0,
                        inspector_name: "tool_monitor".to_string(),
                        finding_id: Some("REP-001".to_string()),
                    });
                }
            }
        }

        // 2. Then check permissions (only if provider is available)
        if let Some(provider) = provider {
            // Clone the permission manager for use in the permission check
            let mut permission_manager = PermissionManager::default();
            
            let (permission_result, extension_request_ids) = check_tool_permissions(
                tool_requests,
                &self.mode,
                self.readonly_tools.clone(),
                self.regular_tools.clone(),
                &mut permission_manager,
                provider,
            )
            .await;

            let permission_results = self.convert_permission_result(permission_result, extension_request_ids);
            results.extend(permission_results);
        } else {
            // If no provider, just allow all tools that passed repetition check
            for tool_request in tool_requests {
                // Only add Allow results for tools that didn't already get denied for repetition
                if !results.iter().any(|r| r.tool_request_id == tool_request.id && r.action == InspectionAction::Deny) {
                    results.push(InspectionResult {
                        tool_request_id: tool_request.id.clone(),
                        action: InspectionAction::Allow,
                        reason: "No permission checking available, allowing by default".to_string(),
                        confidence: 0.5,
                        inspector_name: "tool_monitor".to_string(),
                        finding_id: None,
                    });
                }
            }
        }

        Ok(results)
    }

    fn priority(&self) -> u32 {
        100 // Medium priority - handles both repetition and permissions
    }
}
