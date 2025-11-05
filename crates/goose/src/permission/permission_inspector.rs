use crate::agents::extension_manager_extension::MANAGE_EXTENSIONS_TOOL_NAME_COMPLETE;
use crate::config::permission::PermissionLevel;
use crate::config::{GooseMode, PermissionManager};
use crate::conversation::message::{Message, ToolRequest};
use crate::permission::permission_judge::PermissionCheckResult;
use crate::tool_inspection::{InspectionAction, InspectionResult, ToolInspector};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Permission Inspector that handles tool permission checking
pub struct PermissionInspector {
    mode: Arc<Mutex<GooseMode>>,
    readonly_tools: HashSet<String>,
    regular_tools: HashSet<String>,
    pub permission_manager: Arc<Mutex<PermissionManager>>,
}

impl PermissionInspector {
    pub fn new(
        mode: GooseMode,
        readonly_tools: HashSet<String>,
        regular_tools: HashSet<String>,
    ) -> Self {
        Self {
            mode: Arc::new(Mutex::new(mode)),
            readonly_tools,
            regular_tools,
            permission_manager: Arc::new(Mutex::new(PermissionManager::default())),
        }
    }

    pub fn with_permission_manager(
        mode: GooseMode,
        readonly_tools: HashSet<String>,
        regular_tools: HashSet<String>,
        permission_manager: Arc<Mutex<PermissionManager>>,
    ) -> Self {
        Self {
            mode: Arc::new(Mutex::new(mode)),
            readonly_tools,
            regular_tools,
            permission_manager,
        }
    }

    /// Update the mode of this permission inspector
    pub async fn update_mode(&self, new_mode: GooseMode) {
        let mut mode = self.mode.lock().await;
        *mode = new_mode;
    }

    /// Process inspection results into permission decisions
    /// This method takes all inspection results and converts them into a PermissionCheckResult
    /// that can be used by the agent to determine which tools to approve, deny, or ask for approval
    pub fn process_inspection_results(
        &self,
        remaining_requests: &[ToolRequest],
        inspection_results: &[InspectionResult],
    ) -> PermissionCheckResult {
        use crate::tool_inspection::apply_inspection_results_to_permissions;

        // Start with permission inspector's decisions as the baseline
        let mut permission_check_result = PermissionCheckResult {
            approved: vec![],
            needs_approval: vec![],
            denied: vec![],
        };

        // Apply permission inspector results first (baseline behavior)
        let permission_results: Vec<_> = inspection_results
            .iter()
            .filter(|result| result.inspector_name == "permission")
            .collect();

        for request in remaining_requests {
            // Find the permission decision for this request
            if let Some(permission_result) = permission_results
                .iter()
                .find(|result| result.tool_request_id == request.id)
            {
                match permission_result.action {
                    InspectionAction::Allow => {
                        permission_check_result.approved.push(request.clone());
                    }
                    InspectionAction::Deny => {
                        permission_check_result.denied.push(request.clone());
                    }
                    InspectionAction::RequireApproval(_) => {
                        permission_check_result.needs_approval.push(request.clone());
                    }
                }
            } else {
                // If no permission result found, default to needs approval for safety
                permission_check_result.needs_approval.push(request.clone());
            }
        }

        // Apply security and other inspector results as overrides
        let non_permission_results: Vec<_> = inspection_results
            .iter()
            .filter(|result| result.inspector_name != "permission")
            .cloned()
            .collect();

        if !non_permission_results.is_empty() {
            permission_check_result = apply_inspection_results_to_permissions(
                permission_check_result,
                &non_permission_results,
            );
        }

        permission_check_result
    }
}

#[async_trait]
impl ToolInspector for PermissionInspector {
    fn name(&self) -> &'static str {
        "permission"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn inspect(
        &self,
        tool_requests: &[ToolRequest],
        _messages: &[Message],
    ) -> Result<Vec<InspectionResult>> {
        let mut results = Vec::new();
        let permission_manager = self.permission_manager.lock().await;
        let mode = self.mode.lock().await;

        for request in tool_requests {
            if let Ok(tool_call) = &request.tool_call {
                let tool_name = &tool_call.name;

                let action = match *mode {
                    GooseMode::Chat => continue,
                    GooseMode::Auto => InspectionAction::Allow,
                    GooseMode::Approve | GooseMode::SmartApprove => {
                        // 1. Check user-defined permission first
                        if let Some(level) = permission_manager.get_user_permission(tool_name) {
                            match level {
                                PermissionLevel::AlwaysAllow => InspectionAction::Allow,
                                PermissionLevel::NeverAllow => InspectionAction::Deny,
                                PermissionLevel::AskBefore => {
                                    InspectionAction::RequireApproval(None)
                                }
                            }
                        }
                        // 2. Check if it's a readonly or regular tool (both pre-approved)
                        else if self.readonly_tools.contains(tool_name.as_ref())
                            || self.regular_tools.contains(tool_name.as_ref())
                        {
                            InspectionAction::Allow
                        }
                        // 4. Special case for extension management
                        else if tool_name == MANAGE_EXTENSIONS_TOOL_NAME_COMPLETE {
                            InspectionAction::RequireApproval(Some(
                                "Extension management requires approval for security".to_string(),
                            ))
                        }
                        // 5. Default: require approval for unknown tools
                        else {
                            InspectionAction::RequireApproval(None)
                        }
                    }
                };

                let reason = match &action {
                    InspectionAction::Allow => {
                        if *mode == GooseMode::Auto {
                            "Auto mode - all tools approved".to_string()
                        } else if self.readonly_tools.contains(tool_name.as_ref()) {
                            "Tool marked as read-only".to_string()
                        } else if self.regular_tools.contains(tool_name.as_ref()) {
                            "Tool pre-approved".to_string()
                        } else {
                            "User permission allows this tool".to_string()
                        }
                    }
                    InspectionAction::Deny => "User permission denies this tool".to_string(),
                    InspectionAction::RequireApproval(_) => {
                        if tool_name == MANAGE_EXTENSIONS_TOOL_NAME_COMPLETE {
                            "Extension management requires user approval".to_string()
                        } else {
                            "Tool requires user approval".to_string()
                        }
                    }
                };

                results.push(InspectionResult {
                    tool_request_id: request.id.clone(),
                    action,
                    reason,
                    confidence: 1.0, // Permission decisions are definitive
                    inspector_name: self.name().to_string(),
                    finding_id: None,
                });
            }
        }

        Ok(results)
    }
}
