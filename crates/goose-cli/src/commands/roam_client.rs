//! Thin ACP client UI onto a remote roaming agent.
//!
//! Per design doc §9: `roam connect` is NOT a provider wrapper for a local
//! agent loop. The **host** runs the real agent (its tools, working dir,
//! shell); this side is just an ACP *client* that opens a session, sends
//! prompts, and renders `session/update` notifications to the terminal.
//!
//! We deliberately advertise no client filesystem/terminal capabilities and do
//! not send our local cwd — the host imposes the `share` working directory.

use std::io::Write;

use tokio::io::{AsyncBufReadExt, BufReader};

use agent_client_protocol::schema::v1::{
    ContentBlock, InitializeRequest, RequestPermissionOutcome, RequestPermissionRequest,
    RequestPermissionResponse, SelectedPermissionOutcome, SessionNotification, SessionUpdate,
};
use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::{Agent, Client, ConnectionTo};
use anyhow::Result;
use goose_roaming::RoamingClientStream;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

/// Run an interactive ACP session over an authorized roaming stream.
///
/// The provided `read_prompt` closure supplies successive user prompts; it
/// returns `None` when the user wants to end the session (EOF / quit).
pub async fn run_interactive(stream: RoamingClientStream, agent_label: String) -> Result<()> {
    let RoamingClientStream {
        conn, send, recv, ..
    } = stream;

    let transport = agent_client_protocol::ByteStreams::new(send.compat_write(), recv.compat());

    Client
        .builder()
        .name("goose-roam")
        .on_receive_notification(
            async move |notification: SessionNotification, _cx| {
                render_update(&notification.update);
                Ok(())
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .on_receive_request(
            async move |request: RequestPermissionRequest, responder, _cx| {
                // The host runs the agent, so tool-permission prompts originate
                // there. Present them to the local user and forward the choice.
                let outcome = prompt_permission(&request);
                responder.respond(RequestPermissionResponse::new(outcome))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_with(transport, async move |cx: ConnectionTo<Agent>| {
            let init = cx
                .send_request(InitializeRequest::new(ProtocolVersion::LATEST))
                .block_task()
                .await?;
            eprintln!(
                "connected to remote agent `{agent_label}` (protocol {:?})",
                init.protocol_version
            );
            eprintln!("type a message and press enter; Ctrl-D or /quit to end.\n");

            // ACP requires an absolute cwd. Ideally the HOST imposes its own
            // share working directory and ignores this (design doc §9, tracked
            // as a host-side serve_with_policy change). Until then we send an
            // absolute path so session creation validates; on the same machine
            // this is the connector's cwd.
            let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));
            tracing::debug!(cwd = %cwd.display(), "roam client: creating remote session");
            let result = cx
                .build_session(cwd)
                .block_task()
                .run_until(async |mut session| {
                    tracing::debug!(session_id = ?session.session_id(), "roam client: session created");
                    let mut stdin = BufReader::new(tokio::io::stdin()).lines();
                    loop {
                        eprint!("› ");
                        let _ = std::io::stderr().flush();
                        let line = match stdin.next_line().await {
                            Ok(Some(l)) => l,
                            Ok(None) | Err(_) => break, // EOF
                        };
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }
                        if line == "/quit" || line == "/exit" {
                            break;
                        }
                        session.send_prompt(line)?;
                        // Drain updates until the turn completes; chunks are
                        // rendered live via the notification handler above.
                        let _ = session.read_to_string().await?;
                        println!();
                    }
                    Ok(())
                })
                .await;
            if let Err(e) = &result {
                tracing::warn!("roam client: session ended with error: {e:?}");
            }
            result
        })
        .await?;

    drop(conn);
    Ok(())
}

/// One-shot delegation: open a remote session, send a single task, return the
/// agent's final text response. No interactive loop, no local stdin.
///
/// This is the reusable core a future `roam__delegate` model tool will call.
/// Permission requests are auto-cancelled: a delegated (agent-driven) session
/// must not block waiting for a human, and the caller isn't a person who can
/// answer. Loop/cost safety is the caller's concern (bounded turns/deadline).
pub async fn delegate(stream: RoamingClientStream, task: String) -> Result<String> {
    let RoamingClientStream {
        conn, send, recv, ..
    } = stream;

    let transport = agent_client_protocol::ByteStreams::new(send.compat_write(), recv.compat());
    let collected = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let sink = collected.clone();

    Client
        .builder()
        .name("goose-roam-delegate")
        .on_receive_notification(
            async move |notification: SessionNotification, _cx| {
                if let SessionUpdate::AgentMessageChunk(chunk) = &notification.update {
                    if let ContentBlock::Text(text) = &chunk.content {
                        sink.lock().unwrap().push_str(&text.text);
                    }
                }
                Ok(())
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .on_receive_request(
            async move |_request: RequestPermissionRequest, responder, _cx| {
                // Agent-driven session: never wait on a human. Auto-cancel.
                responder.respond(RequestPermissionResponse::new(
                    RequestPermissionOutcome::Cancelled,
                ))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_with(transport, async move |cx: ConnectionTo<Agent>| {
            cx.send_request(InitializeRequest::new(ProtocolVersion::LATEST))
                .block_task()
                .await?;
            let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));
            cx.build_session(cwd)
                .block_task()
                .run_until(async |mut session| {
                    session.send_prompt(&task)?;
                    let _ = session.read_to_string().await?;
                    Ok(())
                })
                .await
        })
        .await?;

    drop(conn);
    let result = collected.lock().unwrap().clone();
    Ok(result)
}

fn render_update(update: &SessionUpdate) {
    match update {
        SessionUpdate::AgentMessageChunk(chunk) => {
            if let ContentBlock::Text(text) = &chunk.content {
                print!("{}", text.text);
                let _ = std::io::stdout().flush();
            }
        }
        SessionUpdate::AgentThoughtChunk(chunk) => {
            if let ContentBlock::Text(text) = &chunk.content {
                eprint!("\x1b[2m{}\x1b[0m", text.text);
            }
        }
        SessionUpdate::ToolCall(tool_call) => {
            eprintln!("\n🔧 {}", tool_call.title);
        }
        SessionUpdate::ToolCallUpdate(update) => {
            if let Some(status) = &update.fields.status {
                eprintln!("   [{status:?}]");
            }
        }
        _ => {}
    }
}

fn prompt_permission(request: &RequestPermissionRequest) -> RequestPermissionOutcome {
    eprintln!("\n⚠️  the remote agent requests permission:");
    for (i, opt) in request.options.iter().enumerate() {
        eprintln!("   {}) {}", i + 1, opt.name);
    }
    eprint!("choose [1]: ");
    let _ = std::io::stderr().flush();

    let choice = read_line()
        .and_then(|l| l.trim().parse::<usize>().ok())
        .unwrap_or(1);
    let idx = choice
        .saturating_sub(1)
        .min(request.options.len().saturating_sub(1));

    match request.options.get(idx) {
        Some(opt) => RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(
            opt.option_id.clone(),
        )),
        None => RequestPermissionOutcome::Cancelled,
    }
}

fn read_line() -> Option<String> {
    eprint!("› ");
    let _ = std::io::stderr().flush();
    let mut buf = String::new();
    match std::io::stdin().read_line(&mut buf) {
        Ok(0) => None, // EOF
        Ok(_) => Some(buf),
        Err(_) => None,
    }
}
