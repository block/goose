use anyhow::Result;
use ignore::gitignore::Gitignore;
use indoc::indoc;
use mcp_core::{handler::ToolError, role::Role, tool::Tool, Content};
use serde_json::{json, Value};
use std::{env, path::Path, process::Stdio, sync::Arc};
use tokio::process::Command;

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

/// Creates the shell tool with OS-specific descriptions and configurations
pub fn create_shell_tool() -> Tool {
    let shell_tool_desc = match std::env::consts::OS {
        "windows" => indoc! {r#"
            Execute a command in the shell.

            This will return the output and error concatenated into a single string, as
            you would see from running on the command line. There will also be an indication
            of if the command succeeded or failed.

            Avoid commands that produce a large amount of output, and consider piping those outputs to files.

            **Important**: For searching files and code:

            Preferred: Use ripgrep (`rg`) when available - it respects .gitignore and is fast:
              - To locate a file by name: `rg --files | rg example.py`
              - To locate content inside files: `rg 'class Example'`

            Alternative Windows commands (if ripgrep is not installed):
              - To locate a file by name: `dir /s /b example.py`
              - To locate content inside files: `findstr /s /i "class Example" *.py`

            Note: Alternative commands may show ignored/hidden files that should be excluded.
        "#},
        _ => indoc! {r#"
            Execute a command in the shell.

            This will return the output and error concatenated into a single string, as
            you would see from running on the command line. There will also be an indication
            of if the command succeeded or failed.

            Avoid commands that produce a large amount of output, and consider piping those outputs to files.
            If you need to run a long lived command, background it - e.g. `uvicorn main:app &` so that
            this tool does not run indefinitely.

            **Important**: Each shell command runs in its own process. Things like directory changes or
            sourcing files do not persist between tool calls. So you may need to repeat them each time by
            stringing together commands, e.g. `cd example && ls` or `source env/bin/activate && pip install numpy`

            **Important**: Use ripgrep - `rg` - when you need to locate a file or a code reference, other solutions
            may show ignored or hidden files. For example *do not* use `find` or `ls -r`
              - List files by name: `rg --files | rg <filename>`
              - List files that contain a regex: `rg '<regex>' -l`
        "#},
    };

    Tool::new(
        "shell".to_string(),
        shell_tool_desc.to_string(),
        json!({
            "type": "object",
            "required": ["command"],
            "properties": {
                "command": {"type": "string"}
            }
        }),
        None,
    )
}

/// Execute a shell command with platform-specific handling and ignore pattern checking
pub async fn execute_shell_command(
    params: Value,
    ignore_patterns: &Arc<Gitignore>,
) -> Result<Vec<Content>, ToolError> {
    let command =
        params
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or(ToolError::InvalidParameters(
                "The command string is required".to_string(),
            ))?;

    // Check if command might access ignored files and return early if it does
    let cmd_parts: Vec<&str> = command.split_whitespace().collect();
    for arg in &cmd_parts[1..] {
        // Skip command flags
        if arg.starts_with('-') {
            continue;
        }
        // Skip invalid paths
        let path = Path::new(arg);
        if !path.exists() {
            continue;
        }

        if ignore_patterns.matched(path, false).is_ignore() {
            return Err(ToolError::ExecutionError(format!(
                "The command attempts to access '{}' which is restricted by .gooseignore",
                arg
            )));
        }
    }

    // Get platform-specific shell configuration
    let shell_config = get_shell_config();
    let cmd_with_redirect = format_command_for_platform(command);

    // Execute the command using platform-specific shell
    let child = Command::new(&shell_config.executable)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null())
        .kill_on_drop(true)
        .arg(&shell_config.arg)
        .arg(cmd_with_redirect)
        .spawn()
        .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

    // Wait for the command to complete and get output
    let output = child
        .wait_with_output()
        .await
        .map_err(|e| ToolError::ExecutionError(e.to_string()))?;

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    let output_str = stdout_str;

    // Check the character count of the output
    const MAX_CHAR_COUNT: usize = 400_000; // 409600 chars = 400KB
    let char_count = output_str.chars().count();
    if char_count > MAX_CHAR_COUNT {
        return Err(ToolError::ExecutionError(format!(
                "Shell output from command '{}' has too many characters ({}). Maximum character count is {}.",
                command,
                char_count,
                MAX_CHAR_COUNT
            )));
    }

    Ok(vec![
        Content::text(output_str.clone()).with_audience(vec![Role::Assistant]),
        Content::text(output_str)
            .with_audience(vec![Role::User])
            .with_priority(0.0),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use serial_test::serial;

    use ignore::gitignore::GitignoreBuilder;
    use std::sync::Arc;

    #[tokio::test]
    #[serial]
    async fn test_shell_missing_parameters() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let builder = GitignoreBuilder::new(temp_dir.path().to_path_buf());
        let ignore_patterns = Arc::new(builder.build().unwrap());

        let result = execute_shell_command(json!({}), &ignore_patterns).await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(matches!(err, ToolError::InvalidParameters(_)));

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    async fn test_bash_respects_ignore_patterns() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create ignore patterns
        let mut builder = GitignoreBuilder::new(temp_dir.path().to_path_buf());
        builder.add_line(None, "secret.txt").unwrap();
        let ignore_patterns = Arc::new(builder.build().unwrap());

        // Create an ignored file
        let secret_file_path = temp_dir.path().join("secret.txt");
        std::fs::write(&secret_file_path, "secret content").unwrap();

        // Try to cat the ignored file
        let result = execute_shell_command(
            json!({
                "command": format!("cat {}", secret_file_path.to_str().unwrap())
            }),
            &ignore_patterns,
        )
        .await;

        assert!(result.is_err(), "Should not be able to cat ignored file");
        assert!(matches!(result.unwrap_err(), ToolError::ExecutionError(_)));

        // Try to cat a non-ignored file
        let allowed_file_path = temp_dir.path().join("allowed.txt");
        std::fs::write(&allowed_file_path, "allowed content").unwrap();

        let result = execute_shell_command(
            json!({
                "command": format!("cat {}", allowed_file_path.to_str().unwrap())
            }),
            &ignore_patterns,
        )
        .await;

        assert!(result.is_ok(), "Should be able to cat non-ignored file");

        temp_dir.close().unwrap();
    }

    #[tokio::test]
    #[serial]
    #[cfg(windows)]
    async fn test_windows_specific_commands() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let builder = GitignoreBuilder::new(temp_dir.path().to_path_buf());
        let ignore_patterns = Arc::new(builder.build().unwrap());

        // Test PowerShell command
        let result = execute_shell_command(
            json!({
                "command": "Get-ChildItem"
            }),
            &ignore_patterns,
        )
        .await;
        assert!(result.is_ok());

        temp_dir.close().unwrap();
    }

    #[test]
    fn test_create_shell_tool() {
        let tool = create_shell_tool();
        assert_eq!(tool.name, "shell");
        assert!(!tool.description.is_empty());
    }

    #[test]
    fn test_shell_config_creation() {
        let config = get_shell_config();

        if cfg!(windows) {
            assert_eq!(config.executable, "powershell.exe");
            assert!(config.arg.contains("-NoProfile"));
        } else {
            assert_eq!(config.executable, "bash");
            assert_eq!(config.arg, "-c");
        }
    }

    #[test]
    fn test_path_expansion() {
        if cfg!(windows) {
            // Test Windows path expansion
            let path = "%USERPROFILE%\\test";
            let expanded = expand_path(path);
            assert!(!expanded.contains("%USERPROFILE%"));
        } else {
            // Test Unix path expansion
            let path = "~/test";
            let expanded = expand_path(path);
            assert!(!expanded.starts_with('~'));
        }
    }

    #[test]
    fn test_absolute_path_detection() {
        if cfg!(windows) {
            assert!(is_absolute_path("C:\\test"));
            assert!(is_absolute_path("\\\\server\\share"));
            assert!(!is_absolute_path("relative\\path"));
        } else {
            assert!(is_absolute_path("/absolute/path"));
            assert!(!is_absolute_path("relative/path"));
        }
    }

    #[test]
    fn test_line_ending_normalization() {
        let input = "line1\r\nline2\nline3";
        let normalized = normalize_line_endings(input);

        if cfg!(windows) {
            assert_eq!(normalized, "line1\r\nline2\r\nline3");
        } else {
            assert_eq!(normalized, "line1\nline2\nline3");
        }
    }
}
