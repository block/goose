use std::env;

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
