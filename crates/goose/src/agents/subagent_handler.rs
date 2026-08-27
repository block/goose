use crate::{
    agents::{
        subagent_task_config::TaskConfig, Agent, AgentConfig, AgentEvent, ExtensionLoadResult,
        SessionConfig,
    },
    conversation::{
        message::{Message, MessageContent},
        Conversation,
    },
    prompt_template::render_template,
    recipe::Recipe,
};
use anyhow::{anyhow, Result};
use futures::StreamExt;
use rmcp::model::{ErrorCode, ErrorData, Notification, ServerNotification};
#[expect(deprecated)]
use rmcp::model::{LoggingLevel, LoggingMessageNotificationParam};
use serde::Serialize;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info};

pub type OnMessageCallback = Arc<dyn Fn(&Message) + Send + Sync>;

#[derive(Serialize)]
pub struct SubagentPromptContext {
    pub max_turns: usize,
    pub subagent_id: String,
    pub task_instructions: String,
    pub tool_count: usize,
    pub available_tools: String,
}

type AgentMessagesFuture = Pin<
    Box<
        dyn Future<Output = Result<(Conversation, Option<String>, Vec<ExtensionLoadResult>)>>
            + Send,
    >,
>;

/// Bound and redact an extension-attach error before surfacing it to the
/// parent LLM.
///
/// `add_extension()` failures can wrap the MCP process's complete stderr
/// via `ExtensionError::ProcessExit`, which may contain secrets logged
/// during startup and is unbounded in length. Neither is safe to hand back
/// verbatim in the tool response.
///
/// The full unredacted error remains available via the `debug!` log at
/// the attach site — this function only sanitises the model-facing copy.
fn sanitize_extension_error(msg: &str) -> String {
    const MAX_LEN: usize = 500;
    const SECRET_MARKERS: &[&str] = &[
        "TOKEN", "KEY=", "SECRET", "PASSWORD", "AWS_", "BEARER", "API_KEY",
    ];

    let filtered: String = msg
        .lines()
        .filter(|line| {
            let upper = line.to_uppercase();
            !SECRET_MARKERS.iter().any(|m| upper.contains(m))
        })
        .collect::<Vec<_>>()
        .join(" ");

    if filtered.chars().count() > MAX_LEN {
        let truncated: String = filtered.chars().take(MAX_LEN).collect();
        format!("{truncated}… (truncated; full error in logs)")
    } else if filtered.trim().is_empty() {
        "(extension error redacted; see logs)".to_string()
    } else {
        filtered
    }
}

pub struct SubagentRunParams {
    pub config: AgentConfig,
    pub recipe: Recipe,
    pub task_config: TaskConfig,
    pub return_last_only: bool,
    pub session_id: String,
    pub cancellation_token: Option<CancellationToken>,
    pub on_message: Option<OnMessageCallback>,
    pub notification_tx: Option<tokio::sync::mpsc::UnboundedSender<ServerNotification>>,
}

/// Output of a subagent run.
///
/// Carries the text a caller would previously have seen as the bare `String`
/// return, plus a per-extension load status so the parent (LLM or orchestrator)
/// can tell when a delegate ran with fewer tools than requested.
#[derive(Debug, Clone)]
pub struct SubagentRunResult {
    pub text: String,
    pub extension_load_results: Vec<ExtensionLoadResult>,
}

impl SubagentRunResult {
    /// Format the extension-load results as a human- and LLM-readable block,
    /// suitable for appending to a delegate tool response.
    ///
    /// Returns `None` when every requested extension loaded successfully or
    /// when no extensions were requested — in that case callers should emit
    /// the plain `text` output unchanged.
    pub fn format_extension_load_report(&self) -> Option<String> {
        if self.extension_load_results.is_empty()
            || self.extension_load_results.iter().all(|r| r.success)
        {
            return None;
        }

        let mut lines = String::from("Subagent extension load results:");
        for result in &self.extension_load_results {
            if result.success {
                lines.push_str(&format!("\n  loaded: {}", result.name));
            } else {
                let err = result.error.as_deref().unwrap_or("unknown error");
                lines.push_str(&format!("\n  failed: {} ({})", result.name, err));
            }
        }
        Some(lines)
    }

    /// Return the tool-response text a caller should surface to the parent,
    /// appending the extension-load report only when at least one extension
    /// failed to attach.
    ///
    /// This variant is not safe to use when the delegated recipe declared
    /// a `response:` schema, because the appended prose invalidates the
    /// serialised JSON output. In that case call `into_parts()` instead
    /// and let the caller decide how to surface the load results.
    pub fn text_with_report(&self) -> String {
        match self.format_extension_load_report() {
            Some(report) => format!("{}\n\n{}", self.text, report),
            None => self.text.clone(),
        }
    }

    /// Structured accessor. Prefer this at consume sites that must
    /// preserve the raw text output — e.g. recipes with a `response:`
    /// schema whose final output is serialised JSON that must remain
    /// parseable by the caller.
    ///
    /// The returned `Vec<ExtensionLoadResult>` still carries the drop
    /// information the caller can surface separately (via structured
    /// metadata or a log record) without mutating the text.
    pub fn into_parts(self) -> (String, Vec<ExtensionLoadResult>) {
        (self.text, self.extension_load_results)
    }
}

pub async fn run_subagent_task(
    params: SubagentRunParams,
) -> Result<SubagentRunResult, anyhow::Error> {
    let return_last_only = params.return_last_only;
    let (messages, final_output, extension_load_results) =
        get_agent_messages(params).await.map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to execute task: {}", e),
                None,
            )
        })?;

    let text = match final_output {
        Some(output) => output,
        None => extract_response_text(&messages, return_last_only),
    };

    Ok(SubagentRunResult {
        text,
        extension_load_results,
    })
}

fn extract_response_text(messages: &Conversation, return_last_only: bool) -> String {
    if return_last_only {
        messages
            .messages()
            .last()
            .and_then(|message| {
                message.content.iter().find_map(|content| match content {
                    crate::conversation::message::MessageContent::Text(text_content) => {
                        Some(text_content.text.clone())
                    }
                    _ => None,
                })
            })
            .unwrap_or_else(|| String::from("No text content in last message"))
    } else {
        let all_text_content: Vec<String> = messages
            .iter()
            .flat_map(|message| {
                message.content.iter().filter_map(|content| match content {
                    crate::conversation::message::MessageContent::Text(text_content) => {
                        Some(text_content.text.clone())
                    }
                    crate::conversation::message::MessageContent::ToolResponse(tool_response) => {
                        if let Ok(result) = &tool_response.tool_result {
                            let texts: Vec<String> = result
                                .content
                                .iter()
                                .filter_map(|content| {
                                    if let rmcp::model::ContentBlock::Text(raw_text_content) =
                                        content
                                    {
                                        Some(raw_text_content.text.clone())
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            if !texts.is_empty() {
                                Some(format!("Tool result: {}", texts.join("\n")))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    }
                    _ => None,
                })
            })
            .collect();

        all_text_content.join("\n")
    }
}

pub const SUBAGENT_TOOL_REQUEST_TYPE: &str = "subagent_tool_request";

fn get_agent_messages(params: SubagentRunParams) -> AgentMessagesFuture {
    Box::pin(async move {
        let SubagentRunParams {
            config,
            recipe,
            task_config,
            session_id,
            cancellation_token,
            on_message,
            notification_tx,
            ..
        } = params;

        let system_instructions = recipe.instructions.clone().unwrap_or_default();
        let user_task = recipe
            .prompt
            .clone()
            .unwrap_or_else(|| "Begin.".to_string());

        let agent = Arc::new(Agent::with_config(config));

        agent
            .update_provider(
                task_config.provider.clone(),
                task_config.model_config.clone(),
                &session_id,
            )
            .await
            .map_err(|e| anyhow!("Failed to set provider on sub agent: {}", e))?;

        // Start with any drops that build_task_config recorded before the
        // attach step (e.g. requested extension names not active in the
        // parent session). Then append per-extension results from the
        // attach loop below. The parent sees a single unified report.
        let mut extension_load_results: Vec<ExtensionLoadResult> =
            task_config.pre_attach_load_results.clone();
        extension_load_results.reserve(task_config.extensions.len());
        for extension in &task_config.extensions {
            let name = extension.name();
            match agent.add_extension(extension.clone(), &session_id).await {
                Ok(_) => extension_load_results.push(ExtensionLoadResult {
                    name,
                    success: true,
                    error: None,
                }),
                Err(e) => {
                    let raw = e.to_string();
                    debug!("Failed to add extension '{}' to subagent: {}", name, raw);
                    extension_load_results.push(ExtensionLoadResult {
                        name,
                        success: false,
                        error: Some(sanitize_extension_error(&raw)),
                    });
                }
            }
        }

        let has_response_schema = recipe.response.is_some();
        agent
            .apply_recipe_components(recipe.response.clone(), true)
            .await?;

        let subagent_prompt =
            build_subagent_prompt(&agent, &task_config, &session_id, system_instructions).await?;
        agent.override_system_prompt(subagent_prompt).await;

        let user_message = Message::user().with_text(user_task);
        let mut conversation = Conversation::new_unvalidated(vec![user_message.clone()]);

        agent
            .config
            .session_manager
            .update(&session_id)
            .recipe(Some(recipe.clone()))
            .apply()
            .await?;

        if let Some(activities) = recipe.activities {
            for activity in activities {
                info!("Recipe activity: {}", activity);
            }
        }
        let session_config = SessionConfig {
            id: session_id.clone(),
            schedule_id: None,
            max_turns: task_config.max_turns.map(|v| v as u32),
            retry_config: recipe.retry,
        };

        let mut stream =
            crate::session_context::with_session_id(Some(session_id.to_string()), async {
                agent
                    .reply(user_message, session_config, cancellation_token)
                    .await
            })
            .await
            .map_err(|e| anyhow!("Failed to get reply from agent: {}", e))?;

        while let Some(message_result) = stream.next().await {
            match message_result {
                Ok(AgentEvent::Message(msg)) => {
                    if let Some(ref callback) = on_message {
                        callback(&msg);
                    }
                    if let Some(ref tx) = notification_tx {
                        for content in &msg.content {
                            if let Some(notif) = create_tool_notification(content, &session_id) {
                                if tx.send(notif).is_err() {
                                    debug!(
                                        "Notification receiver dropped for subagent {}",
                                        session_id
                                    );
                                }
                            }
                        }
                    }
                    conversation.push(msg);
                }
                Ok(AgentEvent::Usage(_)) => {}
                Ok(AgentEvent::MessageUsage { .. }) => {}
                Ok(AgentEvent::McpNotification(_)) => {}
                Ok(AgentEvent::HistoryReplaced(updated_conversation)) => {
                    conversation = updated_conversation;
                }
                Err(e) => {
                    tracing::error!("Error receiving message from subagent: {}", e);
                    break;
                }
            }
        }

        let final_output = get_final_output(&agent, has_response_schema).await;

        Ok((conversation, final_output, extension_load_results))
    })
}

async fn build_subagent_prompt(
    agent: &Agent,
    task_config: &TaskConfig,
    session_id: &str,
    system_instructions: String,
) -> Result<String> {
    let tools: Vec<_> = agent
        .list_tools(session_id, None)
        .await
        .into_iter()
        .filter(super::reply_parts::is_tool_visible_to_model)
        .collect();
    render_template(
        "subagent_system.md",
        &SubagentPromptContext {
            max_turns: task_config
                .max_turns
                .expect("TaskConfig always sets max_turns"),
            subagent_id: session_id.to_string(),
            task_instructions: system_instructions,
            tool_count: tools.len(),
            available_tools: tools
                .iter()
                .map(|t| t.name.to_string())
                .collect::<Vec<_>>()
                .join(", "),
        },
    )
    .map_err(|e| anyhow!("Failed to render subagent system prompt: {}", e))
}

async fn get_final_output(agent: &Agent, has_response_schema: bool) -> Option<String> {
    if has_response_schema {
        agent
            .final_output_tool
            .lock()
            .await
            .as_ref()
            .and_then(|tool| tool.final_output.clone())
    } else {
        None
    }
}

#[expect(deprecated)]
pub fn create_tool_notification(
    content: &MessageContent,
    subagent_id: &str,
) -> Option<ServerNotification> {
    if let MessageContent::ToolRequest(req) = content {
        let tool_call = req.tool_call.as_ref().ok()?;

        Some(ServerNotification::LoggingMessageNotification(
            Notification::new(
                LoggingMessageNotificationParam::new(
                    LoggingLevel::Info,
                    serde_json::json!({
                        "type": SUBAGENT_TOOL_REQUEST_TYPE,
                        "subagent_id": subagent_id,
                        "tool_call": {
                            "name": tool_call.name,
                            "arguments": tool_call.arguments
                        }
                    }),
                )
                .with_logger(format!("subagent:{}", subagent_id)),
            ),
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{
        create_tool_notification, sanitize_extension_error, SubagentRunResult,
        SUBAGENT_TOOL_REQUEST_TYPE,
    };
    use crate::agents::ExtensionLoadResult;
    use crate::conversation::message::MessageContent;
    use rmcp::model::{CallToolRequestParams, ServerNotification};
    use serde_json::json;

    #[test]
    #[expect(deprecated)]
    fn create_tool_notification_for_tool_request() {
        let tool_call = CallToolRequestParams::new("developer__shell".to_string())
            .with_arguments(json!({"command": "ls"}).as_object().unwrap().clone());
        let content = MessageContent::tool_request("req1", Ok(tool_call));
        let notification =
            create_tool_notification(&content, "session_1").expect("expected notification");

        let ServerNotification::LoggingMessageNotification(log_notif) = notification else {
            panic!("expected logging notification");
        };
        let data = log_notif
            .params
            .data
            .as_object()
            .expect("expected object data");
        assert_eq!(
            data.get("type").and_then(|v| v.as_str()),
            Some(SUBAGENT_TOOL_REQUEST_TYPE)
        );
        assert_eq!(
            data.get("subagent_id").and_then(|v| v.as_str()),
            Some("session_1")
        );
        let tool_call = data
            .get("tool_call")
            .and_then(|v| v.as_object())
            .expect("expected tool_call object");
        assert_eq!(
            tool_call.get("name").and_then(|v| v.as_str()),
            Some("developer__shell")
        );
    }

    #[test]
    fn create_tool_notification_ignores_non_tool_request() {
        let content = MessageContent::text("hello");
        assert!(create_tool_notification(&content, "session_1").is_none());
    }

    #[test]
    fn subagent_run_result_no_report_when_all_extensions_succeed() {
        let result = SubagentRunResult {
            text: "subagent output".to_string(),
            extension_load_results: vec![
                ExtensionLoadResult {
                    name: "developer".to_string(),
                    success: true,
                    error: None,
                },
                ExtensionLoadResult {
                    name: "computercontroller".to_string(),
                    success: true,
                    error: None,
                },
            ],
        };

        assert!(result.format_extension_load_report().is_none());
        assert_eq!(result.text_with_report(), "subagent output");
    }

    #[test]
    fn subagent_run_result_no_report_when_no_extensions_requested() {
        let result = SubagentRunResult {
            text: "subagent output".to_string(),
            extension_load_results: Vec::new(),
        };

        assert!(result.format_extension_load_report().is_none());
        assert_eq!(result.text_with_report(), "subagent output");
    }

    #[test]
    fn subagent_run_result_reports_partial_extension_failures() {
        let result = SubagentRunResult {
            text: "subagent output".to_string(),
            extension_load_results: vec![
                ExtensionLoadResult {
                    name: "developer".to_string(),
                    success: true,
                    error: None,
                },
                ExtensionLoadResult {
                    name: "memory".to_string(),
                    success: false,
                    error: Some("extension not registered".to_string()),
                },
            ],
        };

        let report = result
            .format_extension_load_report()
            .expect("report expected when any extension fails");
        assert!(report.contains("Subagent extension load results"));
        assert!(report.contains("loaded: developer"));
        assert!(report.contains("failed: memory (extension not registered)"));

        let combined = result.text_with_report();
        assert!(combined.starts_with("subagent output"));
        assert!(combined.contains("failed: memory"));
    }

    #[test]
    fn subagent_run_result_reports_all_extension_failures() {
        let result = SubagentRunResult {
            text: "subagent output".to_string(),
            extension_load_results: vec![
                ExtensionLoadResult {
                    name: "memory".to_string(),
                    success: false,
                    error: Some("not registered".to_string()),
                },
                ExtensionLoadResult {
                    name: "sqlite".to_string(),
                    success: false,
                    error: None,
                },
            ],
        };

        let report = result
            .format_extension_load_report()
            .expect("report expected when all extensions fail");
        assert!(report.contains("failed: memory (not registered)"));
        assert!(report.contains("failed: sqlite (unknown error)"));
        assert!(!report.contains("loaded:"));
    }

    #[test]
    fn sanitize_extension_error_redacts_secret_lines() {
        // MCP process stderr can carry secrets logged at startup. Any line
        // matching a known secret marker is dropped from the model-facing
        // copy; the rest is preserved so the diagnostic remains useful.
        // NOTE: fake, non-functional test values only. Real access keys
        // would never appear in a test file. The scanner recognises the
        // AWS_ / BEARER prefixes, which is exactly what this test wants
        // to prove the sanitiser catches.
        let secret_value = format!("test-{}-fake", "not-a-real-key");
        let raw = format!(
            "Process exited with status 1\n\
             Loading config from /etc/mcp/config\n\
             AWS_ACCESS_KEY_ID={secret_value}\n\
             BEARER test-fake-token-value\n\
             Failed to connect to upstream server"
        );
        let sanitised = sanitize_extension_error(&raw);
        assert!(!sanitised.contains(&secret_value));
        assert!(!sanitised.contains("test-fake-token-value"));
        assert!(!sanitised.contains("AWS_"));
        assert!(!sanitised.contains("BEARER"));
        assert!(sanitised.contains("Process exited"));
        assert!(sanitised.contains("Failed to connect"));
    }

    #[test]
    fn sanitize_extension_error_truncates_long_messages() {
        // Model-facing copies are bounded so a runaway stderr can't blow
        // up the tool response. Truncation preserves UTF-8 codepoints
        // (chars-based, not byte-based) and appends a hint pointing to
        // the debug! log for the full error.
        let raw = "x".repeat(1000);
        let sanitised = sanitize_extension_error(&raw);
        assert!(sanitised.chars().count() <= 550);
        assert!(sanitised.ends_with("(truncated; full error in logs)"));
    }

    #[test]
    fn sanitize_extension_error_returns_sentinel_when_all_lines_redacted() {
        // If every line matched a secret marker, an empty string reads as
        // "attach failed silently" to the parent LLM — the exact bug the
        // PR is trying to prevent. Emit an explicit sentinel instead.
        let raw = "AWS_SECRET_ACCESS_KEY=abc123\nTOKEN=xyz789";
        let sanitised = sanitize_extension_error(raw);
        assert_eq!(sanitised, "(extension error redacted; see logs)");
    }

    #[test]
    fn sanitize_extension_error_preserves_normal_messages() {
        // Normal error strings from the attach path (e.g. "not registered")
        // are short and secret-free — they should pass through unchanged
        // so the parent sees the actionable diagnostic verbatim.
        let raw = "extension \"foo\" not registered";
        let sanitised = sanitize_extension_error(raw);
        assert_eq!(sanitised, "extension \"foo\" not registered");
    }

    #[test]
    fn subagent_run_result_into_parts_returns_text_and_results_separately() {
        // Recipes with a `response:` schema serialise their final output
        // as JSON. Callers on that path must be able to reach the raw
        // text without any appended report, and still surface the drop
        // information out-of-band. `into_parts()` is that boundary.
        let result = SubagentRunResult {
            text: r#"{"answer": 42}"#.to_string(),
            extension_load_results: vec![ExtensionLoadResult {
                name: "memory".to_string(),
                success: false,
                error: Some("not registered".to_string()),
            }],
        };

        let (text, results) = result.into_parts();

        // Text is untouched — caller can parse it as JSON.
        assert_eq!(text, r#"{"answer": 42}"#);
        assert!(serde_json::from_str::<serde_json::Value>(&text).is_ok());

        // Load results are still available for structured surfacing.
        assert_eq!(results.len(), 1);
        assert!(!results[0].success);
        assert_eq!(results[0].name, "memory");
    }
}
