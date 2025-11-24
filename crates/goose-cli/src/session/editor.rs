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
    let file_path = temp_file.path().to_path_buf();

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

    // Launch editor
    launch_editor(&editor_cmd, &file_path)?;

    // Read the edited content
    let mut content = String::new();
    let mut file = fs::File::open(&file_path)?;
    file.read_to_string(&mut content)?;

    // Extract user input (remove our template headers)
    let user_input = extract_user_input(&content);

    // Check if the user actually made changes (wrote something meaningful)
    let has_meaningful_content = !user_input.trim().is_empty();

    // Clean up is automatic when temp_file goes out of scope
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
}
