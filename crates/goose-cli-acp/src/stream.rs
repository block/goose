use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use sacp::schema::{
    CancelNotification, RequestPermissionOutcome, RequestPermissionRequest,
    RequestPermissionResponse, ToolCallId,
};
use sacp::{JrRequestCx, SessionMessage};

use crate::dispatch;
use crate::mode::SessionMode;
use crate::permissions::{self, PermissionHandler};
use crate::wire::to_sacp_error;

/// How long to wait for the server to acknowledge cancellation before giving up.
const CANCEL_DRAIN_TIMEOUT: Duration = Duration::from_secs(5);

struct RawModeGuard;

impl RawModeGuard {
    fn acquire() -> Result<Self, sacp::Error> {
        crossterm::terminal::enable_raw_mode().map_err(to_sacp_error)?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StoredToolOutput {
    pub id: usize,
    pub title: String,
    pub output: String,
}

pub(crate) type PermissionMsg = (
    RequestPermissionRequest,
    JrRequestCx<RequestPermissionResponse>,
);

/// Per-turn borrowed state. Created fresh each turn from SessionState fields.
pub(crate) struct TurnState<'a> {
    pub(crate) perm_rx: &'a mut tokio::sync::mpsc::UnboundedReceiver<PermissionMsg>,
    pub(crate) perm_handler: &'a mut PermissionHandler,
    pub(crate) active_tools: &'a mut HashMap<ToolCallId, (Instant, Option<String>)>,
    pub(crate) mode: &'a mut SessionMode,
    pub(crate) tool_outputs: &'a mut VecDeque<StoredToolOutput>,
    /// Monotonic counter for tool output display IDs. Never resets, survives eviction.
    pub(crate) next_tool_id: &'a mut usize,
}

pub(crate) async fn stream_turn<Link>(
    session: &mut sacp::ActiveSession<'_, Link>,
    ctx: &mut TurnState<'_>,
) -> Result<(), sacp::Error>
where
    Link: sacp::JrLink + sacp::HasPeer<sacp::AgentPeer>,
{
    let mut pending_perm: Option<PermissionMsg> = None;

    let _raw_guard = if ctx.mode.uses_raw_mode() {
        Some(RawModeGuard::acquire()?)
    } else {
        None
    };

    let result = stream_loop(session, ctx, &mut pending_perm).await;

    if let Some((_req, req_cx)) = pending_perm.take() {
        let _ = req_cx.respond(RequestPermissionResponse::new(
            RequestPermissionOutcome::Cancelled,
        ));
    }
    while let Ok((_req, req_cx)) = ctx.perm_rx.try_recv() {
        let _ = req_cx.respond(RequestPermissionResponse::new(
            RequestPermissionOutcome::Cancelled,
        ));
    }

    if ctx.mode.is_interactive() {
        eprintln!();
    }
    result
}

fn spawn_key_poller() -> tokio::sync::mpsc::UnboundedReceiver<char> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::task::spawn_blocking(move || {
        while !tx.is_closed() {
            if let Some(key) = permissions::poll_permission_key(Duration::from_millis(20)) {
                if tx.send(key).is_err() {
                    break;
                }
            }
        }
    });
    rx
}

async fn stream_loop<Link>(
    session: &mut sacp::ActiveSession<'_, Link>,
    ctx: &mut TurnState<'_>,
    pending_perm: &mut Option<PermissionMsg>,
) -> Result<(), sacp::Error>
where
    Link: sacp::JrLink + sacp::HasPeer<sacp::AgentPeer>,
{
    let mut key_rx = if ctx.mode.uses_raw_mode() {
        Some(spawn_key_poller())
    } else {
        None
    };

    loop {
        let uses_raw = ctx.mode.uses_raw_mode();
        let is_interactive = ctx.mode.is_interactive();
        let has_active_tools = !ctx.active_tools.is_empty();

        let cancel = tokio::select! {
            update = session.read_update() => {
                match update? {
                    SessionMessage::SessionMessage(msg) => {
                        dispatch::handle_message(msg, ctx).await?;
                    }
                    SessionMessage::StopReason(_) => {
                        if ctx.mode.uses_raw_mode() {
                            ctx.mode.finish();
                        } else {
                            use std::io::Write;
                            let _ = std::io::stdout().flush();
                        }
                        return Ok(());
                    }
                    _ => {}
                }
                false
            }
            Some((req, req_cx)) = ctx.perm_rx.recv() => {
                if uses_raw {
                    if let Some(outcome) = ctx.perm_handler.try_resolve(&req) {
                        let _ = req_cx.respond(RequestPermissionResponse::new(outcome));
                    } else {
                        if let Some((_old_req, old_cx)) = pending_perm.take() {
                            let _ = old_cx.respond(RequestPermissionResponse::new(
                                RequestPermissionOutcome::Cancelled,
                            ));
                        }
                        let title = req.tool_call.fields.title.as_deref().unwrap_or("unknown");
                        let input = req.tool_call.fields.raw_input.as_ref();
                        ctx.mode.render_permission_prompt(title, input);
                        *pending_perm = Some((req, req_cx));
                    }
                } else if ctx.mode.is_interactive() {
                    // Plain interactive mode: line-based prompt (no raw mode).
                    // spawn_blocking avoids stalling the async runtime on stdin.
                    let outcome = match ctx.perm_handler.try_resolve(&req) {
                        Some(o) => o,
                        None => {
                            let title = req.tool_call.fields.title.as_deref().unwrap_or("unknown").to_string();
                            let key = tokio::task::spawn_blocking(move || {
                                permissions::prompt_permission_line(&title)
                            }).await.unwrap_or('\x1b');
                            ctx.perm_handler.resolve_with_key(&req, key)
                        }
                    };
                    let _ = req_cx.respond(RequestPermissionResponse::new(outcome));
                } else {
                    // Pipe mode: no interaction possible, cancel
                    let _ = req_cx.respond(RequestPermissionResponse::new(
                        RequestPermissionOutcome::Cancelled,
                    ));
                }
                false
            }
            Some(key) = async {
                match key_rx.as_mut() {
                    Some(rx) => rx.recv().await,
                    None => std::future::pending().await,
                }
            }, if is_interactive => {
                if let Some((req, req_cx)) = pending_perm.take() {
                    // Permission prompt is active — route key to handler (Esc → Cancelled)
                    let outcome = ctx.perm_handler.resolve_with_key(&req, key);
                    let _ = req_cx.respond(RequestPermissionResponse::new(outcome));
                    false
                } else if key == '\x1b' {
                    // No permission prompt — Esc cancels the entire turn
                    true
                } else {
                    false
                }
            }
            _ = tokio::signal::ctrl_c(), if !uses_raw => {
                true
            }
            _ = tokio::time::sleep(Duration::from_millis(80)), if uses_raw && has_active_tools => {
                ctx.mode.update_spinner();
                false
            }
        };

        if cancel {
            ctx.mode.clear_spinner_and_finish();
            drain_remaining(session, ctx, pending_perm).await;
            return Ok(());
        }
    }
}

async fn drain_remaining<Link>(
    session: &mut sacp::ActiveSession<'_, Link>,
    ctx: &mut TurnState<'_>,
    pending_perm: &mut Option<PermissionMsg>,
) where
    Link: sacp::JrLink + sacp::HasPeer<sacp::AgentPeer>,
{
    let _ = session.connection_cx().send_notification_to(
        sacp::AgentPeer,
        CancelNotification::new(session.session_id().clone()),
    );

    if let Some((_req, req_cx)) = pending_perm.take() {
        let _ = req_cx.respond(RequestPermissionResponse::new(
            RequestPermissionOutcome::Cancelled,
        ));
    }
    loop {
        tokio::select! {
            update = session.read_update() => {
                match update {
                    Ok(SessionMessage::StopReason(_)) | Err(_) => break,
                    _ => {}
                }
            }
            Some((_req, req_cx)) = ctx.perm_rx.recv() => {
                let _ = req_cx.respond(RequestPermissionResponse::new(
                    RequestPermissionOutcome::Cancelled,
                ));
            }
            _ = tokio::time::sleep(CANCEL_DRAIN_TIMEOUT) => break,
        }
    }
}
