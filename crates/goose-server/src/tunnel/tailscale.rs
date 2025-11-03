use super::TunnelInfo;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::fs;
use tokio::process::Command;
use tracing::{error, info};

const SCRIPT_NAME: &str = "tailscale-tunnel.sh";
const TIMEOUT_SECS: u64 = 120;
const POLL_INTERVAL_MS: u64 = 500;

pub async fn start(port: u16, secret: String) -> Result<TunnelInfo> {
    // Find the script path
    let script_path = find_script_path().await?;

    info!(
        "Starting Tailscale tunnel with script: {}",
        script_path.display()
    );

    // Create temp output file
    let output_path = std::env::temp_dir().join(format!(
        "goose-tunnel-{}.json",
        chrono::Utc::now().timestamp()
    ));

    // Execute the script
    let _child = Command::new(&script_path)
        .arg(port.to_string())
        .arg(&secret)
        .arg(&output_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to start tailscale tunnel script")?;

    // Wait for output file
    let tunnel_info = wait_for_output_file(&output_path).await?;

    info!("Tailscale tunnel started successfully");

    Ok(tunnel_info)
}

pub async fn stop() {
    info!("Stopping Tailscale tunnel");
    // The tailscale tunnel script manages its own processes
    // We would need to track PIDs or use a different mechanism to stop it
    // For now, this is a placeholder
}

async fn find_script_path() -> Result<PathBuf> {
    // Check common locations
    let candidates = vec![
        PathBuf::from(format!("/usr/local/bin/{}", SCRIPT_NAME)),
        PathBuf::from(format!("/opt/goose/bin/{}", SCRIPT_NAME)),
        std::env::current_exe()?
            .parent()
            .unwrap()
            .join("bin")
            .join(SCRIPT_NAME),
    ];

    for path in candidates {
        if path.exists() {
            return Ok(path);
        }
    }

    anyhow::bail!("Tailscale tunnel script not found")
}

async fn wait_for_output_file(path: &PathBuf) -> Result<TunnelInfo> {
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(TIMEOUT_SECS);

    loop {
        if start.elapsed() > timeout {
            anyhow::bail!("Timeout waiting for tunnel to start");
        }

        if path.exists() {
            match fs::read_to_string(path).await {
                Ok(contents) if !contents.trim().is_empty() => {
                    match serde_json::from_str::<TunnelInfo>(&contents) {
                        Ok(info) => {
                            // Cleanup temp file
                            let _ = fs::remove_file(path).await;
                            return Ok(info);
                        }
                        Err(e) => {
                            // File might still be being written
                            if !e.is_eof() {
                                error!("Error parsing tunnel output: {}", e);
                            }
                        }
                    }
                }
                Ok(_) => {
                    // File is empty, keep waiting
                }
                Err(e) => {
                    error!("Error reading tunnel output: {}", e);
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;
    }
}
