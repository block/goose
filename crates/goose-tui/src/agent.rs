//! Bridge between the `goose` library and the UI.
//!
//! `build_agent` initialises the Goose agent exactly as the CLI does.
//! `run_agent_loop` is a long-running tokio task that receives prompts and
//! streams `AgentMsg` events back to the UI.

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use base64::Engine as _;
use futures::StreamExt;
use goose::action_required_manager::ActionRequiredManager;
use goose::agents::{Agent, AgentEvent, SessionConfig};
use goose::config::{get_all_extensions, Config};
use goose::conversation::message::{ActionRequiredData, Message, MessageContent};
use goose::permission::permission_confirmation::{Permission, PermissionConfirmation, PrincipalType};
use goose::providers::create;
use goose::session::session_manager::{SessionManager, SessionType};
use rmcp::model::RawContent;
use std::sync::Arc as StdArc;
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use tracing::warn;

use crate::types::{AgentMsg, ElicitationReq, PermissionChoice, PermissionOption, PermissionReq, ToolCallInfo, ToolStatus};

// ── Public types ──────────────────────────────────────────────────────────────

pub struct AgentHandle {
    pub agent: Arc<Agent>,
    pub session_id: String,
    pub working_dir: std::path::PathBuf,
    pub session_manager: StdArc<SessionManager>,
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

    // Read the session's working directory for display in the UI.
    let working_dir = session_manager
        .get_session(&session_id, false)
        .await
        .map(|s| s.working_dir)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());

    Ok(AgentHandle {
        agent: Arc::new(agent),
        session_id,
        working_dir,
        session_manager,
    })
}

// ── Agent processing loop ─────────────────────────────────────────────────────

/// Long-running tokio task.  Receives prompts on `prompt_rx`, streams
/// `AgentMsg` events back on `event_tx`.
///
/// `cancel_rx` is a channel through which the UI sends a fresh
/// `CancellationToken` for each new turn.  When the token is cancelled the
/// current stream is dropped and an `AgentMsg::Finished` is sent immediately.
pub async fn run_agent_loop(
    handle: AgentHandle,
    mut prompt_rx: mpsc::Receiver<String>,
    event_tx: mpsc::Sender<AgentMsg>,
    mut cancel_rx: mpsc::Receiver<CancellationToken>,
) {
    let AgentHandle { agent, session_id, session_manager, .. } = handle;

    while let Some(prompt) = prompt_rx.recv().await {
        // Drain any stale tokens; keep the latest one sent for this turn.
        let mut token = CancellationToken::new();
        while let Ok(t) = cancel_rx.try_recv() {
            token = t;
        }

        let user_msg = build_user_message(&prompt).await;
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

        process_stream(&agent, stream, &event_tx, &token).await;

        let stop_reason = if token.is_cancelled() { "cancelled" } else { "end_turn" };
        let _ = event_tx
            .send(AgentMsg::Finished { stop_reason: stop_reason.into() })
            .await;

        // Query accumulated token usage for this session and send to UI.
        if let Ok(session) = session_manager.get_session(&session_id, false).await {
            let input  = session.accumulated_input_tokens.unwrap_or(0) as i64;
            let output = session.accumulated_output_tokens.unwrap_or(0) as i64;
            let total  = session.accumulated_total_tokens.unwrap_or(0) as i64;
            let _ = event_tx.send(AgentMsg::TokenUsage { input, output, total }).await;
        }
    }
}

async fn process_stream(
    agent: &Arc<Agent>,
    mut stream: futures::stream::BoxStream<'_, Result<AgentEvent, anyhow::Error>>,
    event_tx: &mpsc::Sender<AgentMsg>,
    cancel: &CancellationToken,
) {
    loop {
        let event = tokio::select! {
            biased;
            _ = cancel.cancelled() => break,
            ev = stream.next() => match ev { Some(e) => e, None => break },
        };
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
            match &ar.data {
                ActionRequiredData::ToolConfirmation { id, tool_name, .. } => {
                    let options = vec![
                        PermissionOption { id: "allow_always".into(), label: "Always allow".into(), key: 'a' },
                        PermissionOption { id: "allow_once".into(),   label: "Allow once".into(),   key: 'y' },
                        PermissionOption { id: "deny_once".into(),    label: "Deny once".into(),    key: 'n' },
                        PermissionOption { id: "deny_always".into(),  label: "Always deny".into(),  key: 'N' },
                    ];
                    let req = PermissionReq { tool_title: tool_name.clone(), options };
                    let (reply_tx, reply_rx) = oneshot::channel::<PermissionChoice>();
                    let _ = event_tx.send(AgentMsg::PermissionRequest(req, reply_tx)).await;

                    let choice = reply_rx.await.unwrap_or(PermissionChoice::Cancelled);
                    let permission = match choice {
                        PermissionChoice::Selected(ref opt_id) => match opt_id.as_str() {
                            "allow_always" => Permission::AlwaysAllow,
                            "allow_once"   => Permission::AllowOnce,
                            "deny_once"    => Permission::DenyOnce,
                            "deny_always"  => Permission::AlwaysDeny,
                            _              => Permission::Cancel,
                        },
                        PermissionChoice::Cancelled => Permission::Cancel,
                    };
                    agent.handle_confirmation(
                        id.clone(),
                        PermissionConfirmation { principal_type: PrincipalType::Tool, permission },
                    ).await;
                }

                ActionRequiredData::Elicitation { id, message, .. } => {
                    // Show a free-text input dialog; submit the response via
                    // ActionRequiredManager (the proper elicitation path).
                    let req = ElicitationReq { id: id.clone(), message: message.clone() };
                    let (reply_tx, reply_rx) = oneshot::channel::<String>();
                    let _ = event_tx.send(AgentMsg::ElicitationRequest(req, reply_tx)).await;

                    let text = reply_rx.await.unwrap_or_default();
                    let user_data = serde_json::Value::String(text);
                    if let Err(e) = ActionRequiredManager::global()
                        .submit_response(id.clone(), user_data)
                        .await
                    {
                        warn!("Failed to submit elicitation response: {e}");
                    }
                }

                ActionRequiredData::ElicitationResponse { .. } => {}
            }
        }

        // Thinking, system notifications, frontend tool requests — no UI
        // representation needed in the TUI.
        _ => {}
    }
}

// ── File attachment helpers ────────────────────────────────────────────────────

/// Recognised image extensions and their MIME types.
const IMAGE_MIME: &[(&str, &str)] = &[
    ("png",  "image/png"),
    ("jpg",  "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("gif",  "image/gif"),
    ("webp", "image/webp"),
];

/// Build a user `Message` from raw prompt text.
///
/// Tokens of the form `@/path/to/file` or `@~/path` are extracted and attached
/// as separate content blocks:
///   • image files (.png/.jpg/.jpeg/.gif/.webp) → `MessageContent::Image` (base64)
///   • all other files → `MessageContent::Text` with a fenced code block
///
/// Unresolvable `@path` tokens (file not found, unreadable) are left as-is in
/// the text and a note is appended so the model understands the intent.
pub async fn build_user_message(text: &str) -> Message {
    let mut message = Message::user();
    let mut remaining = String::new();

    for token in text.split_whitespace() {
        if let Some(raw_path) = token.strip_prefix('@') {
            // Expand leading `~` to the home directory.
            let expanded = if raw_path.starts_with("~/") || raw_path == "~" {
                let home = dirs::home_dir().unwrap_or_default();
                home.join(raw_path.trim_start_matches("~/"))
            } else {
                std::path::PathBuf::from(raw_path)
            };

            match attach_path(&expanded).await {
                Ok(AttachResult::Image { data, mime }) => {
                    message = message.with_image(data, mime);
                }
                Ok(AttachResult::Text { label, content }) => {
                    message = message.with_text(format!("{label}\n```\n{content}\n```"));
                }
                Err(e) => {
                    // Leave the token in the text and add an error note.
                    remaining.push(' ');
                    remaining.push_str(token);
                    remaining.push_str(&format!(" (could not attach: {e})"));
                }
            }
        } else {
            remaining.push(' ');
            remaining.push_str(token);
        }
    }

    let clean = remaining.trim().to_string();
    if !clean.is_empty() {
        message = message.with_text(clean);
    }

    message
}

enum AttachResult {
    Image { data: String, mime: String },
    Text  { label: String, content: String },
}

async fn attach_path(path: &Path) -> anyhow::Result<AttachResult> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if let Some((_, mime)) = IMAGE_MIME.iter().find(|(e, _)| *e == ext.as_str()) {
        let bytes = tokio::fs::read(path).await?;
        let data = base64::engine::general_purpose::STANDARD.encode(&bytes);
        Ok(AttachResult::Image { data, mime: mime.to_string() })
    } else {
        let content = tokio::fs::read_to_string(path).await?;
        let label = format!("File: {}", path.display());
        Ok(AttachResult::Text { label, content })
    }
}
