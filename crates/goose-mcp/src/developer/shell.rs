use std::{env, process::Stdio};

#[cfg(unix)]
#[allow(unused_imports)] // False positive: trait is used for process_group method  
use std::os::unix::process::CommandExt;

#[derive(Debug, Clone)]
pub struct ShellConfig {
    pub executable: String,
    pub args: Vec<String>,
}

impl Default for ShellConfig {
    fn default() -> Self {
        if cfg!(windows) {
            // Detect the default shell on Windows
            #[cfg(windows)]
            {
                Self::detect_windows_shell()
            }
            #[cfg(not(windows))]
            {
                // This branch should never be taken on non-Windows
                // but we need it for compilation
                Self {
                    executable: "cmd".to_string(),
                    args: vec!["/c".to_string()],
                }
            }
        } else {
            // Use bash on Unix/macOS (keep existing behavior)
            Self {
                executable: "bash".to_string(),
                args: vec!["-c".to_string()],
            }
        }
    }
}

impl ShellConfig {
    #[cfg(windows)]
    fn detect_windows_shell() -> Self {
        // Check for PowerShell first (more modern)
        if let Ok(ps_path) = which::which("pwsh") {
            // PowerShell 7+ (cross-platform PowerShell)
            Self {
                executable: ps_path.to_string_lossy().to_string(),
                args: vec![
                    "-NoProfile".to_string(),
                    "-NonInteractive".to_string(),
                    "-Command".to_string(),
                ],
            }
        } else if let Ok(ps_path) = which::which("powershell") {
            // Windows PowerShell 5.1
            Self {
                executable: ps_path.to_string_lossy().to_string(),
                args: vec![
                    "-NoProfile".to_string(),
                    "-NonInteractive".to_string(),
                    "-Command".to_string(),
                ],
            }
        } else {
            // Fall back to cmd.exe
            Self {
                executable: "cmd".to_string(),
                args: vec!["/c".to_string()],
            }
        }
    }
}

pub fn get_shell_config() -> ShellConfig {
    ShellConfig::default()
}

pub fn expand_path(path_str: &str) -> String {
    if cfg!(windows) {
        // Expand Windows environment variables (%VAR%)
        let with_userprofile = path_str.replace(
            "%USERPROFILE%",
            &env::var("USERPROFILE").unwrap_or_default(),
        );
        // Add more Windows environment variables as needed
        with_userprofile.replace("%APPDATA%", &env::var("APPDATA").unwrap_or_default())
    } else {
        // Unix-style expansion
        shellexpand::tilde(path_str).into_owned()
    }
}

pub fn is_absolute_path(path_str: &str) -> bool {
    if cfg!(windows) {
        // Check for Windows absolute paths (drive letters and UNC)
        path_str.contains(":\\") || path_str.starts_with("\\\\")
    } else {
        // Unix absolute paths start with /
        path_str.starts_with('/')
    }
}

pub fn normalize_line_endings(text: &str) -> String {
    if cfg!(windows) {
        // Ensure CRLF line endings on Windows
        text.replace("\r\n", "\n").replace("\n", "\r\n")
    } else {
        // Ensure LF line endings on Unix
        text.replace("\r\n", "\n")
    }
}

/// Configure a shell command with process group support for proper child process tracking.
///
/// On Unix systems, creates a new process group so child processes can be killed together.
/// On Windows, the default behavior already supports process tree termination.
pub fn configure_shell_command(shell_config: &ShellConfig, command: &str) -> tokio::process::Command {
    let mut command_builder = tokio::process::Command::new(&shell_config.executable);
    command_builder
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .kill_on_drop(true)
        .env("GOOSE_TERMINAL", "1")
        .args(&shell_config.args)
        .arg(command);

    // On Unix systems, create a new process group so we can kill child processes
    #[cfg(unix)]
    {
        command_builder.process_group(0);
        tracing::info!("Configured process to run in new process group for proper child process management");
    }

    command_builder
}

/// Kill a process and all its child processes using platform-specific approaches.
///
/// On Unix systems, kills the entire process group.
/// On Windows, kills the process tree.
pub async fn kill_process_group(
    child: &mut tokio::process::Child,
    pid: Option<u32>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(unix)]
    {
        if let Some(pid) = pid {
            let sigterm_result = unsafe {
                // Kill the process group (negative PID) - this kills the shell and all child processes
                libc::kill(-(pid as i32), libc::SIGTERM)
            };
            
            // Wait a brief moment for graceful shutdown
            tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
            
            // Force kill the process group with SIGKILL if processes are still running
            let sigkill_result = unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL)
            };
        }
        
        // Also use tokio's kill as a fallback
        match child.kill().await {
            Ok(_) => {
                tracing::info!("Successfully killed process using tokio child.kill()");
            }
            Err(e) => {
                tracing::warn!("Failed to kill process using tokio child.kill(): {}", e);
            }
        }
    }
    
    #[cfg(windows)]
    {
        if let Some(pid) = pid {
            // Use taskkill to kill the process tree on Windows
            let kill_result = tokio::process::Command::new("taskkill")
                .args(&["/F", "/T", "/PID", &pid.to_string()])
                .output()
                .await;
            
            match kill_result {
                Ok(output) if output.status.success() => {
                    tracing::info!("Successfully killed Windows process tree for PID {}", pid);
                }
                Ok(output) => {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    tracing::warn!("taskkill failed for PID {}: {}", pid, stderr);
                }
                Err(e) => {
                    tracing::error!("Failed to execute taskkill for PID {}: {}", pid, e);
                }
            }
        }
        
        // Also use tokio's kill as a fallback
        match child.kill().await {
            Ok(_) => {
                tracing::info!("Successfully killed process using tokio child.kill()");
            }
            Err(e) => {
                tracing::warn!("Failed to kill process using tokio child.kill(): {}", e);
            }
        }
    }
    
    Ok(())
}
