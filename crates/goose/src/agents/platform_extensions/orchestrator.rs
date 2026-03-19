use crate::agents::extension::PlatformExtensionContext;
use crate::agents::mcp_client::{Error, McpClientTrait};
use crate::agents::tool_execution::ToolCallContext;
use crate::agents::{AgentEvent, SessionConfig};
use crate::config::GooseMode;
use crate::context_mgmt::format_message_for_compacting;
use crate::conversation::message::Message;
use crate::execution::manager::AgentManager;
use crate::providers::base::Provider;
use crate::session::session_manager::SessionType;
use anyhow::Result;
use async_trait::async_trait;
use futures::StreamExt;
use rmcp::model::{
    CallToolResult, Content, Implementation, InitializeResult, JsonObject, ListToolsResult,
    ServerCapabilities, Tool,
};
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub static EXTENSION_NAME: &str = "orchestrator";

const DEFAULT_LIST_LIMIT: usize = 10;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ListSessionsParams {
    /// Filter by session type: "user", "sub_agent", "scheduled", "hidden", "terminal", "gateway".
    /// If omitted, returns all session types.
    session_type: Option<String>,
    /// Maximum number of sessions to return (most recent first). Defaults to 10.
    last_n: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ViewSessionParams {
    /// The session ID to inspect
    session_id: String,
    /// How to view the conversation: "first_last" returns the first and last message,
    /// "summarize" calls the LLM to produce a summary. If omitted, returns first and last.
    mode: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct StartAgentParams {
    /// Working directory for the new agent session
    working_dir: String,
    /// Human-readable name for the session
    name: Option<String>,
    // TODO: add a "model_tier" parameter (e.g. "fast" vs "normal") to let the orchestrator
    // choose between a fast/cheap model and the default one. For now we inherit the
    // orchestrator's own provider and model.
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct SendMessageParams {
    /// The session ID of the agent to send a message to
    session_id: String,
    /// The message text to send
    message: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct InterruptAgentParams {
    /// The session ID of the agent to interrupt
    session_id: String,
}

pub struct OrchestratorClient {
    info: InitializeResult,
    context: PlatformExtensionContext,
}

impl OrchestratorClient {
    pub fn new(context: PlatformExtensionContext) -> Result<Self> {
        let info = InitializeResult::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(
                Implementation::new(EXTENSION_NAME, "1.0.0").with_title("Orchestrator"),
            )
            .with_instructions(
                "Manage agent sessions: list, view, start, send messages, and interrupt agents.",
            );

        Ok(Self { info, context })
    }

    async fn get_agent_manager(&self) -> Result<Arc<AgentManager>, String> {
        AgentManager::instance()
            .await
            .map_err(|e| format!("Failed to get agent manager: {}", e))
    }

    async fn get_provider(&self) -> Result<Arc<dyn Provider>, String> {
        let extension_manager = self
            .context
            .extension_manager
            .as_ref()
            .and_then(|weak| weak.upgrade())
            .ok_or("Extension manager not available")?;

        let provider_guard = extension_manager.get_provider().lock().await;
        provider_guard
            .as_ref()
            .cloned()
            .ok_or_else(|| "Provider not available".to_string())
    }

    async fn handle_list_sessions(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<CallToolResult, String> {
        let type_filter = arguments
            .as_ref()
            .and_then(|args| args.get("session_type"))
            .and_then(|v| v.as_str());

        let limit = arguments
            .as_ref()
            .and_then(|args| args.get("last_n"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(DEFAULT_LIST_LIMIT);

        let manager = self.get_agent_manager().await?;

        let mut sessions = if let Some(type_str) = type_filter {
            let session_type: SessionType = type_str
                .parse()
                .map_err(|e| format!("Invalid session type '{}': {}", type_str, e))?;
            self.context
                .session_manager
                .list_sessions_by_types(&[session_type])
                .await
                .map_err(|e| format!("Failed to list sessions: {}", e))?
        } else {
            self.context
                .session_manager
                .list_sessions()
                .await
                .map_err(|e| format!("Failed to list sessions: {}", e))?
        };

        // Most recent first
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        let total = sessions.len();
        sessions.truncate(limit);

        if sessions.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "No sessions found.",
            )]));
        }

        let active_ids = manager.list_active_session_ids().await;

        let mut lines = vec![format!(
            "Showing {} of {} session(s):\n",
            sessions.len(),
            total
        )];
        for session in &sessions {
            let is_loaded = active_ids.contains(&session.id);
            let is_busy = if is_loaded {
                manager.is_session_busy(&session.id).await
            } else {
                false
            };

            let status = if is_busy {
                "🔄 busy"
            } else if is_loaded {
                "✓ loaded"
            } else {
                "○ idle"
            };

            lines.push(format!(
                "- **{}** ({})\n  Type: {} | Status: {} | Messages: {} | Updated: {}",
                session.name,
                session.id,
                session.session_type,
                status,
                session.message_count,
                session.updated_at.format("%Y-%m-%d %H:%M"),
            ));
        }

        Ok(CallToolResult::success(vec![Content::text(
            lines.join("\n"),
        )]))
    }

    async fn handle_view_session(
        &self,
        session_id_for_llm: &str,
        arguments: Option<JsonObject>,
    ) -> Result<CallToolResult, String> {
        let args = arguments.ok_or("Missing arguments")?;
        let session_id = extract_string(&args, "session_id")?;
        let mode = args
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("first_last");

        let session = self
            .context
            .session_manager
            .get_session(&session_id, true)
            .await
            .map_err(|e| format!("Session '{}' not found: {}", session_id, e))?;

        let manager = self.get_agent_manager().await?;
        let is_busy = manager.is_session_busy(&session_id).await;

        let mut output = vec![format!(
            "# Session: {} ({})\n\nType: {} | Status: {} | Working dir: {}\nMessages: {} | Updated: {}\n",
            session.name,
            session.id,
            session.session_type,
            if is_busy { "🔄 busy" } else { "idle" },
            session.working_dir.display(),
            session.message_count,
            session.updated_at.format("%Y-%m-%d %H:%M"),
        )];

        match mode {
            "first_last" => {
                if let Some(conversation) = &session.conversation {
                    let messages = conversation.messages();
                    if messages.is_empty() {
                        output.push("No messages in this session.".to_string());
                    } else {
                        output.push("## First message\n".to_string());
                        output.push(format_message_for_compacting(&messages[0]));

                        if messages.len() > 1 {
                            output.push(format!("\n*({} messages omitted)*\n", messages.len() - 2));
                            output.push("## Last message\n".to_string());
                            output
                                .push(format_message_for_compacting(&messages[messages.len() - 1]));
                        }
                    }
                } else {
                    output.push("No messages in this session.".to_string());
                }
            }
            "summarize" => {
                if let Some(conversation) = &session.conversation {
                    let messages = conversation.messages();
                    if messages.is_empty() {
                        output.push("No messages to summarize.".to_string());
                    } else {
                        let summary = self
                            .summarize_conversation(session_id_for_llm, messages)
                            .await?;
                        output.push(format!("## Summary\n\n{}", summary));
                    }
                } else {
                    output.push("No messages to summarize.".to_string());
                }
            }
            other => {
                return Err(format!(
                    "Unknown mode '{}'. Use 'first_last' or 'summarize'.",
                    other
                ));
            }
        }

        Ok(CallToolResult::success(vec![Content::text(
            output.join("\n"),
        )]))
    }

    async fn summarize_conversation(
        &self,
        session_id: &str,
        messages: &[Message],
    ) -> Result<String, String> {
        let provider = self.get_provider().await?;

        let conversation_text = messages
            .iter()
            .filter(|m| m.is_agent_visible())
            .map(format_message_for_compacting)
            .collect::<Vec<_>>()
            .join("\n");

        let system =
            "You are a helpful assistant. Summarize the following conversation concisely, \
                       capturing the key topics, decisions, and current state. Be brief.";

        let user_message = Message::user().with_text(format!(
            "Summarize this conversation ({} messages):\n\n{}",
            messages.len(),
            conversation_text
        ));

        let (response, _usage) = provider
            .complete_fast(session_id, system, &[user_message], &[])
            .await
            .map_err(|e| format!("LLM summarization failed: {}", e))?;

        Ok(response
            .content
            .iter()
            .filter_map(|c| {
                if let crate::conversation::message::MessageContent::Text(t) = c {
                    Some(t.text.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n"))
    }

    async fn handle_start_agent(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<CallToolResult, String> {
        let args = arguments.ok_or("Missing arguments")?;
        let working_dir = extract_string(&args, "working_dir")?;
        let name = args
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Orchestrated Agent")
            .to_string();

        let path = PathBuf::from(&working_dir);
        if !path.exists() || !path.is_dir() {
            return Err(format!(
                "Working directory '{}' does not exist or is not a directory",
                working_dir
            ));
        }

        let mode = GooseMode::default();

        let session = self
            .context
            .session_manager
            .create_session(path, name.clone(), SessionType::User, mode)
            .await
            .map_err(|e| format!("Failed to create session: {}", e))?;

        let manager = self.get_agent_manager().await?;
        let agent = manager
            .get_or_create_agent(session.id.clone())
            .await
            .map_err(|e| format!("Failed to create agent: {}", e))?;

        // Inherit the orchestrator's provider and model
        let provider = self.get_provider().await?;
        agent
            .update_provider(provider, &session.id)
            .await
            .map_err(|e| format!("Failed to set provider on new agent: {}", e))?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Started agent session '{}' with ID: {}\n\nUse send_message with this session_id to interact with it.",
            name, session.id
        ))]))
    }

    async fn handle_send_message(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<CallToolResult, String> {
        let args = arguments.ok_or("Missing arguments")?;
        let session_id = extract_string(&args, "session_id")?;
        let message_text = extract_string(&args, "message")?;

        let manager = self.get_agent_manager().await?;

        if manager.is_session_busy(&session_id).await {
            return Err(format!(
                "Session '{}' is currently busy processing a request. Use interrupt_agent to cancel it first, or wait for it to finish.",
                session_id
            ));
            // TODO: post the message so it gets picked up on the next turn instead of erroring
        }

        let agent = manager
            .get_or_create_agent(session_id.clone())
            .await
            .map_err(|e| format!("Failed to get agent for session '{}': {}", session_id, e))?;

        // If the agent has no provider yet (e.g. resumed from DB), inherit ours
        if agent.provider().await.is_err() {
            if let Ok(provider) = self.get_provider().await {
                agent
                    .update_provider(provider, &session_id)
                    .await
                    .map_err(|e| format!("Failed to set provider: {}", e))?;
            }
        }

        let cancel_token = CancellationToken::new();
        manager
            .register_cancel_token(&session_id, cancel_token.clone())
            .await;

        let user_message = Message::user().with_text(&message_text);

        let session_config = SessionConfig {
            id: session_id.clone(),
            schedule_id: None,
            max_turns: None,
            retry_config: None,
        };

        let result: Result<String, String> = async {
            let mut stream = agent
                .reply(user_message, session_config, Some(cancel_token))
                .await
                .map_err(|e| format!("Failed to start reply: {}", e))?;

            let mut response_parts: Vec<String> = Vec::new();

            while let Some(event_result) = stream.next().await {
                match event_result {
                    Ok(AgentEvent::Message(msg)) => {
                        let text = msg.as_concat_text();
                        if !text.is_empty() {
                            response_parts.push(text);
                        }
                    }
                    Ok(AgentEvent::HistoryReplaced(_)) | Ok(AgentEvent::McpNotification(_)) => {}
                    Err(e) => {
                        response_parts.push(format!("Error during agent processing: {}", e));
                        break;
                    }
                }
            }

            if response_parts.is_empty() {
                Ok("Agent completed without producing text output.".to_string())
            } else {
                Ok(response_parts.join("\n\n"))
            }
        }
        .await;

        manager.unregister_cancel_token(&session_id).await;

        let response_text = result?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "## Response from session {}\n\n{}",
            session_id, response_text
        ))]))
    }

    async fn handle_interrupt_agent(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<CallToolResult, String> {
        let args = arguments.ok_or("Missing arguments")?;
        let session_id = extract_string(&args, "session_id")?;

        let manager = self.get_agent_manager().await?;

        manager
            .cancel_session(&session_id)
            .await
            .map_err(|e| format!("Failed to interrupt session '{}': {}", session_id, e))?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Interrupted agent session '{}'.",
            session_id
        ))]))
    }
}

#[async_trait]
impl McpClientTrait for OrchestratorClient {
    async fn list_tools(
        &self,
        _session_id: &str,
        _next_cursor: Option<String>,
        _cancel_token: CancellationToken,
    ) -> Result<ListToolsResult, Error> {
        let tools = vec![
            Tool::new(
                "list_sessions".to_string(),
                "List agent sessions with their status (loaded, busy, idle). Returns the most recent 10 by default. Optionally filter by session type."
                    .to_string(),
                schema::<ListSessionsParams>(),
            ),
            Tool::new(
                "view_session".to_string(),
                "View a session's details and conversation. Mode 'first_last' (default) returns the first and last message. Mode 'summarize' calls the LLM to produce a conversation summary."
                    .to_string(),
                schema::<ViewSessionParams>(),
            ),
            Tool::new(
                "start_agent".to_string(),
                "Start a new agent session with its own working directory. Inherits the current provider and model. Returns a session_id for future interaction."
                    .to_string(),
                schema::<StartAgentParams>(),
            ),
            Tool::new(
                "send_message".to_string(),
                "Send a message to an existing agent session and get the response. Returns an error if the agent is currently busy."
                    .to_string(),
                schema::<SendMessageParams>(),
            ),
            Tool::new(
                "interrupt_agent".to_string(),
                "Interrupt a busy agent by cancelling its current operation."
                    .to_string(),
                schema::<InterruptAgentParams>(),
            ),
        ];

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        ctx: &ToolCallContext,
        name: &str,
        arguments: Option<JsonObject>,
        _cancel_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        let result = match name {
            "list_sessions" => self.handle_list_sessions(arguments).await,
            "view_session" => self.handle_view_session(&ctx.session_id, arguments).await,
            "start_agent" => self.handle_start_agent(arguments).await,
            "send_message" => self.handle_send_message(arguments).await,
            "interrupt_agent" => self.handle_interrupt_agent(arguments).await,
            _ => Err(format!("Unknown tool: {}", name)),
        };

        match result {
            Ok(result) => Ok(result),
            Err(error) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error: {}",
                error
            ))])),
        }
    }

    fn get_info(&self) -> Option<&InitializeResult> {
        Some(&self.info)
    }
}

fn schema<T: JsonSchema>() -> JsonObject {
    let mut obj = serde_json::to_value(schema_for!(T))
        .map(|v| v.as_object().unwrap().clone())
        .expect("valid schema");
    obj.entry("properties")
        .or_insert_with(|| serde_json::json!({}));
    obj
}

fn extract_string(args: &JsonObject, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("Missing or invalid '{}'", key))
}
