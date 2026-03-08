//! Goosed process discovery and lifecycle management.
//!
//! Manages a persistent goosed process across CLI invocations by storing
//! connection state in `~/.config/goose/goosed.state`. On first run, spawns
//! goosed and records port/secret/PID. Subsequent runs reuse the running instance.
//!
//! On Linux, an optional systemd user service can be installed for auto-restart.
//! On macOS, a launchd plist serves the same purpose.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Persisted state for a running goosed instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoosedState {
    pub port: u16,
    pub secret_key: String,
    pub pid: u32,
    pub started_at: String,
}

impl GoosedState {
    fn state_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| anyhow!("Could not determine config directory"))?
            .join("goose");
        std::fs::create_dir_all(&config_dir)?;
        Ok(config_dir.join("goosed.state"))
    }

    /// Load persisted state from disk.
    pub fn load() -> Result<Option<Self>> {
        let path = Self::state_path()?;
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)?;
        let state: Self = serde_json::from_str(&content)?;
        Ok(Some(state))
    }

    /// Save state to disk.
    pub fn save(&self) -> Result<()> {
        let path = Self::state_path()?;
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        debug!(port = self.port, pid = self.pid, "Saved goosed state");
        Ok(())
    }

    /// Remove state file.
    pub fn remove() -> Result<()> {
        let path = Self::state_path()?;
        if path.exists() {
            std::fs::remove_file(&path)?;
            debug!("Removed goosed state file");
        }
        Ok(())
    }

    /// Check if the process identified by PID is still running.
    pub fn is_alive(&self) -> bool {
        is_process_alive(self.pid)
    }
}

/// Check if a process with the given PID is still running.
fn is_process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        // kill(pid, 0) checks process existence without sending a signal
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        // On non-Unix, assume alive (will fail on health check)
        let _ = pid;
        true
    }
}

/// Attempt to discover and connect to an existing goosed instance.
/// Returns (base_url, secret_key) if a live instance is found.
pub async fn discover_goosed() -> Result<Option<(String, String)>> {
    let state = match GoosedState::load()? {
        Some(s) => s,
        None => {
            debug!("No goosed state file found");
            return Ok(None);
        }
    };

    // Check PID is alive
    if !state.is_alive() {
        warn!(pid = state.pid, "Goosed process is dead, cleaning up state");
        GoosedState::remove()?;
        return Ok(None);
    }

    // Health check the HTTP endpoint
    let base_url = format!("http://127.0.0.1:{}", state.port);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()?;

    match client
        .get(format!("{}/status", base_url))
        .header("X-Secret-Key", &state.secret_key)
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            info!(
                port = state.port,
                pid = state.pid,
                "Discovered running goosed instance"
            );
            Ok(Some((base_url, state.secret_key)))
        }
        Ok(resp) => {
            warn!(
                status = %resp.status(),
                "Goosed health check returned non-success"
            );
            GoosedState::remove()?;
            Ok(None)
        }
        Err(e) => {
            warn!(error = %e, "Goosed health check failed");
            GoosedState::remove()?;
            Ok(None)
        }
    }
}

/// Record a newly spawned goosed instance for future discovery.
pub fn record_goosed(port: u16, secret_key: &str, pid: u32) -> Result<()> {
    let state = GoosedState {
        port,
        secret_key: secret_key.to_string(),
        pid,
        started_at: chrono::Utc::now().to_rfc3339(),
    };
    state.save()?;
    info!(port, pid, "Recorded goosed instance for discovery");
    Ok(())
}

/// Generate a systemd user service unit file for goosed.
pub fn generate_systemd_unit(goosed_path: &str) -> String {
    format!(
        r#"[Unit]
Description=Goose Agent Server (goosed)
After=network.target

[Service]
Type=simple
ExecStart={goosed_path} agent
Restart=on-failure
RestartSec=3
Environment=GOOSE_PORT=0
Environment=GOOSE_SERVER__SECRET_KEY=%h/.config/goose/goosed.secret

[Install]
WantedBy=default.target
"#
    )
}

/// Generate a macOS launchd plist for goosed.
pub fn generate_launchd_plist(goosed_path: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>dev.block.goosed</string>
    <key>ProgramArguments</key>
    <array>
        <string>{goosed_path}</string>
        <string>agent</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>Crashed</key>
        <true/>
    </dict>
    <key>StandardOutPath</key>
    <string>/tmp/goosed.stdout.log</string>
    <key>StandardErrorPath</key>
    <string>/tmp/goosed.stderr.log</string>
</dict>
</plist>
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_roundtrip() {
        let state = GoosedState {
            port: 12345,
            secret_key: "test-secret".to_string(),
            pid: 9999,
            started_at: "2025-02-15T12:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&state).unwrap();
        let loaded: GoosedState = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.port, 12345);
        assert_eq!(loaded.secret_key, "test-secret");
        assert_eq!(loaded.pid, 9999);
    }

    #[test]
    fn test_dead_pid_not_alive() {
        // PID 0 is kernel — we can't signal it as non-root
        // PID 4_000_000 is almost certainly not running
        assert!(!is_process_alive(4_000_000));
    }

    #[test]
    fn test_current_pid_is_alive() {
        let pid = std::process::id();
        assert!(is_process_alive(pid));
    }

    #[test]
    fn test_generate_systemd_unit() {
        let unit = generate_systemd_unit("/usr/local/bin/goosed");
        assert!(unit.contains("ExecStart=/usr/local/bin/goosed agent"));
        assert!(unit.contains("Restart=on-failure"));
        assert!(unit.contains("[Install]"));
    }

    #[test]
    fn test_generate_launchd_plist() {
        let plist = generate_launchd_plist("/usr/local/bin/goosed");
        assert!(plist.contains("dev.block.goosed"));
        assert!(plist.contains("<string>/usr/local/bin/goosed</string>"));
        assert!(plist.contains("KeepAlive"));
    }
}
