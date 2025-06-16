use std::env;

#[derive(Debug, Clone)]
pub struct ShellConfig {
    pub executable: String,
    pub arg: String,
}

impl Default for ShellConfig {
    fn default() -> Self {
        if cfg!(windows) {
            // Execute PowerShell commands directly
            Self {
                executable: "powershell.exe".to_string(),
                arg: "-NoProfile -NonInteractive -Command".to_string(),
            }
        } else {
            Self {
                executable: "bash".to_string(),
                arg: "-c".to_string(),
            }
        }
    }
}

pub fn get_shell_config() -> ShellConfig {
    ShellConfig::default()
}

pub fn format_command_for_platform(command: &str) -> String {
    if cfg!(windows) {
        // For PowerShell, wrap the command in braces to handle special characters
        format!("{{ {} }}", command)
    } else {
        // For other shells, no braces needed
        command.to_string()
    }
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
