use crate::conversation::message::{Message, ToolRequest};
use crate::tool_inspection::{InspectionAction, InspectionResult, ToolInspector};
use crate::config::permission::PermissionLevel;
use crate::config::PermissionManager;
use crate::agents::platform_tools::PLATFORM_MANAGE_EXTENSIONS_TOOL_NAME;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

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
    ) -> Self {
        Self {
            mode,
            readonly_tools,
            regular_tools,
            permission_manager: Arc::new(Mutex::new(PermissionManager::default())),
        }
    }

    pub fn with_permission_manager(
        mode: String,
        readonly_tools: HashSet<String>,
        regular_tools: HashSet<String>,
        permission_manager: Arc<Mutex<PermissionManager>>,
    ) -> Self {
        Self {
            mode,
            readonly_tools,
            regular_tools,
            permission_manager,
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
    ) -> Result<Vec<InspectionResult>> {
        let mut results = Vec::new();
        let permission_manager = self.permission_manager.lock().await;

        for request in tool_requests {
            if let Ok(tool_call) = &request.tool_call {
                let tool_name = &tool_call.name;

                // Handle different modes
                let action = if self.mode == "chat" {
                    // In chat mode, all tools are skipped (handled elsewhere)
                    continue;
                } else if self.mode == "auto" {
                    // In auto mode, all tools are approved
                    InspectionAction::Allow
                } else {
                    // Smart mode - check permissions
                    
                    // 1. Check user-defined permission first
                    if let Some(level) = permission_manager.get_user_permission(tool_name) {
                        match level {
                            PermissionLevel::AlwaysAllow => InspectionAction::Allow,
                            PermissionLevel::NeverAllow => InspectionAction::Deny,
                            PermissionLevel::AskBefore => InspectionAction::RequireApproval(None),
                        }
                    }
                    // 2. Check if it's a readonly tool
                    else if self.readonly_tools.contains(tool_name) {
                        InspectionAction::Allow
                    }
                    // 3. Check if it's in the regular tools list (pre-approved)
                    else if self.regular_tools.contains(tool_name) {
                        InspectionAction::Allow
                    }
                    // 4. Special case for extension management
                    else if tool_name == PLATFORM_MANAGE_EXTENSIONS_TOOL_NAME {
                        InspectionAction::RequireApproval(Some(
                            "Extension management requires approval for security".to_string()
                        ))
                    }
                    // 5. Default: require approval for unknown tools
                    else {
                        InspectionAction::RequireApproval(None)
                    }
                };

                let reason = match &action {
                    InspectionAction::Allow => {
                        if self.mode == "auto" {
                            "Auto mode - all tools approved".to_string()
                        } else if self.readonly_tools.contains(tool_name) {
                            "Tool marked as read-only".to_string()
                        } else if self.regular_tools.contains(tool_name) {
                            "Tool pre-approved".to_string()
                        } else {
                            "User permission allows this tool".to_string()
                        }
                    }
                    InspectionAction::Deny => "User permission denies this tool".to_string(),
                    InspectionAction::RequireApproval(_) => {
                        if tool_name == PLATFORM_MANAGE_EXTENSIONS_TOOL_NAME {
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

    fn priority(&self) -> u32 {
        150 // Medium-high priority - runs after security but before other inspectors
    }
}
