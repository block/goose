mod config;
mod shell;
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
    pub fn load(working_dir: &Path) -> Self {
        let settings = HookSettingsFile::load_merged(working_dir).unwrap_or_else(|e| {
            tracing::debug!("No hooks config loaded: {}", e);
            HookSettingsFile::default()
        });
        Self { settings }
    }

    pub async fn run(
        &self,
        invocation: HookInvocation,
        extension_manager: &crate::agents::extension_manager::ExtensionManager,
        working_dir: &Path,
        cancel_token: CancellationToken,
    ) -> Result<HooksOutcome> {
        let event_configs = self.settings.get_hooks_for_event(invocation.event);

        let mut outcome = HooksOutcome::default();
        let mut contexts = Vec::new();

        for config in event_configs {
            if !Self::matches_config(config, &invocation) {
                continue;
            }

            for action in &config.hooks {
                match Self::execute_action(
                    action,
                    &invocation,
                    extension_manager,
                    working_dir,
                    cancel_token.clone(),
                )
                .await
                {
                    Ok(Some(result)) => {
                        if let Some(HookDecision::Block) = result.decision {
                            if invocation.event.can_block() {
                                outcome.blocked = true;
                                outcome.reason = result.reason.clone();
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

    // Dispatches hook actions directly via ExtensionManager (McpTool) or subprocess (Command),
    // bypassing tool inspection and approval prompts. This is intentional: hooks are a privileged
    // execution path configured by the user (global) or opted-in (project). Running hooks through
    // the normal tool pipeline would cause infinite recursion (PreToolUse → hook → tool → PreToolUse).
    async fn execute_action(
        action: &HookAction,
        invocation: &HookInvocation,
        extension_manager: &crate::agents::extension_manager::ExtensionManager,
        working_dir: &Path,
        cancel_token: CancellationToken,
    ) -> Result<Option<HookResult>> {
        match action {
            HookAction::Command { command, timeout } => {
                Self::execute_command(command, *timeout, invocation, working_dir, cancel_token)
                    .await
            }
            HookAction::McpTool {
                tool,
                arguments,
                timeout,
            } => {
                Self::execute_mcp_tool(
                    tool,
                    arguments,
                    *timeout,
                    invocation,
                    extension_manager,
                    working_dir,
                    cancel_token,
                )
                .await
            }
        }
    }

    /// Execute a hook command as a direct subprocess.
    async fn execute_command(
        command: &str,
        timeout: u64,
        invocation: &HookInvocation,
        working_dir: &Path,
        cancel_token: CancellationToken,
    ) -> Result<Option<HookResult>> {
        let effective_timeout = if timeout == 0 { 600 } else { timeout };
        let stdin_json = serde_json::to_string(invocation)?;
        let output = shell::run_hook_command(
            command,
            Some(&stdin_json),
            effective_timeout,
            working_dir,
            cancel_token,
        )
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

        if output.timed_out {
            tracing::warn!("Hook timed out after {}s, failing open", effective_timeout);
            return Ok(None);
        }

        Self::parse_command_output(output, invocation.event)
    }

    /// Parse the output of a hook command into a HookResult.
    ///
    /// Exit code semantics:
    ///   0 → success. Stdout is parsed as JSON HookResult, or treated as additionalContext.
    ///   2 → block (on blockable events). Stderr is the block reason.
    ///   other → fail open (warning logged, hook result ignored).
    ///   None (signal) → fail open.
    fn parse_command_output(
        output: shell::HookCommandOutput,
        event: HookEventKind,
    ) -> Result<Option<HookResult>> {
        match output.exit_code {
            Some(0) => {
                let stdout = output.stdout.trim();
                if stdout.is_empty() {
                    Ok(Some(HookResult::default()))
                } else {
                    // Try JSON first, fall back to plain text as additionalContext
                    Ok(Some(
                        serde_json::from_str::<HookResult>(stdout).unwrap_or_else(|_| {
                            let mut context = stdout.to_string();
                            if context.len() > 32_768 {
                                tracing::warn!(
                                    "Hook stdout truncated from {} to 32KB",
                                    context.len()
                                );
                                context.truncate(context.floor_char_boundary(32_768));
                            }
                            HookResult {
                                additional_context: Some(context),
                                ..Default::default()
                            }
                        }),
                    ))
                }
            }
            Some(2) if event.can_block() => {
                // Exit 2 → block. Stderr is the error message (Claude Code compat)
                let reason = {
                    let s = output.stderr.trim();
                    if s.is_empty() {
                        None
                    } else {
                        // Cap stderr at 4KB for block reason
                        let mut r = s.to_string();
                        if r.len() > 4096 {
                            r.truncate(r.floor_char_boundary(4096));
                        }
                        Some(r)
                    }
                };
                Ok(Some(HookResult {
                    decision: Some(HookDecision::Block),
                    reason,
                    ..Default::default()
                }))
            }
            Some(code) => {
                tracing::warn!("Hook exited with code {}, failing open", code);
                Ok(None)
            }
            None => {
                tracing::warn!("Hook terminated without exit code, failing open");
                Ok(None)
            }
        }
    }

    /// Execute a hook via MCP tool dispatch through ExtensionManager.
    async fn execute_mcp_tool(
        tool: &str,
        arguments: &serde_json::Map<String, serde_json::Value>,
        timeout: u64,
        invocation: &HookInvocation,
        extension_manager: &crate::agents::extension_manager::ExtensionManager,
        working_dir: &Path,
        cancel_token: CancellationToken,
    ) -> Result<Option<HookResult>> {
        // Guard zero timeout — default to 10 minutes
        let effective_timeout = if timeout == 0 { 600 } else { timeout };

        let tool_call = CallToolRequestParams {
            meta: None,
            task: None,
            name: tool.to_string().into(),
            arguments: Some(arguments.clone()),
        };

        let tool_call_result = extension_manager
            .dispatch_tool_call(
                &invocation.session_id,
                tool_call,
                Some(working_dir),
                cancel_token.clone(),
            )
            .await?;

        tokio::select! {
            result = tokio::time::timeout(Duration::from_secs(effective_timeout), tool_call_result.result) => {
                match result {
                    Ok(Ok(call_result)) => Self::parse_mcp_result(call_result, invocation.event),
                    Ok(Err(e)) => {
                        tracing::warn!("Hook MCP tool call failed: {}, failing open", e);
                        Ok(None)
                    }
                    Err(_) => {
                        tracing::warn!("Hook MCP tool timed out after {}s, failing open", effective_timeout);
                        Ok(None)
                    }
                }
            }
            _ = cancel_token.cancelled() => {
                tracing::info!("Hook cancelled by session cancellation");
                Ok(None)
            }
        }
    }

    /// Parse the result of an MCP tool call into a HookResult.
    fn parse_mcp_result(
        result: CallToolResult,
        event: HookEventKind,
    ) -> Result<Option<HookResult>> {
        // Suppress unused variable warning — event is kept for future use and API symmetry
        let _ = event;

        if result.is_error.unwrap_or(false) {
            tracing::warn!("Hook MCP tool returned error, failing open");
            return Ok(None);
        }

        let text = result
            .content
            .iter()
            .filter_map(|c| c.as_text().map(|t| t.text.as_str()))
            .collect::<Vec<_>>()
            .join("");

        if text.trim().is_empty() {
            Ok(Some(HookResult::default()))
        } else {
            Ok(Some(serde_json::from_str(text.trim()).unwrap_or_else(
                |e| {
                    tracing::debug!("MCP hook output is not HookResult JSON: {}", e);
                    let mut context = text.trim().to_string();
                    if context.len() > 32_768 {
                        tracing::warn!("MCP hook output truncated from {} to 32KB", context.len());
                        context.truncate(context.floor_char_boundary(32_768));
                    }
                    HookResult {
                        additional_context: Some(context),
                        ..Default::default()
                    }
                },
            )))
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
            PreCompact | PostCompact => {
                (invocation.manual_compact && pattern == "manual")
                    || (!invocation.manual_compact && pattern == "auto")
            }
            _ => true,
        }
    }

    /// Match a tool invocation against a Claude Code-style matcher pattern.
    /// Supports:
    ///   "Bash" or "Bash(...)" — maps to developer__shell, optionally matching command content
    ///   "tool_name" — direct tool name match (goose-native)
    fn matches_tool(pattern: &str, invocation: &HookInvocation) -> bool {
        let tool_name = match &invocation.tool_name {
            Some(name) => name,
            None => return false,
        };

        // Claude Code "Bash" / "Bash(pattern)" syntax
        if pattern == "Bash" {
            return tool_name == "developer__shell";
        }

        if let Some(inner) = pattern
            .strip_prefix("Bash(")
            .and_then(|s| s.strip_suffix(')'))
        {
            if tool_name != "developer__shell" {
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
        tool_name == pattern
    }

    fn glob_match(pattern: &str, text: &str) -> bool {
        glob::Pattern::new(pattern)
            .map(|p| p.matches(text))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use types::{HookDecision, HookEventKind};

    fn make_output(stdout: &str, stderr: &str, exit_code: Option<i32>) -> shell::HookCommandOutput {
        shell::HookCommandOutput {
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
            exit_code,
            timed_out: false,
        }
    }

    // -- parse_command_output: the core exit-code contract --

    #[test]
    fn exit_0_empty_stdout_approves() {
        let output = make_output("", "", Some(0));
        let result = Hooks::parse_command_output(output, HookEventKind::PreToolUse)
            .unwrap()
            .unwrap();
        assert!(result.decision.is_none());
        assert!(result.additional_context.is_none());
    }

    #[test]
    fn exit_0_json_parsed_as_hook_result() {
        let json = r#"{"decision":"block","reason":"tests must pass"}"#;
        let output = make_output(json, "", Some(0));
        let result = Hooks::parse_command_output(output, HookEventKind::PreToolUse)
            .unwrap()
            .unwrap();
        assert_eq!(result.decision, Some(HookDecision::Block));
        assert_eq!(result.reason.as_deref(), Some("tests must pass"));
    }

    #[test]
    fn exit_0_non_json_becomes_additional_context() {
        let output = make_output("plain text from hook", "", Some(0));
        let result = Hooks::parse_command_output(output, HookEventKind::SessionStart)
            .unwrap()
            .unwrap();
        assert_eq!(
            result.additional_context.as_deref(),
            Some("plain text from hook")
        );
        assert!(result.decision.is_none());
    }

    #[test]
    fn exit_0_large_stdout_truncated_to_32kb() {
        let big = "x".repeat(40_000);
        let output = make_output(&big, "", Some(0));
        let result = Hooks::parse_command_output(output, HookEventKind::PostToolUse)
            .unwrap()
            .unwrap();
        let ctx = result.additional_context.unwrap();
        assert!(ctx.len() <= 32_768);
    }

    #[test]
    fn exit_2_blocks_on_blockable_event() {
        let output = make_output("ignored stdout", "rm is dangerous", Some(2));
        let result = Hooks::parse_command_output(output, HookEventKind::PreToolUse)
            .unwrap()
            .unwrap();
        assert_eq!(result.decision, Some(HookDecision::Block));
        assert_eq!(result.reason.as_deref(), Some("rm is dangerous"));
    }

    #[test]
    fn exit_2_stderr_capped_at_4kb() {
        let big_err = "e".repeat(8_000);
        let output = make_output("", &big_err, Some(2));
        let result = Hooks::parse_command_output(output, HookEventKind::UserPromptSubmit)
            .unwrap()
            .unwrap();
        assert_eq!(result.decision, Some(HookDecision::Block));
        assert!(result.reason.as_ref().unwrap().len() <= 4096);
    }

    #[test]
    fn exit_2_on_non_blockable_event_fails_open() {
        // SessionStart.can_block() == false
        let output = make_output("", "error", Some(2));
        let result = Hooks::parse_command_output(output, HookEventKind::SessionStart).unwrap();
        assert!(
            result.is_none(),
            "exit 2 on non-blockable event should fail open"
        );
    }

    #[test]
    fn nonzero_non2_exit_fails_open() {
        let output = make_output("some output", "some error", Some(1));
        let result = Hooks::parse_command_output(output, HookEventKind::PreToolUse).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn no_exit_code_fails_open() {
        let output = make_output("", "", None);
        let result = Hooks::parse_command_output(output, HookEventKind::PreToolUse).unwrap();
        assert!(result.is_none());
    }
}
