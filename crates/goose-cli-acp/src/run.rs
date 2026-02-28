use std::collections::{HashMap, VecDeque};
use std::time::Instant;

use anyhow::Result;
use sacp::schema::{InitializeRequest, ProtocolVersion, RequestPermissionOutcome, ToolCallId};
use sacp::{ClientToAgent, JrConnectionCx, JrRequestCx};

use crate::commands;
use crate::mode::SessionMode;
use crate::permissions::PermissionHandler;
use crate::stream::{self, PermissionMsg, StoredToolOutput, TurnState};
use crate::wire::{sacp_error_to_anyhow, to_sacp_error, CallToolRequest, NewWithRecipeRequest};
use crate::{display, input, recipe, session, slash, transport};

use sacp::schema::{RequestPermissionRequest, RequestPermissionResponse};

struct AbortOnDrop(tokio::task::JoinHandle<()>);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

struct SessionState {
    mode: SessionMode,
    perm_handler: PermissionHandler,
    active_tools: HashMap<ToolCallId, (Instant, Option<String>)>,
    tool_outputs: VecDeque<StoredToolOutput>,
    /// Monotonic counter for tool output display IDs. Never resets, survives eviction.
    next_tool_id: usize,
}

impl SessionState {
    fn new(auto_approve: bool, mode: SessionMode) -> Self {
        Self {
            mode,
            perm_handler: PermissionHandler::new(auto_approve),
            active_tools: HashMap::new(),
            tool_outputs: VecDeque::new(),
            next_tool_id: 1,
        }
    }

    fn turn<'a>(
        &'a mut self,
        perm_rx: &'a mut tokio::sync::mpsc::UnboundedReceiver<PermissionMsg>,
    ) -> TurnState<'a> {
        TurnState {
            perm_rx,
            perm_handler: &mut self.perm_handler,
            active_tools: &mut self.active_tools,
            mode: &mut self.mode,
            tool_outputs: &mut self.tool_outputs,
            next_tool_id: &mut self.next_tool_id,
        }
    }
}

async fn setup_session() -> Result<(
    transport::Transport,
    AbortOnDrop,
    tokio::sync::mpsc::UnboundedSender<PermissionMsg>,
    tokio::sync::mpsc::UnboundedReceiver<PermissionMsg>,
)> {
    let agent = transport::create_agent().await?;
    let (transport, server_handle) = transport::serve_in_process(agent).await?;
    let (perm_tx, perm_rx) = tokio::sync::mpsc::unbounded_channel::<PermissionMsg>();
    Ok((transport, AbortOnDrop(server_handle), perm_tx, perm_rx))
}

/// Expands to the `async move` closure body for `on_receive_request`.
/// `AsyncFnMut` is not object-safe, so this must stay inline at each call site.
macro_rules! perm_handler_closure {
    ($perm_tx:expr) => {
        async move |req: RequestPermissionRequest,
                    req_cx: JrRequestCx<RequestPermissionResponse>,
                    _cx: JrConnectionCx<ClientToAgent>| {
            match $perm_tx.send((req, req_cx)) {
                Ok(()) => {}
                Err(tokio::sync::mpsc::error::SendError((_, req_cx))) => {
                    tracing::warn!("permission channel closed, denying request");
                    let _ = req_cx.respond(RequestPermissionResponse::new(
                        RequestPermissionOutcome::Cancelled,
                    ));
                }
            }
            Ok(())
        }
    };
}

pub async fn run(session_id: Option<String>, auto_approve: bool, plain_stream: bool) -> Result<()> {
    let mode = if plain_stream {
        SessionMode::plain()
    } else {
        SessionMode::rich()
    };
    run_inner(session_id, None, auto_approve, mode).await
}

pub async fn run_single_shot(prompt: &str, auto_approve: bool) -> Result<()> {
    run_inner(
        None,
        Some(prompt.to_owned()),
        auto_approve,
        SessionMode::pipe(),
    )
    .await
}

pub async fn run_recipe(path: &std::path::Path, auto_approve: bool) -> Result<()> {
    use sacp::schema::NewSessionResponse;

    let loaded = recipe::load_recipe(path)?;
    let recipe_json = serde_json::to_value(&loaded)?;
    let recipe_prompt = loaded.prompt.clone();
    let cwd = std::env::current_dir()?
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("non-UTF-8 current directory"))?
        .to_owned();

    let (transport, server_handle, perm_tx, perm_rx) = setup_session().await?;

    let result = ClientToAgent::builder()
        .on_receive_request(perm_handler_closure!(perm_tx), sacp::on_receive_request!())
        .connect_to(transport)
        .map_err(sacp_error_to_anyhow)?
        .run_until(|cx: JrConnectionCx<ClientToAgent>| async move {
            cx.send_request(InitializeRequest::new(ProtocolVersion::LATEST))
                .block_task()
                .await?;

            let response = cx
                .send_request_to(
                    sacp::AgentPeer,
                    NewWithRecipeRequest {
                        cwd,
                        recipe: recipe_json,
                        mcp_servers: vec![],
                    },
                )
                .block_task()
                .await?;

            let prompt = response.prompt.or(recipe_prompt);
            let prompt = prompt.ok_or_else(|| {
                sacp::Error::internal_error()
                    .data("Recipe has no prompt — interactive mode not yet supported for recipes")
            })?;

            let session_response = NewSessionResponse::new(response.session_id);
            let mut active_session = cx.attach_session(session_response, vec![])?;

            let mut perm_rx = perm_rx;
            let mut state = SessionState::new(auto_approve, SessionMode::pipe());
            let mut ctx = state.turn(&mut perm_rx);

            active_session.send_prompt(&prompt).map_err(to_sacp_error)?;
            stream::stream_turn(&mut active_session, &mut ctx).await?;

            Ok(())
        })
        .await;

    drop(server_handle);
    result.map_err(sacp_error_to_anyhow)
}

async fn run_inner(
    resume_session: Option<String>,
    single_shot: Option<String>,
    auto_approve: bool,
    mode: SessionMode,
) -> Result<()> {
    let (transport, server_handle, perm_tx, perm_rx) = setup_session().await?;

    let result = ClientToAgent::builder()
        .on_receive_request(perm_handler_closure!(perm_tx), sacp::on_receive_request!())
        .connect_to(transport)
        .map_err(sacp_error_to_anyhow)?
        .run_until(|cx: JrConnectionCx<ClientToAgent>| async move {
            cx.send_request(InitializeRequest::new(ProtocolVersion::LATEST))
                .block_task()
                .await?;

            let mut active_session = if let Some(ref sid) = resume_session {
                session::load_existing_session(&cx, sid).await?
            } else {
                cx.build_session_cwd()?.block_task().start_session().await?
            };

            let mut perm_rx = perm_rx;
            let mut state = SessionState::new(auto_approve, mode);
            let mut ctx = state.turn(&mut perm_rx);

            if let Some(prompt) = single_shot {
                active_session.send_prompt(&prompt).map_err(to_sacp_error)?;
                stream::stream_turn(&mut active_session, &mut ctx).await?;
                return Ok(());
            }

            run_interactive(&mut active_session, &mut ctx).await?;

            Ok(())
        })
        .await;

    drop(server_handle);
    result.map_err(sacp_error_to_anyhow)
}

async fn run_interactive<Link>(
    session: &mut sacp::ActiveSession<'_, Link>,
    ctx: &mut TurnState<'_>,
) -> Result<(), sacp::Error>
where
    Link: sacp::JrLink + sacp::HasPeer<sacp::AgentPeer>,
{
    use goose::config::paths::Paths;

    let history_path = Paths::state_dir().join("acp-history.txt");
    let mut user_commands = commands::load_commands();
    let mut editor = input::create_editor(Some(history_path.clone()), &user_commands);
    let mut prompt = input::GoosePrompt::new();
    let full_sid = session.session_id().0.to_string();
    const SESSION_ID_DISPLAY_LEN: usize = 13; // "ses_" + 9 chars — enough to identify
    prompt.session_id = Some(
        full_sid
            .get(..SESSION_ID_DISPLAY_LEN)
            .unwrap_or(&full_sid)
            .to_string(),
    );

    eprintln!("{}", display::style::banner());
    session::poll_session_data(session, &mut prompt).await;

    loop {
        prompt.refresh();
        let (ed, pr, event) = input::read_input(editor, prompt).await;
        editor = ed;
        prompt = pr;

        match event {
            input::InputEvent::Line(line) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                match slash::handle_slash_command(trimmed, &user_commands) {
                    slash::SlashResult::Quit => break,
                    slash::SlashResult::Handled { reload_commands } => {
                        if reload_commands {
                            user_commands = commands::load_commands();
                            editor =
                                input::create_editor(Some(history_path.clone()), &user_commands);
                        }
                        continue;
                    }
                    slash::SlashResult::SendPrompt(body) => {
                        session.send_prompt(&body).map_err(to_sacp_error)?;
                        stream::stream_turn(session, ctx).await?;
                        session::poll_session_data(session, &mut prompt).await;
                        continue;
                    }
                    slash::SlashResult::ToolCall {
                        tool_name,
                        arg_key,
                        body,
                    } => {
                        handle_tool_call(session, tool_name, arg_key, body).await;
                        continue;
                    }
                    slash::SlashResult::Show(req) => {
                        display::handle_show(req, ctx.tool_outputs);
                        continue;
                    }
                    slash::SlashResult::NotHandled => {}
                }

                session.send_prompt(trimmed).map_err(to_sacp_error)?;
                stream::stream_turn(session, ctx).await?;
                session::poll_session_data(session, &mut prompt).await;
            }
            input::InputEvent::CtrlC => {
                display::print_hint("Use Ctrl+D or /exit to quit");
            }
            input::InputEvent::CtrlD => {
                break;
            }
        }
    }

    Ok(())
}

async fn handle_tool_call<Link>(
    session: &mut sacp::ActiveSession<'_, Link>,
    tool_name: String,
    arg_key: String,
    body: String,
) where
    Link: sacp::JrLink + sacp::HasPeer<sacp::AgentPeer>,
{
    let mut arguments = serde_json::Map::new();
    arguments.insert(arg_key, serde_json::Value::String(body));
    let session_id = session.session_id().0.to_string();
    match session
        .connection_cx()
        .send_request_to(
            sacp::AgentPeer,
            CallToolRequest {
                session_id,
                tool_name,
                arguments,
            },
        )
        .block_task()
        .await
    {
        Ok(resp) => {
            for content in &resp.content {
                if let Some(text) = content.get("text").and_then(|v| v.as_str()) {
                    let clean = display::sanitize_control_chars(text);
                    eprintln!("{clean}");
                }
            }
            if resp.is_error == Some(true) {
                display::print_hint("tool call returned an error");
            }
        }
        Err(e) => {
            display::print_hint(&format!("tool call failed: {e}"));
        }
    }
}
