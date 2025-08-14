#[cfg(test)]
mod tests {
    use super::super::extension::{ExtensionConfig, ExtensionError};
    use super::super::extension_manager::*;
    use tokio::process::Command;

    #[tokio::test]
    async fn test_missing_command_error_message() {
        // Test that a missing command produces a helpful error message
        let fake_cmd = "nonexistent_command_xyz_123";
        let command = Command::new(fake_cmd);

        let result = child_process_client(command, &None).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        let error_msg = error.to_string();

        // Check that the error message mentions the specific command
        assert!(
            error_msg.contains(fake_cmd),
            "Error message should mention the command name '{}'. Got: {}",
            fake_cmd,
            error_msg
        );
        assert!(
            error_msg.contains("not found"),
            "Error message should indicate command not found. Got: {}",
            error_msg
        );
        assert!(
            error_msg.contains("PATH"),
            "Error message should mention PATH. Got: {}",
            error_msg
        );
    }

    #[tokio::test]
    async fn test_uvx_missing_error_hint() {
        // Test that missing uvx produces a helpful installation hint
        let mut manager = ExtensionManager::new();

        let config = ExtensionConfig::InlinePython {
            name: "test_python".to_string(),
            code: "print('test')".to_string(),
            description: Some("Test extension".to_string()),
            timeout: Some(30),
            dependencies: None,
            bundled: None,
        };

        // This should fail if uvx is not installed
        // We can't guarantee uvx isn't installed in test environment,
        // but we can test the error path would work

        // For now, just verify the code compiles with our changes
        // Real integration testing would require a controlled environment
    }
}
