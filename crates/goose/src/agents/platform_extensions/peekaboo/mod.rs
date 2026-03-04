//! Peekaboo helper functions for macOS GUI automation via the Peekaboo CLI.
//!
//! These are used by `ComputerControllerServer` on macOS to auto-install
//! and invoke peekaboo. This module does not expose its own MCP server —
//! peekaboo is accessed through the `computer_control` tool on macOS.

const BREW_FORMULA: &str = "steipete/tap/peekaboo";

pub fn is_peekaboo_installed() -> bool {
    std::process::Command::new("which")
        .arg("peekaboo")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn resolve_brew() -> Option<String> {
    if let Ok(output) = std::process::Command::new("which").arg("brew").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(path);
            }
        }
    }

    for candidate in &["/opt/homebrew/bin/brew", "/usr/local/bin/brew"] {
        if std::path::Path::new(candidate).exists() {
            return Some(candidate.to_string());
        }
    }

    None
}

pub fn auto_install_peekaboo() -> Result<(), String> {
    let brew = resolve_brew().ok_or_else(|| {
        "Homebrew is not installed. Install Homebrew first (https://brew.sh), then run: brew install steipete/tap/peekaboo".to_string()
    })?;

    tracing::info!("Running: {} install {}", brew, BREW_FORMULA);

    let output = std::process::Command::new(&brew)
        .args(["install", BREW_FORMULA])
        .output()
        .map_err(|e| format!("Failed to run brew: {}", e))?;

    if output.status.success() {
        if is_peekaboo_installed() {
            return Ok(());
        }
        // brew succeeded but binary not on PATH — try adding brew bin
        if let Ok(prefix_output) = std::process::Command::new(&brew)
            .args(["--prefix"])
            .output()
        {
            let prefix = String::from_utf8_lossy(&prefix_output.stdout)
                .trim()
                .to_string();
            let bin_path = format!("{}/bin/peekaboo", prefix);
            if std::path::Path::new(&bin_path).exists() {
                if let Ok(current_path) = std::env::var("PATH") {
                    std::env::set_var("PATH", format!("{}/bin:{}", prefix, current_path));
                    if is_peekaboo_installed() {
                        return Ok(());
                    }
                }
            }
        }
        Err("brew install succeeded but peekaboo binary not found on PATH".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        Err(format!(
            "brew install failed (exit {}):\n{}{}",
            output.status,
            stderr.trim(),
            if stdout.trim().is_empty() {
                String::new()
            } else {
                format!("\n{}", stdout.trim())
            }
        ))
    }
}
