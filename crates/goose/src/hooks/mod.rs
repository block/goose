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
    /// Tracks which ContextFill thresholds have already fired this session.
    /// Prevents re-firing every turn while above the threshold.
    fired_context_thresholds: std::sync::Mutex<std::collections::HashSet<u32>>,
}

impl Hooks {
    pub fn load(working_dir: &Path) -> Self {
        let settings = HookSettingsFile::load_merged(working_dir).unwrap_or_else(|e| {
            tracing::debug!("No hooks config loaded: {}", e);
            HookSettingsFile::default()
        });
        Self {
            settings,
            fired_context_thresholds: std::sync::Mutex::new(std::collections::HashSet::new()),
        }
    }

    /// Check context fill and fire ContextFill hooks for any thresholds that have been crossed.
    /// Returns context to inject, if any.
    ///
    /// Call this once per turn in the agent loop with the current token count.
    pub async fn check_context_fill(
        &self,
        session_id: &str,
        current_tokens: usize,
        context_limit: usize,
        extension_manager: &crate::agents::extension_manager::ExtensionManager,
        working_dir: &Path,
        cancel_token: CancellationToken,
    ) -> Option<String> {
        if context_limit == 0 {
            return None;
        }

        let fill_pct = ((current_tokens as f64 / context_limit as f64) * 100.0) as u32;

        // Get configured ContextFill thresholds from settings
        let event_configs = self
            .settings
            .get_hooks_for_event(HookEventKind::ContextFill);
        if event_configs.is_empty() {
            return None;
        }

        // Find thresholds that are newly crossed
        let mut new_thresholds = Vec::new();
        {
            let mut fired = self
                .fired_context_thresholds
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            for config in event_configs {
                if let Some(pattern) = &config.matcher {
                    if let Ok(threshold) = pattern.parse::<u32>() {
                        if fill_pct >= threshold && !fired.contains(&threshold) {
                            fired.insert(threshold);
                            new_thresholds.push(threshold);
                        }
                    }
                }
            }
        }

        if new_thresholds.is_empty() {
            return None;
        }

        // Fire hooks for each newly crossed threshold.
        // fill_percentage is set to the threshold value (not the actual fill) so that
        // matches_config can use exact equality to route to the correct config entry.
        // The actual fill level is derivable from current_tokens / context_limit.
        let mut all_context = Vec::new();
        for threshold in new_thresholds {
            tracing::info!(
                "Context fill {}% crossed threshold {}%, firing hooks",
                fill_pct,
                threshold
            );
            let invocation = HookInvocation::context_fill(
                session_id.to_string(),
                current_tokens,
                context_limit,
                threshold,
                working_dir.to_string_lossy().to_string(),
            );
            if let Ok(outcome) = self
                .run(
                    invocation,
                    extension_manager,
                    working_dir,
                    cancel_token.clone(),
                )
                .await
            {
                if let Some(ctx) = outcome.context {
                    all_context.push(ctx);
                }
            }
        }

        if all_context.is_empty() {
            None
        } else {
            Some(all_context.join("\n"))
        }
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
                .is_some_and(|t| Self::regex_matches(pattern, t)),
            PreCompact | PostCompact => {
                (invocation.manual_compact && pattern == "manual")
                    || (!invocation.manual_compact && pattern == "auto")
            }
            ContextFill => {
                // Matcher is a threshold percentage (e.g., "70").
                // Exact equality: check_context_fill sets fill_percentage to the
                // specific threshold being fired (not the current fill level),
                // preventing double-execution when multiple thresholds cross at once.
                if let (Ok(threshold), Some(fill)) =
                    (pattern.parse::<u32>(), invocation.fill_percentage)
                {
                    fill == threshold
                } else {
                    false
                }
            }
            _ => true,
        }
    }

    /// Match a tool invocation against a Claude Code-style matcher pattern.
    ///
    /// Supports regex patterns (Claude Code compat):
    ///   "Bash" — maps to developer__shell or shell
    ///   "Bash(regex)" — developer__shell/shell with command content regex
    ///   "Edit|Write" — regex alternation matching tool names
    ///   "mcp__memory__.*" — regex wildcard matching MCP tool names
    ///   "developer__shell" — exact match (also valid regex)
    fn matches_tool(pattern: &str, invocation: &HookInvocation) -> bool {
        let tool_name = match &invocation.tool_name {
            Some(name) => name,
            None => return false,
        };

        // Claude Code "Bash" / "Bash(pattern)" syntax
        if pattern == "Bash" {
            return tool_name == "developer__shell" || tool_name == "shell";
        }

        if let Some(inner) = pattern
            .strip_prefix("Bash(")
            .and_then(|s| s.strip_suffix(')'))
        {
            if tool_name != "developer__shell" && tool_name != "shell" {
                return false;
            }
            let command_str = invocation
                .tool_input
                .as_ref()
                .and_then(|v| v.get("command"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            return Self::regex_matches(inner, command_str);
        }

        // Regex match against tool name (Claude Code compat: "Edit|Write", "mcp__.*", etc.)
        Self::regex_matches(pattern, tool_name)
    }

    /// Test if `text` matches `pattern` as a full-string regex.
    /// Anchors the pattern to match the entire string (not a substring).
    fn regex_matches(pattern: &str, text: &str) -> bool {
        let anchored = if pattern.starts_with('^') || pattern.ends_with('$') {
            pattern.to_string()
        } else {
            format!("^(?:{})$", pattern)
        };
        regex::Regex::new(&anchored)
            .map(|re| re.is_match(text))
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

    // -- regex_matches: Claude Code-compatible tool matching --

    #[test]
    fn regex_alternation_matches_either_tool() {
        assert!(Hooks::regex_matches("Edit|Write", "Edit"));
        assert!(Hooks::regex_matches("Edit|Write", "Write"));
        assert!(!Hooks::regex_matches("Edit|Write", "Read"));
    }

    #[test]
    fn regex_wildcard_matches_mcp_tools() {
        assert!(Hooks::regex_matches(
            "mcp__memory__.*",
            "mcp__memory__create_entities"
        ));
        assert!(Hooks::regex_matches(
            "mcp__memory__.*",
            "mcp__memory__search"
        ));
        assert!(!Hooks::regex_matches(
            "mcp__memory__.*",
            "mcp__filesystem__read"
        ));
    }

    #[test]
    fn regex_exact_string_still_works() {
        assert!(Hooks::regex_matches("developer__shell", "developer__shell"));
        assert!(!Hooks::regex_matches(
            "developer__shell",
            "developer__shell_extra"
        ));
    }

    #[test]
    fn regex_invalid_pattern_returns_false() {
        assert!(!Hooks::regex_matches("[invalid", "anything"));
    }
}
