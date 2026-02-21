mod config;
pub mod types;

pub use config::{HookAction, HookEventConfig, HookSettingsFile};
pub use types::{HookDecision, HookEventKind, HookInvocation, HookResult, HooksOutcome};

use anyhow::Result;
use rmcp::model::{CallToolRequestParams, CallToolResult};
use std::path::Path;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub struct Hooks {
    settings: HookSettingsFile,
}

impl Hooks {
    pub fn load(working_dir: &Path) -> Result<Self> {
        let settings = HookSettingsFile::load_merged(working_dir)?;
        Ok(Self { settings })
    }

    pub async fn run(
        &self,
        invocation: HookInvocation,
        extension_manager: &crate::agents::extension_manager::ExtensionManager,
        working_dir: &Path,
    ) -> Result<HooksOutcome> {
        let event_configs = self.settings.get_hooks_for_event(invocation.event);

        let mut outcome = HooksOutcome::default();
        let mut contexts = Vec::new();

        for config in event_configs {
            if !Self::matches_config(config, &invocation) {
                continue;
            }

            for action in &config.hooks {
                match Self::execute_action(action, &invocation, extension_manager, working_dir)
                    .await
                {
                    Ok(Some(result)) => {
                        if let Some(HookDecision::Block) = result.decision {
                            if invocation.event.can_block() {
                                outcome.blocked = true;
                                tracing::info!("Hook blocked event {:?}", invocation.event);
                                return Ok(outcome);
                            }
                            tracing::warn!(
                                "Hook returned Block for non-blockable event {:?}, ignoring",
                                invocation.event
                            );
                        }

                        if let Some(context) = result.additional_context {
                            contexts.push(context);
                        }
                    }
                    Ok(None) => {
                        tracing::debug!("Hook returned no result, continuing");
                    }
                    Err(e) => {
                        tracing::warn!("Hook execution failed: {}, continuing", e);
                    }
                }
            }
        }

        if !contexts.is_empty() {
            outcome.context = Some(contexts.join("\n"));
        }

        Ok(outcome)
    }

    pub async fn fire(
        &self,
        invocation: HookInvocation,
        extension_manager: &crate::agents::extension_manager::ExtensionManager,
        working_dir: &Path,
    ) {
        if let Err(e) = self.run(invocation, extension_manager, working_dir).await {
            tracing::warn!("Hook fire failed: {}", e);
        }
    }

    // Dispatches hook actions directly via ExtensionManager, bypassing tool inspection
    // and approval prompts. This is intentional: hooks are a privileged execution path
    // configured by the user (global) or opted-in (project). Running hooks through the
    // normal tool pipeline would cause infinite recursion (PreToolUse → hook → tool → PreToolUse).
    async fn execute_action(
        action: &HookAction,
        invocation: &HookInvocation,
        extension_manager: &crate::agents::extension_manager::ExtensionManager,
        working_dir: &Path,
    ) -> Result<Option<HookResult>> {
        let (tool_call, timeout_secs) = Self::build_tool_call(action, invocation)?;
        let cancel_token = CancellationToken::new();

        let tool_call_result = extension_manager
            .dispatch_tool_call(
                &invocation.session_id,
                tool_call,
                Some(working_dir),
                cancel_token.clone(),
            )
            .await?;

        let result =
            tokio::time::timeout(Duration::from_secs(timeout_secs), tool_call_result.result).await;

        match result {
            Ok(Ok(call_result)) => Self::parse_result(call_result, action, invocation.event),
            Ok(Err(e)) => {
                tracing::warn!("Hook tool call failed: {}, failing open", e);
                Ok(None)
            }
            Err(_) => {
                cancel_token.cancel();
                tracing::warn!("Hook timed out after {}s, failing open", timeout_secs);
                Ok(None)
            }
        }
    }

    fn build_tool_call(
        action: &HookAction,
        invocation: &HookInvocation,
    ) -> Result<(CallToolRequestParams, u64)> {
        match action {
            HookAction::Command { command, timeout } => {
                let json = serde_json::to_string(invocation)?;
                let escaped = json.replace('\'', "'\\''");
                let shell_cmd = format!(
                    "printf '%s' '{}' | {}; printf '\\nGOOSE_HOOK_EXIT:%d' $?",
                    escaped, command
                );

                let args = serde_json::json!({"command": shell_cmd});
                Ok((
                    CallToolRequestParams {
                        meta: None,
                        task: None,
                        name: "developer__shell".into(),
                        arguments: args.as_object().cloned(),
                    },
                    *timeout,
                ))
            }
            HookAction::McpTool {
                tool,
                arguments,
                timeout,
            } => Ok((
                CallToolRequestParams {
                    meta: None,
                    task: None,
                    name: tool.clone().into(),
                    arguments: Some(arguments.clone()),
                },
                *timeout,
            )),
        }
    }

    fn parse_result(
        result: CallToolResult,
        action: &HookAction,
        event: HookEventKind,
    ) -> Result<Option<HookResult>> {
        if result.is_error.unwrap_or(false) {
            tracing::warn!("Hook tool returned error, failing open");
            return Ok(None);
        }

        let text = result
            .content
            .iter()
            .filter_map(|c| c.as_text().map(|t| t.text.as_str()))
            .collect::<Vec<_>>()
            .join("");

        match action {
            HookAction::Command { .. } => {
                if text.is_empty() {
                    return Ok(Some(HookResult::default()));
                }

                if let Some(exit_marker) = text.rfind("GOOSE_HOOK_EXIT:") {
                    let (output, exit_part) = text.split_at(exit_marker);
                    if let Some(code_str) = exit_part.strip_prefix("GOOSE_HOOK_EXIT:") {
                        if let Ok(code) = code_str
                            .split_whitespace()
                            .next()
                            .unwrap_or("")
                            .parse::<i32>()
                        {
                            return match code {
                                0 => {
                                    if output.trim().is_empty() {
                                        Ok(Some(HookResult::default()))
                                    } else {
                                        Ok(serde_json::from_str(output.trim())
                                            .map(Some)
                                            .unwrap_or_else(|_| {
                                                // Non-JSON stdout from exit-0 command → surface as context (Claude Code compat)
                                                let mut context = output.trim().to_string();
                                                if context.len() > 32_768 {
                                                    tracing::warn!(
                                                        "Hook stdout truncated from {} to 32KB",
                                                        context.len()
                                                    );
                                                    context.truncate(
                                                        context.floor_char_boundary(32_768),
                                                    );
                                                }
                                                Some(HookResult {
                                                    additional_context: Some(context),
                                                    ..Default::default()
                                                })
                                            }))
                                    }
                                }
                                2 if event.can_block() => Ok(Some(HookResult {
                                    decision: Some(HookDecision::Block),
                                    ..Default::default()
                                })),
                                _ => Ok(None),
                            };
                        }
                    }
                }

                // No exit marker — can't confirm exit 0, so don't surface raw output
                Ok(serde_json::from_str(text.trim())
                    .map(Some)
                    .unwrap_or_else(|e| {
                        tracing::debug!("Hook output is not JSON (no exit marker): {}", e);
                        None
                    }))
            }
            HookAction::McpTool { .. } => {
                if text.trim().is_empty() {
                    Ok(Some(HookResult::default()))
                } else {
                    Ok(serde_json::from_str(text.trim())
                        .map(Some)
                        .unwrap_or_else(|e| {
                            tracing::debug!("Hook output is not HookResult JSON: {}", e);
                            None
                        }))
                }
            }
        }
    }

    fn matches_config(config: &HookEventConfig, invocation: &HookInvocation) -> bool {
        let Some(pattern) = &config.matcher else {
            return true;
        };

        use HookEventKind::*;
        match invocation.event {
            PreToolUse | PostToolUse | PostToolUseFailure | PermissionRequest => {
                Self::matches_tool(pattern, invocation)
            }
            Notification => invocation
                .notification_type
                .as_ref()
                .is_some_and(|t| t.contains(pattern)),
            PreCompact | PostCompact => invocation.manual_compact.is_some_and(|manual| {
                (manual && pattern == "manual") || (!manual && pattern == "auto")
            }),
            _ => true,
        }
    }

    /// Match a tool invocation against a Claude Code-style matcher pattern.
    /// Supports:
    ///   "Bash" or "Bash(...)" — maps to developer__shell, optionally matching command content
    ///   "tool_name" — direct tool name substring match (goose-native)
    fn matches_tool(pattern: &str, invocation: &HookInvocation) -> bool {
        let tool_name = match &invocation.tool_name {
            Some(name) => name,
            None => return false,
        };

        // Claude Code "Bash" / "Bash(pattern)" syntax
        if pattern == "Bash" {
            return tool_name.contains("shell");
        }

        if let Some(inner) = pattern
            .strip_prefix("Bash(")
            .and_then(|s| s.strip_suffix(')'))
        {
            if !tool_name.contains("shell") {
                return false;
            }
            // Match the inner pattern against the command argument
            let command_str = invocation
                .tool_input
                .as_ref()
                .and_then(|v| v.get("command"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            return Self::glob_match(inner, command_str);
        }

        // Direct tool name match (goose-native: "developer__shell", "slack__post_message", etc.)
        tool_name.contains(pattern)
    }

    /// Simple glob matching: only supports trailing * (prefix match).
    /// "git push*" matches "git push origin main"
    /// "git push" matches exactly "git push"
    fn glob_match(pattern: &str, text: &str) -> bool {
        if let Some(prefix) = pattern.strip_suffix('*') {
            text.starts_with(prefix)
        } else {
            text == pattern
        }
    }
}
