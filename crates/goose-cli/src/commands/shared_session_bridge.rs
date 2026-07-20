//! Real ACP wire adapter that lets MULTIPLE roaming peers share ONE live goose
//! session (see the "Wire adapter plan" in `goose_roaming::broker`).
//!
//! Unlike [`super::roam_bridge::GooseAcpBridge`], which spawns a fresh agent per
//! accepted connection, this bridge runs a SINGLE ACP client against one live
//! local agent (the *agent-facing half*) and re-serves ACP to every roaming
//! peer (the *peer-facing half*), fanning out `session/update` notifications to
//! all of them and funnelling prompts back, with only the current controller
//! able to answer `session/request_permission`.
//!
//! The transport-neutral routing policy lives in `goose_roaming` (`Router` /
//! `SessionBroker`); this module is the thin ACP plumbing layered on top. It is
//! the only place that couples roaming to the ACP crate, so it lives in the CLI
//! composition layer, keeping `goose-roaming` ACP-free and goose core iroh-free.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use futures::future::BoxFuture;
use futures::io::{AsyncRead, AsyncWrite};
use tokio::sync::{broadcast, mpsc, oneshot, watch, Mutex};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

use agent_client_protocol::schema::v1::{
    InitializeRequest, InitializeResponse, LoadSessionRequest, NewSessionRequest,
    NewSessionResponse, PromptRequest, PromptResponse, RequestPermissionOutcome,
    RequestPermissionRequest, RequestPermissionResponse, SessionId, SessionNotification,
    StopReason,
};
use agent_client_protocol::schema::ProtocolVersion;
use agent_client_protocol::{Agent as SacpAgent, Client, ConnectionTo};

use goose::acp::server::serve;
use goose::acp::server_factory::AcpServer;
use goose_roaming::{
    AcpStreamServer, EndpointId, Refused, Role, Route, Router, Scope, SubscriberId,
};

/// A permission request forwarded from the agent to the controlling peer,
/// paired with a channel to relay the peer's decision back.
type PermRelay = (
    RequestPermissionRequest,
    oneshot::Sender<RequestPermissionResponse>,
);

/// Drives the agent side of an ACP connection over an in-memory byte stream.
///
/// Production uses [`GooseAgentBackend`] (goose's real `serve`); tests supply a
/// stub so the wire adapter can be exercised without a live provider.
pub trait AgentBackend: Send + Sync + 'static {
    fn serve(
        &self,
        recv: Box<dyn AsyncRead + Send + Unpin>,
        send: Box<dyn AsyncWrite + Send + Unpin>,
    ) -> BoxFuture<'static, Result<()>>;
}

/// The default backend: one fresh goose agent per broker, serving ACP over the
/// in-memory duplex the agent-facing client talks to.
pub struct GooseAgentBackend {
    server: Arc<AcpServer>,
}

impl GooseAgentBackend {
    pub fn new(server: Arc<AcpServer>) -> Self {
        Self { server }
    }
}

impl AgentBackend for GooseAgentBackend {
    fn serve(
        &self,
        recv: Box<dyn AsyncRead + Send + Unpin>,
        send: Box<dyn AsyncWrite + Send + Unpin>,
    ) -> BoxFuture<'static, Result<()>> {
        let server = self.server.clone();
        Box::pin(async move {
            let agent = server.create_agent().await?;
            serve(agent, recv, send).await
        })
    }
}

/// Shared broker state, cloned into every peer connection.
struct Shared {
    /// Fan-out source: every `session/update` from the live agent.
    updates: broadcast::Sender<SessionNotification>,
    /// Funnel sink: prompts submitted by peers, drained by the agent-facing half.
    prompts: mpsc::UnboundedSender<String>,
    /// Routing policy + subscriber ids (transport-neutral, from goose-roaming).
    router: Mutex<Router>,
    next_id: Mutex<u64>,
    /// Per-controller channel used to deliver permission requests to the peer
    /// that currently controls the session.
    perm_senders: Mutex<HashMap<SubscriberId, mpsc::UnboundedSender<PermRelay>>>,
    /// The live session's id, published once the agent-facing half starts it.
    session_id: watch::Receiver<Option<SessionId>>,
}

impl Shared {
    async fn alloc_id(&self) -> SubscriberId {
        let mut n = self.next_id.lock().await;
        let id = SubscriberId(*n);
        *n += 1;
        id
    }
}

/// What the shared session should run against: a fresh session, or an existing
/// persisted session resumed by id (its history is replayed into the agent).
#[derive(Debug, Clone)]
pub enum ResumeTarget {
    /// Start a brand-new session in `cwd`.
    New { cwd: PathBuf },
    /// Resume the persisted session `session_id`, activated in `cwd` (its own
    /// working directory).
    Existing { session_id: String, cwd: PathBuf },
}

/// An [`AcpStreamServer`] that shares one live session across all roaming peers.
pub struct SharedSessionBridge {
    shared: Arc<Shared>,
    agent_id: String,
}

impl SharedSessionBridge {
    /// Start the agent-facing half against the given backend and return a bridge
    /// ready to serve peers. Spawns the long-lived ACP client task.
    pub fn start(
        backend: Arc<dyn AgentBackend>,
        agent_id: impl Into<String>,
        target: ResumeTarget,
    ) -> Self {
        let (updates, _) = broadcast::channel(256);
        let (prompt_tx, prompt_rx) = mpsc::unbounded_channel();
        let (session_tx, session_rx) = watch::channel(None);

        let shared = Arc::new(Shared {
            updates: updates.clone(),
            prompts: prompt_tx,
            router: Mutex::new(Router::new()),
            next_id: Mutex::new(0),
            perm_senders: Mutex::new(HashMap::new()),
            session_id: session_rx,
        });

        let agent_shared = shared.clone();
        tokio::spawn(async move {
            if let Err(e) = run_agent_facing(
                backend,
                updates,
                prompt_rx,
                session_tx,
                agent_shared,
                target,
            )
            .await
            {
                tracing::warn!("roaming shared session agent-facing half ended: {e:?}");
            }
        });

        Self {
            shared,
            agent_id: agent_id.into(),
        }
    }
}

impl AcpStreamServer for SharedSessionBridge {
    fn serve_stream(
        &self,
        client: EndpointId,
        scope: Scope,
        recv: Box<dyn AsyncRead + Send + Unpin>,
        send: Box<dyn AsyncWrite + Send + Unpin>,
    ) -> BoxFuture<'static, Result<()>> {
        let shared = self.shared.clone();
        Box::pin(async move {
            tracing::info!(%client, ?scope, "roaming: attaching peer to shared session");
            run_peer_facing(shared, scope, recv, send).await
        })
    }

    fn agent_id(&self) -> String {
        self.agent_id.clone()
    }
}

/// Agent-facing half: one ACP client to the live local agent. Starts (or
/// resumes) a session, pushes every `session/update` into the broadcast, drains
/// funnelled prompts into the session, and relays `session/request_permission`
/// to the controller.
async fn run_agent_facing(
    backend: Arc<dyn AgentBackend>,
    updates: broadcast::Sender<SessionNotification>,
    prompt_rx: mpsc::UnboundedReceiver<String>,
    session_tx: watch::Sender<Option<SessionId>>,
    shared: Arc<Shared>,
    target: ResumeTarget,
) -> Result<()> {
    // In-memory duplex: one end feeds goose's ACP server, the other is our client.
    let (client_side, server_side) = tokio::io::duplex(64 * 1024);
    let (server_recv, server_send) = tokio::io::split(server_side);
    let backend_task = tokio::spawn(async move {
        let recv = Box::new(server_recv.compat()) as Box<dyn AsyncRead + Send + Unpin>;
        let send = Box::new(server_send.compat_write()) as Box<dyn AsyncWrite + Send + Unpin>;
        backend.serve(recv, send).await
    });

    let (client_recv, client_send) = tokio::io::split(client_side);
    let transport =
        agent_client_protocol::ByteStreams::new(client_send.compat_write(), client_recv.compat());

    let notif_tx = updates.clone();
    let perm_shared = shared.clone();
    let prompt_rx = Arc::new(Mutex::new(prompt_rx));

    Client
        .builder()
        .name("goose-roam-broker")
        .on_receive_notification(
            async move |notification: SessionNotification, _cx| {
                let _ = notif_tx.send(notification);
                Ok(())
            },
            agent_client_protocol::on_receive_notification!(),
        )
        .on_receive_request(
            async move |request: RequestPermissionRequest, responder, _cx| {
                let outcome = relay_permission_to_controller(&perm_shared, request).await;
                responder.respond(RequestPermissionResponse::new(outcome))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_with(transport, async move |cx: ConnectionTo<SacpAgent>| {
            cx.send_request(InitializeRequest::new(ProtocolVersion::LATEST))
                .block_task()
                .await?;

            // `build_session` (session/new) and `session/load` both yield a
            // drivable `ActiveSession`; the ACP client has a builder for the
            // former only, so a resume sends the request directly and attaches
            // to the returned session id. Either way the top-level notification
            // handler above forwards `session/update`s to the peer broadcast.
            let mut session = match target {
                ResumeTarget::New { cwd } => {
                    cx.build_session(cwd).block_task().start_session().await?
                }
                ResumeTarget::Existing { session_id, cwd } => {
                    let session_id = SessionId::from(session_id);
                    cx.send_request(LoadSessionRequest::new(session_id.clone(), cwd))
                        .block_task()
                        .await?;
                    cx.attach_session(NewSessionResponse::new(session_id), Vec::new())?
                }
            };

            let _ = session_tx.send(Some(session.session_id().clone()));
            let mut rx = prompt_rx.lock().await;
            while let Some(prompt) = rx.recv().await {
                session.send_prompt(&prompt)?;
                let _ = session.read_to_string().await?;
            }
            Ok(())
        })
        .await?;

    backend_task.abort();
    Ok(())
}

/// Route a permission request to the current controller and await its decision.
/// Falls back to `Cancelled` if there is no controller (a safe default).
async fn relay_permission_to_controller(
    shared: &Shared,
    request: RequestPermissionRequest,
) -> RequestPermissionOutcome {
    let route = shared.router.lock().await.route_permission_request();
    let controller = match route {
        Route::To(id) => id,
        Route::Broadcast | Route::Drop => return RequestPermissionOutcome::Cancelled,
    };
    let sender = shared.perm_senders.lock().await.get(&controller).cloned();
    let Some(sender) = sender else {
        return RequestPermissionOutcome::Cancelled;
    };
    let (reply_tx, reply_rx) = oneshot::channel();
    if sender.send((request, reply_tx)).is_err() {
        return RequestPermissionOutcome::Cancelled;
    }
    match reply_rx.await {
        Ok(response) => response.outcome,
        Err(_) => RequestPermissionOutcome::Cancelled,
    }
}

/// Peer-facing half: an ACP agent facade for one roaming peer. Answers
/// `initialize`/`session/*`, attaches the peer via the broker, subscribes to the
/// shared update stream and fans each notification out, funnels the peer's
/// prompts to the agent-facing half (role-gated), and — if this peer is the
/// controller — delivers permission requests to it and relays the reply.
async fn run_peer_facing(
    shared: Arc<Shared>,
    scope: Scope,
    recv: Box<dyn AsyncRead + Send + Unpin>,
    send: Box<dyn AsyncWrite + Send + Unpin>,
) -> Result<()> {
    let role = Role::from_scope(scope);
    let id = shared.alloc_id().await;
    shared.router.lock().await.attach(id, role);

    let (perm_tx, perm_rx) = mpsc::unbounded_channel::<PermRelay>();
    if matches!(role, Role::Controller) {
        shared.perm_senders.lock().await.insert(id, perm_tx);
    }

    let transport = agent_client_protocol::ByteStreams::new(send, recv);

    let init_shared = shared.clone();
    let new_session_shared = shared.clone();
    let prompt_shared = shared.clone();
    let conn_shared = shared.clone();

    let result = SacpAgent
        .builder()
        .name("goose-roam-facade")
        .on_receive_request(
            async move |_request: InitializeRequest, responder, _cx| {
                let _ = &init_shared;
                responder.respond(InitializeResponse::new(ProtocolVersion::LATEST))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |_request: NewSessionRequest, responder, _cx| {
                let session_id = wait_for_session_id(&new_session_shared).await;
                responder.respond(NewSessionResponse::new(session_id))
            },
            agent_client_protocol::on_receive_request!(),
        )
        .on_receive_request(
            async move |request: PromptRequest, responder, _cx| {
                let allowed = prompt_shared.router.lock().await.accept_steer(id);
                match allowed {
                    Ok(()) => {
                        let text = prompt_text(&request);
                        let _ = prompt_shared.prompts.send(text);
                        responder.respond(PromptResponse::new(StopReason::EndTurn))
                    }
                    Err(Refused::NotPermitted) | Err(Refused::Unknown) => {
                        responder.respond(PromptResponse::new(StopReason::Refusal))
                    }
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
        .connect_with(transport, async move |cx: ConnectionTo<Client>| {
            let shared = conn_shared;
            let mut rx = shared.updates.subscribe();
            let mut perm_rx = perm_rx;
            loop {
                tokio::select! {
                    update = rx.recv() => match update {
                        Ok(notification) => {
                            if cx.send_notification(notification).is_err() {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    },
                    perm = perm_rx.recv() => {
                        if let Some((request, reply_tx)) = perm {
                            match cx.send_request(request).block_task().await {
                                Ok(response) => {
                                    let _ = reply_tx.send(response);
                                }
                                Err(_) => break,
                            }
                        }
                    },
                }
            }
            Ok(())
        })
        .await;

    shared.perm_senders.lock().await.remove(&id);
    shared.router.lock().await.detach(id);
    result.map_err(|e| anyhow!(e))
}

async fn wait_for_session_id(shared: &Shared) -> SessionId {
    let mut rx = shared.session_id.clone();
    loop {
        if let Some(id) = rx.borrow().clone() {
            return id;
        }
        if rx.changed().await.is_err() {
            // Sender dropped; fall back to a placeholder so the peer still gets
            // a well-formed response instead of a hung request.
            return SessionId::from("roam-shared-session");
        }
    }
}

fn prompt_text(request: &PromptRequest) -> String {
    use agent_client_protocol::schema::v1::ContentBlock;
    request
        .prompt
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text(text) => Some(text.text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;
    use std::time::Duration;

    use agent_client_protocol::schema::v1::{ContentBlock, ContentChunk, SessionUpdate};

    const MARKER: &str = "hello-peers";

    /// A minimal ACP *agent* that stands in for a live goose agent. It answers
    /// `initialize`/`session/new` and, once connected, streams an
    /// `AgentMessageChunk` notification carrying [`MARKER`] on a short interval
    /// so any peer that attaches to the shared session eventually observes it.
    /// It stays alive until the byte stream closes so delivery is never torn
    /// down prematurely.
    struct StubBackend;

    impl AgentBackend for StubBackend {
        fn serve(
            &self,
            recv: Box<dyn AsyncRead + Send + Unpin>,
            send: Box<dyn AsyncWrite + Send + Unpin>,
        ) -> BoxFuture<'static, Result<()>> {
            Box::pin(async move {
                let transport = agent_client_protocol::ByteStreams::new(send, recv);
                SacpAgent
                    .builder()
                    .name("stub-agent")
                    .on_receive_request(
                        async move |_request: InitializeRequest, responder, _cx| {
                            responder.respond(InitializeResponse::new(ProtocolVersion::LATEST))
                        },
                        agent_client_protocol::on_receive_request!(),
                    )
                    .on_receive_request(
                        async move |_request: NewSessionRequest, responder, _cx| {
                            responder
                                .respond(NewSessionResponse::new(SessionId::from("stub-session")))
                        },
                        agent_client_protocol::on_receive_request!(),
                    )
                    .connect_with(transport, async move |cx: ConnectionTo<Client>| {
                        let mut ticker = tokio::time::interval(Duration::from_millis(100));
                        loop {
                            ticker.tick().await;
                            let notification = SessionNotification::new(
                                SessionId::from("stub-session"),
                                SessionUpdate::AgentMessageChunk(ContentChunk::new(
                                    ContentBlock::from(MARKER),
                                )),
                            );
                            if cx.send_notification(notification).is_err() {
                                break;
                            }
                        }
                        Ok(())
                    })
                    .await
                    .map_err(|e| anyhow!(e))?;
                Ok(())
            })
        }
    }

    /// Drive a real ACP [`Client`] over `recv`/`send`: initialize, open a
    /// session, and collect streamed agent text until it contains [`MARKER`],
    /// returning the accumulated text.
    async fn run_peer_client(
        recv: Box<dyn AsyncRead + Send + Unpin>,
        send: Box<dyn AsyncWrite + Send + Unpin>,
    ) -> Result<String> {
        let transport = agent_client_protocol::ByteStreams::new(send, recv);
        let collected = Arc::new(StdMutex::new(String::new()));
        let sink = collected.clone();
        let wait_sink = collected.clone();

        Client
            .builder()
            .name("test-peer")
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
            .connect_with(transport, async move |cx: ConnectionTo<SacpAgent>| {
                cx.send_request(InitializeRequest::new(ProtocolVersion::LATEST))
                    .block_task()
                    .await?;
                cx.build_session(PathBuf::from("/"))
                    .block_task()
                    .run_until(async |_session| loop {
                        if wait_sink.lock().unwrap().contains(MARKER) {
                            return Ok(());
                        }
                        tokio::time::sleep(Duration::from_millis(25)).await;
                    })
                    .await
            })
            .await
            .map_err(|e| anyhow!(e))?;

        let text = collected.lock().unwrap().clone();
        Ok(text)
    }

    /// Two independent ACP peers attach to a single shared session via
    /// [`SharedSessionBridge`] and BOTH observe the same `session/update`
    /// broadcast from the one underlying agent, proving the fan-out shares one
    /// live session rather than spawning a fresh agent per peer.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn two_peers_share_one_session() {
        let assertion = async {
            let bridge = SharedSessionBridge::start(
                Arc::new(StubBackend),
                "test",
                ResumeTarget::New {
                    cwd: PathBuf::from("/"),
                },
            );

            let mut peer_tasks = Vec::new();
            for scope in [Scope::Control, Scope::Observe] {
                let (bridge_side, client_side) = tokio::io::duplex(64 * 1024);
                let (bridge_recv, bridge_send) = tokio::io::split(bridge_side);
                let (client_recv, client_send) = tokio::io::split(client_side);

                let endpoint = goose_roaming::RoamingIdentity::generate().public_key();
                let serve = bridge.serve_stream(
                    endpoint,
                    scope,
                    Box::new(bridge_recv.compat()),
                    Box::new(bridge_send.compat_write()),
                );
                tokio::spawn(serve);

                peer_tasks.push(tokio::spawn(run_peer_client(
                    Box::new(client_recv.compat()),
                    Box::new(client_send.compat_write()),
                )));
            }

            let mut results = Vec::new();
            for task in peer_tasks {
                results.push(task.await.expect("peer task panicked"));
            }
            results
        };

        let results = tokio::time::timeout(Duration::from_secs(15), assertion)
            .await
            .expect("timed out waiting for peers to observe the shared session update");

        for (i, result) in results.iter().enumerate() {
            let text = result
                .as_ref()
                .unwrap_or_else(|e| panic!("peer {i} client failed: {e:?}"));
            assert!(
                text.contains(MARKER),
                "peer {i} did not observe the shared session update; got: {text:?}"
            );
        }
    }
}
