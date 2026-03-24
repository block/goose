use crate::agents::platform_extensions::MANAGE_EXTENSIONS_TOOL_NAME_COMPLETE;
use crate::agents::types::SharedProvider;
use crate::config::permission::PermissionLevel;
use crate::config::{GooseMode, PermissionManager};
use crate::conversation::message::{Message, ToolRequest};
use crate::permission::permission_judge::{detect_read_only_tools, PermissionCheckResult};
use crate::tool_inspection::{InspectionAction, InspectionResult, ToolInspector};
use anyhow::Result;
use async_trait::async_trait;
use rmcp::model::Tool;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

/// Permission Inspector that handles tool permission checking
pub struct PermissionInspector {
    pub permission_manager: Arc<PermissionManager>,
    provider: SharedProvider,
    readonly_tools: RwLock<HashSet<String>>,
}

impl PermissionInspector {
    pub fn new(permission_manager: Arc<PermissionManager>, provider: SharedProvider) -> Self {
        Self {
            permission_manager,
            provider,
            readonly_tools: RwLock::new(HashSet::new()),
        }
    }

    // readonly_tools is per-agent to avoid concurrent session clobbering; write-annotated
    // tools are cached globally via PermissionManager.
    pub fn apply_tool_annotations(&self, tools: &[Tool]) {
        let mut readonly_annotated = HashSet::new();
        for tool in tools {
            let Some(anns) = &tool.annotations else {
                continue;
            };
            if anns.read_only_hint == Some(true) {
                readonly_annotated.insert(tool.name.to_string());
            }
        }
        *self.readonly_tools.write().unwrap() = readonly_annotated;
        self.permission_manager.apply_tool_annotations(tools);
    }

    pub fn is_readonly_annotated_tool(&self, tool_name: &str) -> bool {
        self.readonly_tools.read().unwrap().contains(tool_name)
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
        session_id: &str,
        tool_requests: &[ToolRequest],
        _messages: &[Message],
        goose_mode: GooseMode,
    ) -> Result<Vec<InspectionResult>> {
        let mut results = Vec::new();
        let permission_manager = &self.permission_manager;
        let mut llm_detect_candidates: Vec<&ToolRequest> = Vec::new();

        for request in tool_requests {
            if let Ok(tool_call) = &request.tool_call {
                let tool_name = &tool_call.name;

                let action = match goose_mode {
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
                        // 2. Check if it's a smart-approved tool (annotation or cached LLM decision)
                        } else if self.is_readonly_annotated_tool(tool_name)
                            || (goose_mode == GooseMode::SmartApprove
                                && permission_manager.get_smart_approve_permission(tool_name)
                                    == Some(PermissionLevel::AlwaysAllow))
                        {
                            InspectionAction::Allow
                        // 3. Special case for extension management
                        } else if tool_name == MANAGE_EXTENSIONS_TOOL_NAME_COMPLETE {
                            InspectionAction::RequireApproval(Some(
                                "Extension management requires approval for security".to_string(),
                            ))
                        // 4. Defer to LLM detection (SmartApprove, not yet cached)
                        } else if goose_mode == GooseMode::SmartApprove
                            && permission_manager
                                .get_smart_approve_permission(tool_name)
                                .is_none()
                        {
                            llm_detect_candidates.push(request);
                            continue;
                        // 5. Default: require approval for unknown tools
                        } else {
                            InspectionAction::RequireApproval(None)
                        }
                    }
                };

                let reason = match &action {
                    InspectionAction::Allow => {
                        if goose_mode == GooseMode::Auto {
                            "Auto mode - all tools approved".to_string()
                        } else if self.is_readonly_annotated_tool(tool_name) {
                            "Tool annotated as read-only".to_string()
                        } else if goose_mode == GooseMode::SmartApprove {
                            "SmartApprove cached as read-only".to_string()
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

        // LLM-based read-only detection for deferred SmartApprove candidates
        if !llm_detect_candidates.is_empty() {
            let detected: HashSet<String> = match self.provider.lock().await.clone() {
                Some(provider) => {
                    detect_read_only_tools(provider, session_id, llm_detect_candidates.to_vec())
                        .await
                        .into_iter()
                        .collect()
                }
                None => Default::default(),
            };

            for candidate in &llm_detect_candidates {
                let is_readonly = candidate
                    .tool_call
                    .as_ref()
                    .map(|tc| detected.contains(&tc.name.to_string()))
                    .unwrap_or(false);

                // Cache the LLM decision for future calls
                if let Ok(tc) = &candidate.tool_call {
                    let level = if is_readonly {
                        PermissionLevel::AlwaysAllow
                    } else {
                        PermissionLevel::AskBefore
                    };
                    permission_manager.update_smart_approve_permission(&tc.name, level);
                }

                results.push(InspectionResult {
                    tool_request_id: candidate.id.clone(),
                    action: if is_readonly {
                        InspectionAction::Allow
                    } else {
                        InspectionAction::RequireApproval(None)
                    },
                    reason: if is_readonly {
                        "LLM detected as read-only".to_string()
                    } else {
                        "Tool requires user approval".to_string()
                    },
                    confidence: 1.0, // Permission decisions are definitive
                    inspector_name: self.name().to_string(),
                    finding_id: None,
                });
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::CallToolRequestParams;
    use rmcp::object;
    use std::sync::Arc;
    use test_case::test_case;
    use tokio::sync::Mutex;

    #[test_case(GooseMode::Auto, false, None, InspectionAction::Allow; "auto_allows")]
    #[test_case(GooseMode::SmartApprove, true, None, InspectionAction::Allow; "smart_approve_annotation_allows")]
    #[test_case(GooseMode::SmartApprove, false, Some(PermissionLevel::AlwaysAllow), InspectionAction::Allow; "smart_approve_cached_allow")]
    #[test_case(GooseMode::SmartApprove, false, Some(PermissionLevel::AskBefore), InspectionAction::RequireApproval(None); "smart_approve_cached_ask")]
    #[test_case(GooseMode::SmartApprove, false, None, InspectionAction::RequireApproval(None); "smart_approve_unknown_defers")]
    #[test_case(GooseMode::Approve, false, None, InspectionAction::RequireApproval(None); "approve_requires_approval")]
    #[test_case(GooseMode::Approve, false, Some(PermissionLevel::AlwaysAllow), InspectionAction::RequireApproval(None); "approve_ignores_cache")]
    #[tokio::test]
    async fn test_inspect_action(
        mode: GooseMode,
        smart_approved: bool,
        cache: Option<PermissionLevel>,
        expected: InspectionAction,
    ) {
        let pm = Arc::new(PermissionManager::new(tempfile::tempdir().unwrap().keep()));
        if let Some(level) = cache {
            pm.update_smart_approve_permission("tool", level);
        }
        let inspector = PermissionInspector::new(pm, Arc::new(Mutex::new(None)));
        if smart_approved {
            *inspector.readonly_tools.write().unwrap() = ["tool".to_string()].into_iter().collect();
        }
        let req = ToolRequest {
            id: "req".into(),
            tool_call: Ok(CallToolRequestParams::new("tool").with_arguments(object!({}))),
            metadata: None,
            tool_meta: None,
        };
        let results = inspector
            .inspect(goose_test_support::TEST_SESSION_ID, &[req], &[], mode)
            .await
            .unwrap();
        assert_eq!(results[0].action, expected);
    }
}
