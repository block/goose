use crate::config::PermissionManager;
use crate::conversation::message::{Message, ToolRequest};
use crate::permission::permission_judge::check_tool_permissions;
use crate::providers::base::Provider;
use crate::tool_inspection::{InspectionAction, InspectionResult, ToolInspector};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

/// Permission Inspector that handles tool permission checking
pub struct PermissionInspector {
    mode: String,
    readonly_tools: HashSet<String>,
    regular_tools: HashSet<String>,
    permission_manager: Arc<Mutex<PermissionManager>>,
}

impl PermissionInspector {
    pub fn new(
        mode: String,
        readonly_tools: HashSet<String>,
        regular_tools: HashSet<String>,
        permission_manager: PermissionManager,
    ) -> Self {
        Self {
            mode,
            readonly_tools,
            regular_tools,
            permission_manager: Arc::new(Mutex::new(permission_manager)),
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
        let mut results = Vec::new();
        
        // Clone the permission manager to avoid holding the lock across await
        let mut permission_manager = {
            let guard = self.permission_manager.lock().unwrap();
            guard.clone()
        };
        
        let (permission_result, extension_request_ids) = check_tool_permissions(
            tool_requests,
            &self.mode,
            self.readonly_tools.clone(),
            self.regular_tools.clone(),
            &mut permission_manager,
            provider.unwrap_or_else(|| {
                // Create a dummy provider if none provided - this shouldn't happen in practice
                panic!("Provider required for permission checking")
            }),
        ).await;

        // Update the shared permission manager with any changes
        {
            let mut guard = self.permission_manager.lock().unwrap();
            *guard = permission_manager;
        }

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
