use super::{TunnelInfo, TunnelPids};
use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use reqwest;
use serde::{Deserialize, Serialize};
use socket2::{Socket, TcpKeepalive};
use std::collections::HashMap;
#[cfg(unix)]
use std::os::unix::io::{AsRawFd, FromRawFd};
#[cfg(windows)]
use std::os::windows::io::{AsRawSocket, FromRawSocket};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{error, info, warn};
use url::Url;

const WORKER_URL: &str = "https://cloudflare-tunnel-proxy.michael-neale.workers.dev";
const IDLE_TIMEOUT_SECS: u64 = 600; // 10 minutes
const RECONNECT_DELAY_MS: u64 = 100;

type WebSocketSender = Arc<
    RwLock<
        Option<
            futures::stream::SplitSink<
                tokio_tungstenite::WebSocketStream<
                    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
                >,
                Message,
            >,
        >,
    >,
>;

#[derive(Debug, Serialize, Deserialize)]
struct TunnelMessage {
    #[serde(rename = "requestId")]
    request_id: String,
    method: String,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<String>,
}

#[derive(Debug, Serialize)]
struct TunnelResponse {
    #[serde(rename = "requestId")]
    request_id: String,
    status: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "chunkIndex")]
    chunk_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "totalChunks")]
    total_chunks: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "isChunked")]
    is_chunked: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "isStreaming")]
    is_streaming: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "isFirstChunk")]
    is_first_chunk: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "isLastChunk")]
    is_last_chunk: Option<bool>,
}

fn build_request(
    client: &reqwest::Client,
    url: &str,
    message: &TunnelMessage,
    server_secret: &str,
) -> reqwest::RequestBuilder {
    let mut request_builder = match message.method.as_str() {
        "GET" => client.get(url),
        "POST" => client.post(url),
        "PUT" => client.put(url),
        "DELETE" => client.delete(url),
        "PATCH" => client.patch(url),
        _ => client.get(url),
    };

    if let Some(headers) = &message.headers {
        for (key, value) in headers {
            if key.to_lowercase() == "x-secret-key" {
                continue;
            }
            request_builder = request_builder.header(key, value);
        }
    }

    request_builder = request_builder.header("X-Secret-Key", server_secret);

    if let Some(body) = &message.body {
        if message.method != "GET" && message.method != "HEAD" {
            request_builder = request_builder.body(body.clone());
        }
    }

    request_builder
}

async fn handle_streaming_response(
    response: reqwest::Response,
    status: u16,
    headers_map: HashMap<String, String>,
    request_id: String,
    message_path: String,
    ws_tx: WebSocketSender,
) -> Result<()> {
    info!("← {} {} [{}] (streaming)", status, message_path, request_id);

    let mut stream = response.bytes_stream();
    let mut chunk_index = 0;
    let mut is_first_chunk = true;

    while let Some(chunk_result) = stream.next().await {
        match chunk_result {
            Ok(chunk) => {
                let chunk_str = String::from_utf8_lossy(&chunk).to_string();
                let tunnel_response = TunnelResponse {
                    request_id: request_id.clone(),
                    status,
                    headers: if is_first_chunk {
                        Some(headers_map.clone())
                    } else {
                        None
                    },
                    body: Some(chunk_str),
                    error: None,
                    chunk_index: Some(chunk_index),
                    total_chunks: None,
                    is_chunked: None,
                    is_streaming: Some(true),
                    is_first_chunk: Some(is_first_chunk),
                    is_last_chunk: Some(false),
                };
                send_response(ws_tx.clone(), tunnel_response).await?;
                chunk_index += 1;
                is_first_chunk = false;
            }
            Err(e) => {
                error!("Error reading stream chunk: {}", e);
                break;
            }
        }
    }

    let tunnel_response = TunnelResponse {
        request_id: request_id.clone(),
        status,
        headers: None,
        body: Some(String::new()),
        error: None,
        chunk_index: Some(chunk_index),
        total_chunks: None,
        is_chunked: None,
        is_streaming: Some(true),
        is_first_chunk: Some(false),
        is_last_chunk: Some(true),
    };
    send_response(ws_tx, tunnel_response).await?;
    info!(
        "← {} {} [{}] (complete, {} chunks)",
        status, message_path, request_id, chunk_index
    );
    Ok(())
}

async fn handle_chunked_response(
    body: String,
    status: u16,
    headers_map: HashMap<String, String>,
    request_id: String,
    message_path: String,
    ws_tx: WebSocketSender,
) -> Result<()> {
    const MAX_WS_SIZE: usize = 900_000;
    let total_chunks = body.len().div_ceil(MAX_WS_SIZE);
    info!(
        "← {} {} [{}] ({} bytes, {} chunks)",
        status,
        message_path,
        request_id,
        body.len(),
        total_chunks
    );

    for (i, chunk) in body.as_bytes().chunks(MAX_WS_SIZE).enumerate() {
        let chunk_str = String::from_utf8_lossy(chunk).to_string();
        let tunnel_response = TunnelResponse {
            request_id: request_id.clone(),
            status,
            headers: if i == 0 {
                Some(headers_map.clone())
            } else {
                None
            },
            body: Some(chunk_str),
            error: None,
            chunk_index: Some(i),
            total_chunks: Some(total_chunks),
            is_chunked: Some(true),
            is_streaming: None,
            is_first_chunk: None,
            is_last_chunk: None,
        };
        send_response(ws_tx.clone(), tunnel_response).await?;
    }
    Ok(())
}

async fn handle_request(
    message: TunnelMessage,
    port: u16,
    ws_tx: WebSocketSender,
    server_secret: String,
) -> Result<()> {
    let request_id = message.request_id.clone();
    info!("→ {} {} [{}]", message.method, message.path, request_id);

    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}{}", port, message.path);
    let request_builder = build_request(&client, &url, &message, &server_secret);

    let response = match request_builder.send().await {
        Ok(resp) => resp,
        Err(e) => {
            error!("✗ Request error [{}]: {}", request_id, e);
            let error_response = TunnelResponse {
                request_id,
                status: 500,
                headers: None,
                body: None,
                error: Some(e.to_string()),
                chunk_index: None,
                total_chunks: None,
                is_chunked: None,
                is_streaming: None,
                is_first_chunk: None,
                is_last_chunk: None,
            };
            send_response(ws_tx, error_response).await?;
            return Ok(());
        }
    };

    let status = response.status().as_u16();
    let headers_map: HashMap<String, String> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let is_streaming = headers_map
        .get("content-type")
        .map(|ct| ct.contains("text/event-stream"))
        .unwrap_or(false);

    if is_streaming {
        handle_streaming_response(
            response,
            status,
            headers_map,
            request_id,
            message.path,
            ws_tx,
        )
        .await?;
    } else {
        let body = response.text().await.unwrap_or_default();
        const MAX_WS_SIZE: usize = 900_000;

        if body.len() > MAX_WS_SIZE {
            handle_chunked_response(body, status, headers_map, request_id, message.path, ws_tx)
                .await?;
        } else {
            let tunnel_response = TunnelResponse {
                request_id: request_id.clone(),
                status,
                headers: Some(headers_map),
                body: Some(body),
                error: None,
                chunk_index: None,
                total_chunks: None,
                is_chunked: None,
                is_streaming: None,
                is_first_chunk: None,
                is_last_chunk: None,
            };
            send_response(ws_tx, tunnel_response).await?;
            info!("← {} {} [{}]", status, message.path, request_id);
        }
    }

    Ok(())
}

async fn send_response(ws_tx: WebSocketSender, response: TunnelResponse) -> Result<()> {
    let json = serde_json::to_string(&response)?;
    if let Some(tx) = ws_tx.write().await.as_mut() {
        tx.send(Message::Text(json.into()))
            .await
            .context("Failed to send response")?;
    }
    Ok(())
}

fn configure_tcp_keepalive(
    stream: &tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) {
    let tcp_stream = stream.get_ref().get_ref();

    #[cfg(unix)]
    let socket: Socket = {
        let fd = tcp_stream.as_raw_fd();
        unsafe { Socket::from_raw_fd(fd) }
    };

    #[cfg(windows)]
    let socket: Socket = {
        let sock = tcp_stream.as_raw_socket();
        unsafe { Socket::from_raw_socket(sock) }
    };

    let keepalive = TcpKeepalive::new()
        .with_time(Duration::from_secs(30))
        .with_interval(Duration::from_secs(30));

    if let Err(e) = socket.set_tcp_keepalive(&keepalive) {
        warn!("Failed to set TCP keep-alive: {}", e);
    } else {
        info!("✓ TCP keep-alive enabled (30s interval)");
    }

    std::mem::forget(socket);
}

async fn handle_websocket_messages(
    mut read: futures::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    ws_tx: WebSocketSender,
    port: u16,
    server_secret: String,
    last_activity: Arc<RwLock<Instant>>,
    active_tasks: Arc<RwLock<Vec<JoinHandle<()>>>>,
) {
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Text(text)) => {
                *last_activity.write().await = Instant::now();

                match serde_json::from_str::<TunnelMessage>(&text) {
                    Ok(tunnel_msg) => {
                        let ws_tx_clone = ws_tx.clone();
                        let server_secret_clone = server_secret.clone();
                        let task = tokio::spawn(async move {
                            if let Err(e) =
                                handle_request(tunnel_msg, port, ws_tx_clone, server_secret_clone)
                                    .await
                            {
                                error!("Error handling request: {}", e);
                            }
                        });
                        active_tasks.write().await.push(task);
                    }
                    Err(e) => {
                        error!("Error parsing tunnel message: {}", e);
                    }
                }
            }
            Ok(Message::Close(_)) => {
                info!("✗ Connection closed by server");
                break;
            }
            Ok(Message::Ping(_)) | Ok(Message::Pong(_)) => {
                *last_activity.write().await = Instant::now();
            }
            Err(e) => {
                error!("✗ WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }
}

async fn cleanup_connection(
    ws_tx: WebSocketSender,
    active_tasks: Arc<RwLock<Vec<JoinHandle<()>>>>,
) {
    if let Some(mut tx) = ws_tx.write().await.take() {
        let _ = tx.close().await;
    }

    let tasks = active_tasks.write().await.drain(..).collect::<Vec<_>>();
    info!("Aborting {} active request tasks", tasks.len());
    for task in tasks {
        task.abort();
    }
}

async fn run_tunnel_loop(
    port: u16,
    agent_id: String,
    server_secret: String,
    handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
) {
    let worker_url =
        std::env::var("GOOSE_TUNNEL_WORKER_URL").unwrap_or_else(|_| WORKER_URL.to_string());
    let ws_url = worker_url
        .replace("https://", "wss://")
        .replace("http://", "ws://");

    let url = format!("{}/connect?agent_id={}", ws_url, agent_id);

    loop {
        info!("Connecting to {}...", url);

        let ws_stream = match connect_async(url.clone()).await {
            Ok((stream, _)) => {
                configure_tcp_keepalive(&stream);
                stream
            }
            Err(e) => {
                error!("✗ WebSocket connection error: {}", e);
                tokio::time::sleep(Duration::from_millis(RECONNECT_DELAY_MS)).await;
                continue;
            }
        };

        info!("✓ Connected as agent: {}", agent_id);
        info!("✓ Proxying to: http://127.0.0.1:{}", port);
        let public_url = format!("{}/tunnel/{}", worker_url, agent_id);
        info!("✓ Public URL: {}", public_url);

        let (write, read) = ws_stream.split();
        let ws_tx: WebSocketSender = Arc::new(RwLock::new(Some(write)));
        let last_activity = Arc::new(RwLock::new(Instant::now()));
        let active_tasks: Arc<RwLock<Vec<JoinHandle<()>>>> = Arc::new(RwLock::new(Vec::new()));

        let last_activity_clone = last_activity.clone();
        let idle_task = async move {
            loop {
                tokio::time::sleep(Duration::from_secs(60)).await;
                let elapsed = last_activity_clone.read().await.elapsed();
                if elapsed > Duration::from_secs(IDLE_TIMEOUT_SECS) {
                    warn!(
                        "No activity for {} minutes, forcing reconnect",
                        IDLE_TIMEOUT_SECS / 60
                    );
                    break;
                }
            }
        };

        tokio::select! {
            _ = idle_task => {
                info!("✗ Idle timeout triggered, reconnecting...");
            }
            _ = handle_websocket_messages(
                read,
                ws_tx.clone(),
                port,
                server_secret.clone(),
                last_activity,
                active_tasks.clone()
            ) => {
                info!("✗ Connection ended");
            }
        }

        cleanup_connection(ws_tx, active_tasks).await;

        if handle.read().await.is_none() {
            info!("Tunnel stopped, not reconnecting");
            break;
        }

        info!("✗ Connection lost, reconnecting...");
        tokio::time::sleep(Duration::from_millis(RECONNECT_DELAY_MS)).await;
    }
}

pub async fn start(
    port: u16,
    tunnel_secret: String,
    server_secret: String,
    agent_id: String,
    handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>,
) -> Result<TunnelInfo> {
    let worker_url =
        std::env::var("GOOSE_TUNNEL_WORKER_URL").unwrap_or_else(|_| WORKER_URL.to_string());

    if worker_url.to_lowercase() == "none" || worker_url.to_lowercase() == "no" {
        anyhow::bail!("Tunnel is disabled via GOOSE_TUNNEL_WORKER_URL environment variable");
    }

    let agent_id_clone = agent_id.clone();
    let server_secret_clone = server_secret;
    let handle_clone = handle.clone();

    let task = tokio::spawn(async move {
        run_tunnel_loop(port, agent_id_clone, server_secret_clone, handle_clone).await;
    });

    *handle.write().await = Some(task);

    let public_url = format!("{}/tunnel/{}", worker_url, agent_id);
    let hostname = Url::parse(&worker_url)?
        .host_str()
        .unwrap_or("")
        .to_string();

    Ok(TunnelInfo {
        url: public_url,
        ipv4: String::new(),
        ipv6: String::new(),
        hostname,
        secret: tunnel_secret,
        port,
        pids: TunnelPids::default(),
    })
}

pub async fn stop(handle: Arc<RwLock<Option<tokio::task::JoinHandle<()>>>>) {
    if let Some(task) = handle.write().await.take() {
        task.abort();
        info!("Lapstone tunnel stopped");
    }
}
