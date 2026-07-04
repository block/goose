//! Peekaboo helper functions for macOS GUI automation via the Peekaboo CLI.
//!
//! These are used by `ComputerControllerServer` on macOS to auto-install
//! and invoke peekaboo. This module does not expose its own MCP server —
//! peekaboo is accessed through the `computer_control` tool on macOS.

const BREW_FORMULA: &str = "steipete/tap/peekaboo";

pub fn is_peekaboo_installed() -> bool {
    // A GUI-launched app inherits a minimal PATH that excludes Homebrew, so a
    // bare `which peekaboo` reports "not installed" even when it is — which drove
    // a spurious `brew install` on the first computer_control call every
    // restricted-PATH session. Check the well-known Homebrew locations directly
    // (mirrors resolve_brew) before falling back to PATH lookup.
    for candidate in &["/opt/homebrew/bin/peekaboo", "/usr/local/bin/peekaboo"] {
        if std::path::Path::new(candidate).exists() {
            return true;
        }
    }

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
        // brew succeeded but the binary isn't in a well-known location — confirm
        // it exists under the brew prefix. run_peekaboo_cmd invokes peekaboo with
        // the login-shell PATH (merged_path), which includes the brew bin, so we
        // don't need to mutate this process's global PATH (which would be a data
        // race against other tokio worker threads reading the environment).
        if let Ok(prefix_output) = std::process::Command::new(&brew)
            .args(["--prefix"])
            .output()
        {
            let prefix = String::from_utf8_lossy(&prefix_output.stdout)
                .trim()
                .to_string();
            let bin_path = format!("{}/bin/peekaboo", prefix);
            if std::path::Path::new(&bin_path).exists() {
                return Ok(());
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
