use crate::config::PermissionManager;
use crate::conversation::message::{Message, ToolRequest};
use crate::permission::permission_judge::check_tool_permissions;
use crate::providers::base::Provider;
use crate::tool_inspection::{InspectionAction, InspectionResult, ToolInspector};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::Arc;

/// Permission Inspector that handles tool permission checking
pub struct PermissionInspector {
    mode: String,
    readonly_tools: HashSet<String>,
    regular_tools: HashSet<String>,
}

impl PermissionInspector {
    pub fn new(
        mode: String,
        readonly_tools: HashSet<String>,
        regular_tools: HashSet<String>,
    ) -> Self {
        Self {
            mode,
            readonly_tools,
            regular_tools,
        }
    }
}

#[async_trait]
impl ToolInspector for PermissionInspector {
    fn name(&self) -> &'static str {
        "permission"
    }

    async fn inspect(
        &self,
        tool_requests: &[ToolRequest],
        _messages: &[Message],
        provider: Option<Arc<dyn Provider>>,
    ) -> Result<Vec<InspectionResult>> {
        let provider = provider.ok_or_else(|| anyhow::anyhow!("Provider required for permission checking"))?;
        
        // Create a fresh permission manager for this check
        let mut permission_manager = PermissionManager::default();
        
        let (permission_result, extension_request_ids) = check_tool_permissions(
            tool_requests,
            &self.mode,
            self.readonly_tools.clone(),
            self.regular_tools.clone(),
            &mut permission_manager,
            provider,
        ).await;

        let mut results = Vec::new();

        // Convert permission results to inspection results
        for request in &permission_result.approved {
            results.push(InspectionResult {
                tool_request_id: request.id.clone(),
                action: InspectionAction::Allow,
                reason: "Tool approved by permission system".to_string(),
                confidence: 1.0,
                inspector_name: self.name().to_string(),
                finding_id: None,
            });
        }

        for request in &permission_result.needs_approval {
            let warning_message = if extension_request_ids.contains(&request.id) {
                Some("This tool will install or manage extensions. Please review carefully.".to_string())
            } else {
                Some("This tool requires user approval before execution.".to_string())
            };

            results.push(InspectionResult {
                tool_request_id: request.id.clone(),
                action: InspectionAction::RequireApproval(warning_message),
                reason: "Tool requires user approval".to_string(),
                confidence: 1.0,
                inspector_name: self.name().to_string(),
                finding_id: None,
            });
        }

        for request in &permission_result.denied {
            results.push(InspectionResult {
                tool_request_id: request.id.clone(),
                action: InspectionAction::Deny,
                reason: "Tool denied by permission system".to_string(),
                confidence: 1.0,
                inspector_name: self.name().to_string(),
                finding_id: None,
            });
        }

        Ok(results)
    }

    fn priority(&self) -> u32 {
        150 // Medium-high priority - runs after security but before other inspectors
    }
}
