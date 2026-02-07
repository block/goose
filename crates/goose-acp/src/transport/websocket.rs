use anyhow::Result;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use futures::{SinkExt, StreamExt};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use tracing::{debug, error, info, warn};

use super::{TransportSession, HEADER_SESSION_ID};
use crate::adapters::{ReceiverToAsyncRead, SenderToAsyncWrite};
use crate::server_factory::AcpServer;

pub(crate) struct WsState {
    server: Arc<AcpServer>,
    sessions: RwLock<HashMap<String, TransportSession>>,
}

impl WsState {
    pub fn new(server: Arc<AcpServer>) -> Self {
        Self {
            server,
            sessions: RwLock::new(HashMap::new()),
        }
    }

    async fn create_connection(&self) -> Result<String> {
        let (to_agent_tx, to_agent_rx) = mpsc::channel::<String>(256);
        let (from_agent_tx, from_agent_rx) = mpsc::channel::<String>(256);

        let agent = self.server.create_agent().await?;

        // Create a Goose ACP session (not just the transport connection)
        let session_id = agent.create_session().await?;

        let handle = tokio::spawn(async move {
            let read_stream = ReceiverToAsyncRead::new(to_agent_rx);
            let write_stream = SenderToAsyncWrite::new(from_agent_tx);

            if let Err(e) =
                crate::server::serve(agent, read_stream.compat(), write_stream.compat_write()).await
            {
                error!("ACP WebSocket session error: {}", e);
            }
        });

        self.sessions.write().await.insert(
            session_id.clone(),
            TransportSession {
                to_agent_tx,
                from_agent_rx: Arc::new(Mutex::new(from_agent_rx)),
                handle,
            },
        );

        info!(session_id = %session_id, "WebSocket connection created");
        Ok(session_id)
    }

    async fn remove_connection(&self, session_id: &str) {
        if let Some(session) = self.sessions.write().await.remove(session_id) {
            session.handle.abort();
            info!(session_id = %session_id, "WebSocket connection removed");
        }
    }
}

pub(crate) async fn handle_get(state: Arc<WsState>, ws: WebSocketUpgrade) -> Response {
    let session_id = match state.create_connection().await {
        Ok(id) => id,
        Err(e) => {
            error!("Failed to create WebSocket connection: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create WebSocket connection",
            )
                .into_response();
        }
    };

    let mut response = ws.on_upgrade({
        let session_id = session_id.clone();
        move |socket| handle_ws(socket, state, session_id)
    });
    response
        .headers_mut()
        .insert(HEADER_SESSION_ID, session_id.parse().unwrap());
    response
}

pub(crate) async fn handle_ws(socket: WebSocket, state: Arc<WsState>, session_id: String) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    let (to_agent, from_agent) = {
        let sessions = state.sessions.read().await;
        match sessions.get(&session_id) {
            Some(session) => (session.to_agent_tx.clone(), session.from_agent_rx.clone()),
            None => {
                error!(session_id = %session_id, "Session not found after creation");
                return;
            }
        }
    };

    debug!(session_id = %session_id, "Starting bidirectional message loop");

    let mut from_agent_rx = from_agent.lock().await;

    loop {
        tokio::select! {
            Some(msg_result) = ws_rx.next() => {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        let text_str = text.to_string();
                        debug!(session_id = %session_id, "Client → Agent: {} bytes", text_str.len());
                        if let Err(e) = to_agent.send(text_str).await {
                            error!(session_id = %session_id, "Failed to send to agent: {}", e);
                            break;
                        }
                    }
                    Ok(Message::Close(frame)) => {
                        debug!(session_id = %session_id, "Client closed connection: {:?}", frame);
                        break;
                    }
                    Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {
                        // Axum handles ping/pong automatically
                        continue;
                    }
                    Ok(Message::Binary(_)) => {
                        warn!(session_id = %session_id, "Ignoring binary message (ACP uses text)");
                        continue;
                    }
                    Err(e) => {
                        error!(session_id = %session_id, "WebSocket error: {}", e);
                        break;
                    }
                }
            }

            Some(text) = from_agent_rx.recv() => {
                debug!(session_id = %session_id, "Agent → Client: {} bytes", text.len());
                if let Err(e) = ws_tx.send(Message::Text(text.into())).await {
                    error!(session_id = %session_id, "Failed to send to client: {}", e);
                    break;
                }
            }

            else => {
                debug!(session_id = %session_id, "Both channels closed");
                break;
            }
        }
    }

    debug!(session_id = %session_id, "Cleaning up connection");
    state.remove_connection(&session_id).await;
}
