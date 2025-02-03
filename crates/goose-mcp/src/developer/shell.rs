use std::env;

#[derive(Debug, Clone)]
pub struct ShellConfig {
    pub executable: String,
    pub arg: String,
    pub redirect_syntax: String,
}

impl Default for ShellConfig {
    fn default() -> Self {
        if cfg!(windows) {
            // Prefer PowerShell over cmd.exe for better functionality
            if std::process::Command::new("powershell.exe")
                .arg("-Command")
                .arg("exit")
                .output()
                .is_ok()
            {
                Self {
                    executable: "powershell.exe".to_string(),
                    arg: "-Command".to_string(),
                    redirect_syntax: "2>&1".to_string(), // PowerShell supports Unix-style redirection
                }
            } else {
                Self {
                    executable: "cmd.exe".to_string(),
                    arg: "/C".to_string(),
                    redirect_syntax: "2>&1".to_string(), // cmd.exe also supports this syntax
                }
            }
        } else {
            Self {
                executable: "bash".to_string(),
                arg: "-c".to_string(),
                redirect_syntax: "2>&1".to_string(),
            }
        }
    }
}

pub fn get_shell_config() -> ShellConfig {
    ShellConfig::default()
}

pub fn format_command_for_platform(command: &str) -> String {
    let config = get_shell_config();
    if cfg!(windows) && config.executable == "powershell.exe" {
        // For Windows PowerShell, wrap the command in braces
        format!("{{ {} }} {}", command, config.redirect_syntax)
    } else {
        // For cmd.exe and Unix shells, no braces needed
        format!("{} {}", command, config.redirect_syntax)
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
