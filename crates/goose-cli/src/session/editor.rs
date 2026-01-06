use anyhow::Result;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::process::Command;
use tempfile::Builder;
use tempfile::NamedTempFile;

/// Get the editor command from config
fn get_editor_command() -> String {
    use goose::config::Config;

    // Try config first
    if let Ok(Some(editor)) = Config::global().get_goose_prompt_editor() {
        return editor;
    }

    // Fall back to default editor
    "vi".into()
}

/// Create temporary markdown file with conversation history
fn create_temp_file(messages: &[&str]) -> Result<NamedTempFile> {
    // Create a temporary file with a specific prefix and .md extension
    let temp_file = Builder::new()
        .prefix("goose_prompt_")
        .suffix(".md")
        .tempfile()?;

    // Write the structure with "Your prompt" first, then conversation history
    let mut content = String::from("# Goose Prompt Editor\n\n");

    // Add "Your prompt:" section first
    content.push_str("# Your prompt:\n\n");

    // Then add conversation history (newest first)
    if !messages.is_empty() {
        content.push_str("# Recent conversation for context (newest first):\n\n");
        // Reverse the messages to show newest first
        for message in messages.iter().rev() {
            content.push_str(&format!("{}\n", message));
        }
        content.push('\n');
    }

    fs::write(temp_file.path(), content)?;
    Ok(temp_file)
}

/// RAII guard to ensure symlink is cleaned up even on panic
struct SymlinkCleanup {
    symlink_path: PathBuf,
}

impl SymlinkCleanup {
    fn new(symlink_path: PathBuf) -> Self {
        Self { symlink_path }
    }
}

impl Drop for SymlinkCleanup {
    fn drop(&mut self) {
        // Always try to clean up the symlink, ignoring any errors
        let _ = std::fs::remove_file(&self.symlink_path);
    }
}

/// Launch editor and wait for completion
fn launch_editor(editor_cmd: &str, file_path: &PathBuf) -> Result<()> {
    use std::process::Stdio;

    // Split editor command and arguments
    let parts: Vec<&str> = editor_cmd.split_whitespace().collect();
    if parts.is_empty() {
        return Err(anyhow::anyhow!("Empty editor command"));
    }

    let mut cmd = Command::new(parts[0]);
    if let Ok(cwd) = std::env::current_dir() {
        cmd.current_dir(cwd);
    }
    if parts.len() > 1 {
        cmd.args(&parts[1..]);
    }
    cmd.arg(file_path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let status = cmd.status()?;

    if !status.success() {
        return Err(anyhow::anyhow!(
            "Editor exited with non-zero status: {}",
            status.code().unwrap_or(-1)
        ));
    }

    Ok(())
}

/// Main function to get input from editor
pub fn get_editor_input(messages: &[&str]) -> Result<(String, bool)> {
    // Create temporary file with context
    let temp_file = create_temp_file(messages)?;
    let temp_path = temp_file.path().to_path_buf();

    // Create a symlink in the current directory (project directory)
    let symlink_path = PathBuf::from(".goose_prompt_temp.md");

    // Remove existing symlink if it exists
    if symlink_path.exists() {
        std::fs::remove_file(&symlink_path)?;
    }

    // Create the symlink - handle both Unix and Windows
    #[cfg(unix)]
    std::os::unix::fs::symlink(&temp_path, &symlink_path)?;

    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&temp_path, &symlink_path)?;

    // Create RAII guard to ensure symlink cleanup even on panic or error
    let _cleanup_guard = SymlinkCleanup::new(symlink_path.clone());

    // Store the original template for comparison
    let _original_template = {
        let mut template_content = String::from("# Goose Prompt Editor\n\n");
        // Add "Your prompt:" section first
        template_content.push_str("# Your prompt:\n\n");
        if !messages.is_empty() {
            template_content.push_str("# Recent conversation for context (newest first):\n\n");
            // Reverse the messages to show newest first
            for message in messages.iter().rev() {
                template_content.push_str(&format!("{}\n", message));
            }
            template_content.push('\n');
        }
        template_content
    };

    // Get editor command
    let editor_cmd = get_editor_command();

    // Launch editor with the symlink path
    launch_editor(&editor_cmd, &symlink_path)?;

    // Read the edited content from the symlink (which points to the temp file)
    let mut content = String::new();
    let mut file = std::fs::File::open(&symlink_path)?;
    file.read_to_string(&mut content)?;

    // Extract user input (remove our template headers)
    let user_input = extract_user_input(&content);

    // Check if the user actually made changes (wrote something meaningful)
    let has_meaningful_content = !user_input.trim().is_empty();

    // The symlink will be automatically cleaned up by the Drop trait of _cleanup_guard
    Ok((user_input, has_meaningful_content))
}

/// Extract only the user's input from the markdown file
fn extract_user_input(content: &str) -> String {
    // Find the "# Your prompt:" line and return everything after it
    if let Some(start) = content.find("# Your prompt:") {
        let marker_len = "# Your prompt:".len();
        #[allow(clippy::string_slice)]
        let user_section = &content[start + marker_len..];

        // Look for the conversation history heading and stop there if found
        let end_patterns = [
            "# Recent conversation for context",
            "# Recent conversation for context (newest first):",
        ];

        let mut end_pos = None;
        for pattern in &end_patterns {
            if let Some(pos) = user_section.find(pattern) {
                end_pos = Some(pos);
                break;
            }
        }

        let user_input_section = match end_pos {
            Some(pos) =>
            {
                #[allow(clippy::string_slice)]
                &user_section[..pos]
            }
            None => user_section,
        };

        // Trim leading and trailing whitespace
        user_input_section.trim().to_string()
    } else {
        // If we can't find our marker, return the whole content
        content.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_extract_user_input_with_editor_output() {
        // Test the actual case where our fake editor outputs both header and response
        let content = r#"# Goose Prompt Editor

# Your prompt:
This is the hardcoded prompt response
# Recent conversation for context (newest first):

## User: Hello
## Assistant: Hi there!
"#;

        let result = extract_user_input(content);

        // Should return just the user input, not include our template
        assert_eq!(result, "This is the hardcoded prompt response");
    }

    #[test]
    fn test_extract_user_input_multiline() {
        let content = r#"# Goose Prompt Editor

# Your prompt:
This is the user's actual input
with multiple lines.
# Recent conversation for context:
"#;

        let result = extract_user_input(content);
        assert_eq!(
            result,
            "This is the user's actual input\nwith multiple lines."
        );
    }

    #[test]
    fn test_extract_user_input_no_marker() {
        let content = "Just plain text without markers";
        let result = extract_user_input(content);
        assert_eq!(result, "Just plain text without markers");
    }

    #[test]
    fn test_extract_user_input_conversation_history_heading() {
        // Test that the function stops when it finds the conversation history heading
        let content = r#"# Goose Prompt Editor

# Your prompt:
This is the user's input

# Recent conversation for context (newest first):

## User: Previous message
## Assistant: Previous response
"#;

        let result = extract_user_input(content);
        assert_eq!(result, "This is the user's input");
    }

    #[test]
    fn test_create_temp_file_with_messages() {
        let messages = vec!["## User: Hello", "## Assistant: Hi there!"];

        let temp_file = create_temp_file(&messages).unwrap();
        let path = temp_file.path();

        // Verify file exists and has correct content
        assert!(path.exists());
        assert!(path.to_str().unwrap().contains("goose_prompt_"));
        assert!(path.to_str().unwrap().ends_with(".md"));

        let content = fs::read_to_string(path).unwrap();
        assert!(content.contains("# Goose Prompt Editor"));
        assert!(content.contains("## User: Hello"));
        assert!(content.contains("## Assistant: Hi there!"));
        assert!(content.contains("# Your prompt:"));
        assert!(content.contains("# Recent conversation for context (newest first):"));

        // File is automatically cleaned up when temp_file goes out of scope
    }

    #[test]
    fn test_create_temp_file_with_prefix_suffix() {
        // Test using Builder pattern like in tempfile tests
        let temp_file = Builder::new()
            .prefix("goose_test_")
            .suffix(".md")
            .tempfile()
            .unwrap();

        let name = temp_file.path().file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with("goose_test_"));
        assert!(name.ends_with(".md"));
    }

    #[test]
    fn test_extract_user_input() {
        let content = r#"# Goose Prompt Editor

# Recent conversation for context:

# Your prompt:
This is the user's actual input
with multiple lines.
"#;

        let result = extract_user_input(content);
        assert_eq!(
            result,
            "This is the user's actual input\nwith multiple lines."
        );
    }

    #[test]
    fn test_tempfile_cleanup() {
        // Test that temporary files are cleaned up automatically
        let path = {
            let temp_file = Builder::new()
                .prefix("goose_cleanup_test_")
                .tempfile()
                .unwrap();
            let path = temp_file.path().to_path_buf();
            assert!(path.exists());
            path
        };

        // File should be automatically deleted when temp_file goes out of scope
        assert!(!path.exists());
    }

    #[test]
    fn test_editor_command_detection() {
        let result = get_editor_command();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_message_ordering_newest_first() {
        let messages = vec![
            "## User: First message",
            "## Assistant: First response",
            "## User: Second message",
            "## Assistant: Second response",
            "## User: Third message (newest)",
        ];

        let temp_file = create_temp_file(&messages).unwrap();
        let content = fs::read_to_string(temp_file.path()).unwrap();

        // Verify that messages are in reverse order (newest first)
        let newest_first = [
            "## User: Third message (newest)",
            "## Assistant: Second response",
            "## User: Second message",
            "## Assistant: First response",
            "## User: First message",
        ];

        for expected_msg in &newest_first {
            assert!(
                content.contains(expected_msg),
                "Expected to find message '{}' in content",
                expected_msg
            );
        }

        // Verify that the newest message appears before the oldest
        let newest_pos = content.find("## User: Third message (newest)").unwrap();
        let oldest_pos = content.find("## User: First message").unwrap();
        assert!(
            newest_pos < oldest_pos,
            "Newest message should appear before oldest message"
        );
    }

    #[test]
    fn test_symlink_raii_cleanup_on_panic() {
        use std::os::unix::fs;
        use std::panic;

        let messages = vec!["## User: Test message for panic cleanup"];
        let temp_file = create_temp_file(&messages).unwrap();
        let temp_path = temp_file.path().to_path_buf();

        // Use a unique filename for this test
        let symlink_path = PathBuf::from(format!("test_panic_cleanup_{}.md", std::process::id()));

        // Remove existing symlink if it exists
        if symlink_path.exists() {
            let _ = std::fs::remove_file(&symlink_path);
        }

        // Verify symlink doesn't exist initially
        assert!(
            !symlink_path.exists(),
            "Symlink should not exist before test"
        );

        // Create the symlink
        #[cfg(unix)]
        fs::symlink(&temp_path, &symlink_path).unwrap();

        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&temp_path, &symlink_path).unwrap();

        // Verify symlink was created
        assert!(symlink_path.exists(), "Symlink should exist after creation");

        // Test that the RAII cleanup guard works by creating one and triggering a panic
        let cleanup_guard = SymlinkCleanup::new(symlink_path.clone());

        // Trigger a panic to simulate an error condition
        let result = panic::catch_unwind(|| {
            let _guard = cleanup_guard;
            panic!("Simulating a panic to test cleanup");
        });

        // The panic should have been caught
        assert!(result.is_err(), "Panic should have been caught");

        // Verify that the symlink was cleaned up by the Drop trait despite the panic
        assert!(
            !symlink_path.exists(),
            "Symlink should be cleaned up even after panic"
        );
    }

    #[test]
    fn test_symlink_creation_and_cleanup() {
        use std::os::unix::fs;

        let messages = vec!["## User: Test message"];
        let temp_file = create_temp_file(&messages).unwrap();
        let temp_path = temp_file.path().to_path_buf();

        // Use a more unique filename to avoid conflicts
        let symlink_path = PathBuf::from(format!("test_symlink_cleanup_{}.md", std::process::id()));

        // Remove existing symlink if it exists (handle both files and symlinks)
        if symlink_path.exists() {
            let _ = std::fs::remove_file(&symlink_path);
        }

        // Ensure it's actually gone before creating symlink
        assert!(
            !symlink_path.exists(),
            "Symlink should be removed before creating new one"
        );

        // Create the symlink
        #[cfg(unix)]
        fs::symlink(&temp_path, &symlink_path).unwrap();

        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&temp_path, &symlink_path).unwrap();

        // Verify symlink was created and points to the temp file
        assert!(symlink_path.exists());

        // Verify content can be read through symlink
        let content = std::fs::read_to_string(&symlink_path).unwrap();
        assert!(content.contains("## User: Test message"));

        // Verify symlink points to the correct target
        #[cfg(unix)]
        {
            let read_link = std::fs::read_link(&symlink_path).unwrap();
            assert_eq!(read_link, temp_path);
        }

        #[cfg(windows)]
        {
            // On Windows, we can verify the file exists and contains expected content
            assert!(temp_path.exists());
            let temp_content = std::fs::read_to_string(&temp_path).unwrap();
            assert_eq!(content, temp_content);
        }

        // Clean up
        let _ = std::fs::remove_file(&symlink_path);
        assert!(!symlink_path.exists());
    }
}
