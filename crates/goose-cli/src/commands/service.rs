use anyhow::{bail, Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::goosed_client::discovery::{generate_launchd_plist, generate_systemd_unit};

fn detect_goosed_path() -> Result<String> {
    if let Ok(path) = which::which("goosed") {
        return Ok(path.to_string_lossy().into_owned());
    }
    let exe = std::env::current_exe().context("failed to resolve current executable")?;
    let dir = exe.parent().unwrap_or(exe.as_ref());
    let candidate = dir.join("goosed");
    if candidate.exists() {
        return Ok(candidate.to_string_lossy().into_owned());
    }
    bail!("could not find goosed binary; ensure it is on PATH or next to the goose binary")
}

fn is_macos() -> bool {
    cfg!(target_os = "macos")
}

fn is_linux() -> bool {
    cfg!(target_os = "linux")
}

fn systemd_unit_path() -> Result<PathBuf> {
    let config = dirs::config_dir().context("could not determine config directory")?;
    Ok(config.join("systemd/user/goosed.service"))
}

fn launchd_plist_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("could not determine home directory")?;
    Ok(home.join("Library/LaunchAgents/dev.block.goosed.plist"))
}

pub fn handle_service_install() -> Result<()> {
    let goosed_path = detect_goosed_path()?;
    println!("Using goosed binary: {goosed_path}");

    if is_linux() {
        let unit_path = systemd_unit_path()?;
        if let Some(parent) = unit_path.parent() {
            fs::create_dir_all(parent).context("failed to create systemd user unit directory")?;
        }

        let unit = generate_systemd_unit(&goosed_path);
        fs::write(&unit_path, unit).context("failed to write systemd unit file")?;
        println!("Created {}", unit_path.display());

        println!("Running systemctl --user daemon-reload...");
        let status = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status()
            .context("failed to run systemctl daemon-reload")?;
        if !status.success() {
            bail!("systemctl --user daemon-reload failed");
        }

        println!("Running systemctl --user enable goosed...");
        let status = Command::new("systemctl")
            .args(["--user", "enable", "goosed"])
            .status()
            .context("failed to run systemctl enable")?;
        if !status.success() {
            bail!("systemctl --user enable goosed failed");
        }

        println!("goosed installed as a systemd user service.");
        println!("Start it with: systemctl --user start goosed");
    } else if is_macos() {
        let plist_path = launchd_plist_path()?;
        if let Some(parent) = plist_path.parent() {
            fs::create_dir_all(parent).context("failed to create LaunchAgents directory")?;
        }

        let plist = generate_launchd_plist(&goosed_path);
        fs::write(&plist_path, plist).context("failed to write launchd plist")?;
        println!("Created {}", plist_path.display());

        println!("Loading launchd service...");
        let status = Command::new("launchctl")
            .args(["load", &plist_path.to_string_lossy()])
            .status()
            .context("failed to run launchctl load")?;
        if !status.success() {
            bail!("launchctl load failed");
        }

        println!("goosed installed as a launchd service.");
    } else {
        bail!("unsupported platform; only Linux (systemd) and macOS (launchd) are supported");
    }

    Ok(())
}

pub fn handle_service_uninstall() -> Result<()> {
    if is_linux() {
        println!("Disabling and stopping goosed systemd service...");
        let _ = Command::new("systemctl")
            .args(["--user", "disable", "goosed"])
            .status();
        let _ = Command::new("systemctl")
            .args(["--user", "stop", "goosed"])
            .status();

        let unit_path = systemd_unit_path()?;
        if unit_path.exists() {
            fs::remove_file(&unit_path).context("failed to remove systemd unit file")?;
            println!("Removed {}", unit_path.display());
        } else {
            println!("No unit file found at {}", unit_path.display());
        }

        let _ = Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status();

        println!("goosed systemd service uninstalled.");
    } else if is_macos() {
        let plist_path = launchd_plist_path()?;

        println!("Unloading launchd service...");
        let _ = Command::new("launchctl")
            .args(["unload", &plist_path.to_string_lossy()])
            .status();

        if plist_path.exists() {
            fs::remove_file(&plist_path).context("failed to remove launchd plist")?;
            println!("Removed {}", plist_path.display());
        } else {
            println!("No plist found at {}", plist_path.display());
        }

        println!("goosed launchd service uninstalled.");
    } else {
        bail!("unsupported platform; only Linux (systemd) and macOS (launchd) are supported");
    }

    Ok(())
}

pub fn handle_service_status() -> Result<()> {
    if is_linux() {
        let output = Command::new("systemctl")
            .args(["--user", "status", "goosed"])
            .output()
            .context("failed to run systemctl status")?;
        print!("{}", String::from_utf8_lossy(&output.stdout));
        if !output.stderr.is_empty() {
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
        }
    } else if is_macos() {
        let output = Command::new("launchctl")
            .args(["list", "dev.block.goosed"])
            .output()
            .context("failed to run launchctl list")?;
        print!("{}", String::from_utf8_lossy(&output.stdout));
        if !output.stderr.is_empty() {
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
        }
    } else {
        bail!("unsupported platform; only Linux (systemd) and macOS (launchd) are supported");
    }

    Ok(())
}

pub fn handle_service_logs() -> Result<()> {
    if is_linux() {
        let status = Command::new("journalctl")
            .args(["--user", "-u", "goosed", "-f", "--no-pager", "-n", "50"])
            .status()
            .context("failed to run journalctl")?;
        if !status.success() {
            bail!("journalctl exited with non-zero status");
        }
    } else if is_macos() {
        let stdout_log = PathBuf::from("/tmp/goosed.stdout.log");
        let stderr_log = PathBuf::from("/tmp/goosed.stderr.log");

        if stdout_log.exists() {
            println!("=== stdout ({}) ===", stdout_log.display());
            let content = fs::read_to_string(&stdout_log).context("failed to read stdout log")?;
            print!("{content}");
        } else {
            println!("No stdout log at {}", stdout_log.display());
        }

        if stderr_log.exists() {
            println!("\n=== stderr ({}) ===", stderr_log.display());
            let content = fs::read_to_string(&stderr_log).context("failed to read stderr log")?;
            print!("{content}");
        } else {
            println!("No stderr log at {}", stderr_log.display());
        }
    } else {
        bail!("unsupported platform; only Linux (systemd) and macOS (launchd) are supported");
    }

    Ok(())
}
