use anyhow::{anyhow, Result};
use std::path::PathBuf;
use tokio::sync::mpsc;

use super::types::SseEvent;

/// Parse complete SSE events from the buffer and send them through the channel.
pub(crate) async fn process_sse_buffer(buffer: &mut String, tx: &mpsc::Sender<Result<SseEvent>>) {
    while let Some(boundary) = buffer.find("\n\n") {
        let (event_part, rest) = buffer.split_at(boundary);
        let event_block = event_part.to_string();
        *buffer = rest.get(2..).unwrap_or("").to_string();
        for line in event_block.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                let data = data.trim();
                if data.is_empty() {
                    continue;
                }
                match serde_json::from_str::<SseEvent>(data) {
                    Ok(event) => {
                        if tx.send(Ok(event)).await.is_err() {
                            return;
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse SSE event: {} - data: {}", e, data);
                    }
                }
            }
        }
    }
}

pub(crate) async fn find_available_port() -> Result<u16> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

pub(crate) fn generate_secret() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let random: u64 = rng.gen();
    format!("cli-{:016x}", random)
}

pub(crate) fn find_goosed_binary() -> Result<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("goosed");
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }

    if let Ok(path) = which::which("goosed") {
        return Ok(path);
    }

    Err(anyhow!(
        "Could not find goosed binary. Ensure it is built or on PATH."
    ))
}
