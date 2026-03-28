//! Bridge between the `goose` library and the UI.
//!
//! `build_agent` initialises the Goose agent exactly as the CLI does.
//! `run_agent_loop` is a long-running tokio task that receives prompts and
//! streams `AgentMsg` events back to the UI.

use std::sync::Arc;

use anyhow::Result;
use futures::StreamExt;
use goose::agents::{Agent, AgentEvent, SessionConfig};
use goose::config::{get_all_extensions, Config};
use goose::conversation::message::{ActionRequiredData, Message, MessageContent};
use goose::permission::permission_confirmation::{Permission, PermissionConfirmation, PrincipalType};
use goose::providers::create;
use goose::session::session_manager::SessionType;
use rmcp::model::RawContent;
use tokio::sync::{mpsc, oneshot};
use tracing::warn;

use crate::types::{AgentMsg, PermissionChoice, PermissionOption, PermissionReq, ToolCallInfo, ToolStatus};

// ── Public types ──────────────────────────────────────────────────────────────

pub struct AgentHandle {
    pub agent: Arc<Agent>,
    pub session_id: String,
}

// ── Initialisation ────────────────────────────────────────────────────────────

/// Build the Goose agent and open (or resume) a session.
pub async fn build_agent(session_id_hint: Option<String>) -> Result<AgentHandle> {
    let config = Config::global();
    let provider_name = config.get_goose_provider()?;
    let model_name = config.get_goose_model()?;

    let model_config = goose::model::ModelConfig::new(&model_name)?
        .with_canonical_limits(&provider_name);

    let extensions: Vec<_> = get_all_extensions()
        .into_iter()
        .filter(|e| e.enabled)
        .map(|e| e.config)
        .collect();

    let provider = create(&provider_name, model_config, extensions.clone()).await?;

    let agent = Agent::new();
    let session_manager = agent.config.session_manager.clone();

    let session_id = if let Some(ref id) = session_id_hint {
        // Resume: verify the session exists, then use its id.
        let session = session_manager.get_session(id, false).await?;
        session.id
    } else {
        let cwd = std::env::current_dir()?;
        let session = session_manager
            .create_session(
                cwd,
                "TUI Session".to_string(),
                SessionType::Terminal,
                agent.config.goose_mode,
            )
            .await?;
        session.id
    };

    agent.update_provider(provider, &session_id).await?;
    agent.update_goose_mode(agent.config.goose_mode, &session_id).await?;

    for ext in extensions {
        if let Err(e) = agent.add_extension(ext, &session_id).await {
            warn!("Extension load failed: {e}");
        }
    }

    Ok(AgentHandle { agent: Arc::new(agent), session_id })
}

// ── Agent processing loop ─────────────────────────────────────────────────────

/// Long-running tokio task.  Receives prompts on `prompt_rx`, streams
/// `AgentMsg` events back on `event_tx`.
pub async fn run_agent_loop(
    handle: AgentHandle,
    mut prompt_rx: mpsc::Receiver<String>,
    event_tx: mpsc::Sender<AgentMsg>,
) {
    let AgentHandle { agent, session_id } = handle;

    while let Some(prompt) = prompt_rx.recv().await {
        let user_msg = Message::user().with_text(prompt);
        let session_config = SessionConfig {
            id: session_id.clone(),
            schedule_id: None,
            max_turns: None,
            retry_config: None,
        };

        let stream = match agent.reply(user_msg, session_config, None).await {
            Ok(s) => s,
            Err(e) => {
                let _ = event_tx.send(AgentMsg::Error(e.to_string())).await;
                continue;
            }
        };

        process_stream(&agent, stream, &event_tx).await;

        let _ = event_tx
            .send(AgentMsg::Finished { stop_reason: "end_turn".into() })
            .await;
    }
}

async fn process_stream(
    agent: &Arc<Agent>,
    mut stream: futures::stream::BoxStream<'_, Result<AgentEvent, anyhow::Error>>,
    event_tx: &mpsc::Sender<AgentMsg>,
) {
    while let Some(event) = stream.next().await {
        match event {
            Ok(AgentEvent::Message(msg)) => {
                for content in msg.content {
                    handle_message_content(agent, content, event_tx).await;
                }
            }
            Ok(AgentEvent::HistoryReplaced(_)) => {}
            Ok(AgentEvent::McpNotification(_)) => {}
            Err(e) => {
                let _ = event_tx.send(AgentMsg::Error(e.to_string())).await;
            }
        }
    }
}

async fn handle_message_content(
    agent: &Arc<Agent>,
    content: MessageContent,
    event_tx: &mpsc::Sender<AgentMsg>,
) {
    match content {
        MessageContent::Text(t) => {
            let _ = event_tx.send(AgentMsg::TextChunk(t.text.clone())).await;
        }

        MessageContent::ToolRequest(req) => {
            let (title, input_preview) = match &req.tool_call {
                Ok(call) => (
                    call.name.to_string(),
                    Some(serde_json::to_string_pretty(&call.arguments).unwrap_or_default()),
                ),
                Err(e) => (format!("error: {e}"), None),
            };
            let info = ToolCallInfo {
                id: req.id.clone(),
                title,
                status: ToolStatus::Running,
                input_preview,
                output_preview: None,
            };
            let _ = event_tx.send(AgentMsg::ToolCallUpdate(info)).await;
        }

        MessageContent::ToolResponse(res) => {
            let (status, output_preview) = match &res.tool_result {
                Ok(result) => {
                    let text = result
                        .content
                        .iter()
                        .filter_map(|c| {
                            if let RawContent::Text(tc) = &c.raw {
                                Some(tc.text.as_str())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    (ToolStatus::Success, Some(text))
                }
                Err(_) => (ToolStatus::Error, None),
            };
            let info = ToolCallInfo {
                id: res.id.clone(),
                title: String::new(), // filled in when matched with the ToolRequest
                status,
                input_preview: None,
                output_preview,
            };
            let _ = event_tx.send(AgentMsg::ToolCallUpdate(info)).await;
        }

        MessageContent::ActionRequired(ar) => {
            // Only handle ToolConfirmation; ignore Elicitation for now.
            let (request_id, tool_title) = match &ar.data {
                ActionRequiredData::ToolConfirmation { id, tool_name, .. } => {
                    (id.clone(), tool_name.clone())
                }
                ActionRequiredData::Elicitation { id, message, .. } => {
                    (id.clone(), message.clone())
                }
                ActionRequiredData::ElicitationResponse { .. } => return,
            };

            let options = vec![
                PermissionOption { id: "allow_always".into(), label: "Always allow".into(), key: 'a' },
                PermissionOption { id: "allow_once".into(),   label: "Allow once".into(),   key: 'y' },
                PermissionOption { id: "deny_once".into(),    label: "Deny once".into(),    key: 'n' },
                PermissionOption { id: "deny_always".into(),  label: "Always deny".into(),  key: 'N' },
            ];

            let req = PermissionReq { tool_title, options };
            let (reply_tx, reply_rx) = oneshot::channel::<PermissionChoice>();
            let _ = event_tx.send(AgentMsg::PermissionRequest(req, reply_tx)).await;

            // Block this async task until the UI sends a reply.
            let choice = reply_rx.await.unwrap_or(PermissionChoice::Cancelled);

            let permission = match choice {
                PermissionChoice::Selected(ref id) => match id.as_str() {
                    "allow_always" => Permission::AlwaysAllow,
                    "allow_once"   => Permission::AllowOnce,
                    "deny_once"    => Permission::DenyOnce,
                    "deny_always"  => Permission::AlwaysDeny,
                    _              => Permission::Cancel,
                },
                PermissionChoice::Cancelled => Permission::Cancel,
            };

            agent
                .handle_confirmation(
                    request_id,
                    PermissionConfirmation {
                        principal_type: PrincipalType::Tool,
                        permission,
                    },
                )
                .await;
        }

        // Thinking, system notifications, frontend tool requests — no UI
        // representation needed in the TUI.
        _ => {}
    }
}
