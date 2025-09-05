use rmcp::{
    model::{ErrorCode, Role},
    service::RequestContext,
    RoleServer, ServerHandler,
};
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use crate::developer::rmcp_developer::{SearchCodeParams, ShellParams, TextEditorParams};
    use crate::DeveloperServer;

    use super::*;
    use rmcp::handler::server::tool::Parameters;
    use rmcp::model::NumberOrString;
    use rmcp::service::serve_directly;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_server() -> DeveloperServer {
        DeveloperServer::new()
    }

    /// Creates a test transport using in-memory streams instead of stdio
    /// This avoids the hanging issues caused by multiple tests competing for stdio
    fn create_test_transport() -> impl rmcp::transport::IntoTransport<
        RoleServer,
        std::io::Error,
        rmcp::transport::async_rw::TransportAdapterAsyncCombinedRW,
    > {
        let (_client, server) = tokio::io::duplex(1024);
        server
    }

    /// Helper function to run shell tests with proper runtime management
    /// This ensures clean shutdown and prevents hanging tests
    fn run_shell_test<F, Fut, T>(test_fn: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        // Create a separate runtime for this test to ensure clean shutdown
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(test_fn());

        // Force shutdown the runtime to kill ALL spawned tasks
        // This terminates the fire-and-forget tasks that rmcp doesn't track
        rt.shutdown_timeout(std::time::Duration::from_millis(100));

        // Return the test result
        result
    }

    /// Helper function to clean up test services and prevent hanging tests
    /// This should be called at the end of tests that create running services
    fn cleanup_test_service(
        running_service: rmcp::service::RunningService<RoleServer, DeveloperServer>,
        peer: rmcp::service::Peer<RoleServer>,
    ) {
        let cancellation_token = running_service.cancellation_token();
        cancellation_token.cancel();
        drop(peer);
        drop(running_service);
    }

    #[test]
    #[serial]
    fn test_shell_missing_parameters() {
        run_shell_test(|| async {
            let server = create_test_server();
            let running_service = serve_directly(server.clone(), create_test_transport(), None);
            let peer = running_service.peer().clone();

            let result = server
                .shell(
                    Parameters(ShellParams {
                        command: "".to_string(),
                    }),
                    RequestContext {
                        ct: Default::default(),
                        id: NumberOrString::Number(1),
                        meta: Default::default(),
                        extensions: Default::default(),
                        peer: peer.clone(),
                    },
                )
                .await;

            assert!(result.is_err());
            let err = result.err().unwrap();
            assert_eq!(err.code, ErrorCode::INVALID_PARAMS);

            // Force cleanup before runtime shutdown
            cleanup_test_service(running_service, peer);
        });
    }

    #[test]
    #[serial]
    #[cfg(windows)]
    fn test_windows_specific_commands() {
        run_shell_test(|| async {
            let temp_dir = tempfile::tempdir().unwrap();
            std::env::set_current_dir(&temp_dir).unwrap();

            let server = create_test_server();
            let running_service = serve_directly(server.clone(), create_test_transport(), None);
            let peer = running_service.peer().clone();

            // Test PowerShell command
            let shell_params = Parameters(ShellParams {
                command: "Get-ChildItem".to_string(),
            });

            let result = server
                .shell(
                    shell_params,
                    RequestContext {
                        ct: Default::default(),
                        id: NumberOrString::Number(1),
                        meta: Default::default(),
                        extensions: Default::default(),
                        peer: peer.clone(),
                    },
                )
                .await;

            assert!(result.is_err());

            // Test that resolve_path works with Windows paths
            let windows_path = r"C:\Windows\System32";
            if Path::new(windows_path).exists() {
                let resolved = server.resolve_path(windows_path);
                assert!(resolved.is_ok());
            }

            // Force cleanup before runtime shutdown
            cleanup_test_service(running_service, peer);
        });
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_size_limits() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();
        let server = create_test_server();

        // Test file size limit
        {
            let large_file_path = temp_dir.path().join("large.txt");

            // Create a file larger than 2MB
            let content = "x".repeat(3 * 1024 * 1024); // 3MB
            fs::write(&large_file_path, content).unwrap();

            let view_params = Parameters(TextEditorParams {
                path: large_file_path.to_str().unwrap().to_string(),
                command: "view".to_string(),
                view_range: None,
                file_text: None,
                old_str: None,
                new_str: None,
                insert_line: None,
            });

            let result = server.text_editor(view_params).await;

            assert!(result.is_err());
            let err = result.err().unwrap();
            assert_eq!(err.code, ErrorCode::INTERNAL_ERROR);
            assert!(err.to_string().contains("too large"));
        }

        // Test character count limit
        {
            let many_chars_path = temp_dir.path().join("many_chars.txt");

            // This is above MAX_FILE_SIZE
            let content = "x".repeat(500_000);
            fs::write(&many_chars_path, content).unwrap();

            let view_params = Parameters(TextEditorParams {
                path: many_chars_path.to_str().unwrap().to_string(),
                command: "view".to_string(),
                view_range: None,
                file_text: None,
                old_str: None,
                new_str: None,
                insert_line: None,
            });

            let result = server.text_editor(view_params).await;

            assert!(result.is_err());
            let err = result.err().unwrap();
            assert_eq!(err.code, ErrorCode::INTERNAL_ERROR);
            assert!(err.to_string().contains("is too large"));
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_write_and_view_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a new file
        let write_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some("Hello, world!".to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        server.text_editor(write_params).await.unwrap();

        // View the file
        let view_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "view".to_string(),
            view_range: None,
            file_text: None,
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        let view_result = server.text_editor(view_params).await.unwrap();

        assert!(!view_result.content.is_empty());
        let user_content = view_result
            .content
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();
        assert!(user_content.text.contains("Hello, world!"));
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_str_replace() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a new file
        let write_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some("Hello, world!".to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        server.text_editor(write_params).await.unwrap();

        // Replace string
        let replace_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "str_replace".to_string(),
            view_range: None,
            file_text: None,
            old_str: Some("world".to_string()),
            new_str: Some("Rust".to_string()),
            insert_line: None,
        });

        let replace_result = server.text_editor(replace_params).await.unwrap();

        let assistant_content = replace_result
            .content
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::Assistant))
            })
            .unwrap()
            .as_text()
            .unwrap();

        assert!(
            assistant_content.text.contains("The file")
                && assistant_content.text.contains("has been edited")
        );

        // Verify the file contents changed
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Hello, Rust!"));
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_undo_edit() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a file
        let write_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some("Original content".to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        server.text_editor(write_params).await.unwrap();

        // Make an edit
        let replace_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "str_replace".to_string(),
            view_range: None,
            file_text: None,
            old_str: Some("Original".to_string()),
            new_str: Some("Modified".to_string()),
            insert_line: None,
        });

        server.text_editor(replace_params).await.unwrap();

        // Verify the edit was made
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Modified content"));

        // Undo the edit
        let undo_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "undo_edit".to_string(),
            view_range: None,
            file_text: None,
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        let undo_result = server.text_editor(undo_params).await.unwrap();

        // Verify undo worked
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Original content"));

        let undo_content = undo_result
            .content
            .iter()
            .find(|c| c.as_text().is_some())
            .unwrap()
            .as_text()
            .unwrap();
        assert!(undo_content.text.contains("Undid the last edit"));
    }

    #[tokio::test]
    #[serial]
    async fn test_goose_ignore_basic_patterns() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create .gooseignore file with patterns
        fs::write(".gooseignore", "secret.txt\n*.env").unwrap();

        let server = create_test_server();

        // Test basic file matching
        assert!(
            server.is_ignored(Path::new("secret.txt")),
            "secret.txt should be ignored"
        );
        assert!(
            server.is_ignored(Path::new("./secret.txt")),
            "./secret.txt should be ignored"
        );
        assert!(
            !server.is_ignored(Path::new("not_secret.txt")),
            "not_secret.txt should not be ignored"
        );

        // Test pattern matching
        assert!(
            server.is_ignored(Path::new("test.env")),
            "*.env pattern should match test.env"
        );
        assert!(
            server.is_ignored(Path::new("./test.env")),
            "*.env pattern should match ./test.env"
        );
        assert!(
            !server.is_ignored(Path::new("test.txt")),
            "*.env pattern should not match test.txt"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_respects_ignore_patterns() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create .gooseignore file
        fs::write(".gooseignore", "secret.txt").unwrap();

        let server = create_test_server();

        // Try to write to an ignored file
        let secret_path = temp_dir.path().join("secret.txt");
        let write_params = Parameters(TextEditorParams {
            path: secret_path.to_str().unwrap().to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some("test content".to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        let result = server.text_editor(write_params).await;
        assert!(
            result.is_err(),
            "Should not be able to write to ignored file"
        );
        assert_eq!(result.unwrap_err().code, ErrorCode::INTERNAL_ERROR);

        // Try to write to a non-ignored file
        let allowed_path = temp_dir.path().join("allowed.txt");
        let write_params = Parameters(TextEditorParams {
            path: allowed_path.to_str().unwrap().to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some("test content".to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        let result = server.text_editor(write_params).await;
        assert!(
            result.is_ok(),
            "Should be able to write to non-ignored file"
        );
    }

    #[test]
    #[serial]
    fn test_shell_respects_ignore_patterns() {
        run_shell_test(|| async {
            let temp_dir = tempfile::tempdir().unwrap();
            std::env::set_current_dir(&temp_dir).unwrap();

            let server = create_test_server();
            let running_service = serve_directly(server.clone(), create_test_transport(), None);
            let peer = running_service.peer().clone();

            // Create an ignored file
            let secret_file_path = temp_dir.path().join("secrets.txt");
            fs::write(&secret_file_path, "secret content").unwrap();

            // try to cat the ignored file
            let result = server
                .shell(
                    Parameters(ShellParams {
                        command: format!("cat {}", secret_file_path.to_str().unwrap()),
                    }),
                    RequestContext {
                        ct: Default::default(),
                        id: NumberOrString::Number(1),
                        meta: Default::default(),
                        extensions: Default::default(),
                        peer: peer.clone(),
                    },
                )
                .await;

            assert!(result.is_err(), "Should not be able to cat ignored file");
            assert_eq!(result.unwrap_err().code, ErrorCode::INTERNAL_ERROR);

            // Try to cat a non-ignored file
            let allowed_file_path = temp_dir.path().join("allowed.txt");
            fs::write(&allowed_file_path, "allowed content").unwrap();

            let result = server
                .shell(
                    Parameters(ShellParams {
                        command: format!("cat {}", allowed_file_path.to_str().unwrap()),
                    }),
                    RequestContext {
                        ct: Default::default(),
                        id: NumberOrString::Number(1),
                        meta: Default::default(),
                        extensions: Default::default(),
                        peer: peer.clone(),
                    },
                )
                .await;

            assert!(result.is_ok(), "Should be able to cat non-ignored file");

            // Clean up
            let cancellation_token = running_service.cancellation_token();
            cancellation_token.cancel();
            drop(peer);
            drop(running_service);
        });
    }

    #[tokio::test]
    #[serial]
    async fn test_gitignore_fallback_when_no_gooseignore() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create .gitignore file (no .gooseignore)
        fs::write(".gitignore", "*.log").unwrap();

        let server = create_test_server();

        assert!(
            server.is_ignored(Path::new("debug.log")),
            "*.log pattern from .gitignore should match debug.log"
        );
        assert!(
            !server.is_ignored(Path::new("debug.txt")),
            "*.log pattern should not match debug.txt"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_gooseignore_takes_precedence_over_gitignore() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create both files
        fs::write(".gitignore", "*.log").unwrap();
        fs::write(".gooseignore", "*.env").unwrap();

        let server = create_test_server();

        // Should respect .gooseignore patterns
        assert!(
            server.is_ignored(Path::new("test.env")),
            ".gooseignore pattern should work"
        );
        // Should NOT respect .gitignore patterns when .gooseignore exists
        assert!(
            !server.is_ignored(Path::new("test.log")),
            ".gitignore patterns should be ignored when .gooseignore exists"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_descriptions() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Test without editor API configured (should be the case in tests due to cfg!(test))
        let server = create_test_server();

        // Get server info which contains tool descriptions
        let server_info = server.get_info();
        let instructions = server_info.instructions.unwrap_or_default();

        // Should use traditional description with str_replace command
        assert!(instructions.contains("Replace a string in a file with a new string"));
        assert!(instructions.contains("the `old_str` needs to exactly match one"));
        assert!(instructions.contains("str_replace"));

        // Should not contain editor API description or edit_file command
        assert!(!instructions.contains("Edit the file with the new content"));
        assert!(!instructions.contains("edit_file"));
        assert!(!instructions.contains("work out how to place old_str with it intelligently"));
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_respects_gitignore_fallback() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a .gitignore file but no .gooseignore
        fs::write(temp_dir.path().join(".gitignore"), "*.log").unwrap();

        let server = create_test_server();

        // Try to write to a file ignored by .gitignore
        let result = server
            .text_editor(Parameters(TextEditorParams {
                command: "write".to_string(),
                path: temp_dir
                    .path()
                    .join("test.log")
                    .to_str()
                    .unwrap()
                    .to_string(),
                file_text: Some("test content".parse().unwrap()),
                old_str: None,
                new_str: None,
                view_range: None,
                insert_line: None,
            }))
            .await;

        assert!(
            result.is_err(),
            "Should not be able to write to file ignored by .gitignore fallback"
        );
        assert_eq!(result.unwrap_err().code, ErrorCode::INTERNAL_ERROR);

        let result = server
            .text_editor(Parameters(TextEditorParams {
                command: "write".to_string(),
                path: temp_dir
                    .path()
                    .join("allowed.txt")
                    .to_str()
                    .unwrap()
                    .to_string(),
                file_text: Some("test content".to_string()),
                old_str: None,
                new_str: None,
                view_range: None,
                insert_line: None,
            }))
            .await;

        assert!(
            result.is_ok(),
            "Should be able to write to non-ignored file"
        );

        temp_dir.close().unwrap();
    }

    #[test]
    #[serial]
    fn test_shell_respects_gitignore_fallback() {
        run_shell_test(|| async {
            let temp_dir = tempfile::tempdir().unwrap();
            std::env::set_current_dir(&temp_dir).unwrap();

            // Create a .gitignore file but no .gooseignore
            std::fs::write(temp_dir.path().join(".gitignore"), "*.log").unwrap();

            let server = create_test_server();
            let running_service = serve_directly(server.clone(), create_test_transport(), None);
            let peer = running_service.peer().clone();

            // Create a file that would be ignored by .gitignore
            let log_file_path = temp_dir.path().join("test.log");
            std::fs::write(&log_file_path, "log content").unwrap();

            // Try to cat the ignored file
            let result = server
                .shell(
                    Parameters(ShellParams {
                        command: format!("cat {}", log_file_path.to_str().unwrap()),
                    }),
                    RequestContext {
                        ct: Default::default(),
                        id: NumberOrString::Number(1),
                        meta: Default::default(),
                        extensions: Default::default(),
                        peer: peer.clone(),
                    },
                )
                .await;

            assert!(
                result.is_err(),
                "Should not be able to cat file ignored by .gitignore fallback"
            );
            assert_eq!(result.unwrap_err().code, ErrorCode::INTERNAL_ERROR);

            // Try to cat a non-ignored file
            let allowed_file_path = temp_dir.path().join("allowed.txt");
            fs::write(&allowed_file_path, "allowed content").unwrap();

            let result = server
                .shell(
                    Parameters(ShellParams {
                        command: format!("cat {}", allowed_file_path.to_str().unwrap()),
                    }),
                    RequestContext {
                        ct: Default::default(),
                        id: NumberOrString::Number(1),
                        meta: Default::default(),
                        extensions: Default::default(),
                        peer: peer.clone(),
                    },
                )
                .await;

            assert!(result.is_ok(), "Should be able to cat non-ignored file");

            // Force cleanup before runtime shutdown
            cleanup_test_service(running_service, peer);

            temp_dir.close().unwrap();
        });
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_view_range() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a multi-line file
        let content =
            "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6\nLine 7\nLine 8\nLine 9\nLine 10";
        let write_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some(content.to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        server.text_editor(write_params).await.unwrap();

        // Test viewing specific range
        let view_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "view".to_string(),
            view_range: Some(vec![3, 6]),
            file_text: None,
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        let view_result = server.text_editor(view_params).await.unwrap();

        let text = view_result
            .content
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();

        // Should contain lines 3-6 with line numbers
        assert!(text.text.contains("3: Line 3"));
        assert!(text.text.contains("4: Line 4"));
        assert!(text.text.contains("5: Line 5"));
        assert!(text.text.contains("6: Line 6"));
        assert!(text.text.contains("(lines 3-6)"));
        // Should not contain other lines
        assert!(!text.text.contains("1: Line 1"));
        assert!(!text.text.contains("7: Line 7"));
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_view_range_to_end() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a multi-line file
        let content = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";
        let write_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some(content.to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        server.text_editor(write_params).await.unwrap();

        // Test viewing from line 3 to end using -1
        let view_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "view".to_string(),
            view_range: Some(vec![3, -1]),
            file_text: None,
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        let view_result = server.text_editor(view_params).await.unwrap();

        let text = view_result
            .content
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();

        // Should contain lines 3-5
        assert!(text.text.contains("3: Line 3"));
        assert!(text.text.contains("4: Line 4"));
        assert!(text.text.contains("5: Line 5"));
        assert!(text.text.contains("(lines 3-end)"));
        // Should not contain lines 1-2
        assert!(!text.text.contains("1: Line 1"));
        assert!(!text.text.contains("2: Line 2"));
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_view_range_invalid() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a small file
        let content = "Line 1\nLine 2\nLine 3";
        let write_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some(content.to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        server.text_editor(write_params).await.unwrap();

        // Test invalid range - start line beyond file
        let view_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "view".to_string(),
            view_range: Some(vec![10, 15]),
            file_text: None,
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        let result = server.text_editor(view_params).await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
        assert!(error.message.contains("beyond the end of the file"));
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_insert_at_beginning() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a file with some content
        let content = "Line 2\nLine 3\nLine 4";
        let write_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some(content.to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        server.text_editor(write_params).await.unwrap();

        // Insert at the beginning (line 0)
        let insert_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "insert".to_string(),
            view_range: None,
            file_text: None,
            old_str: None,
            new_str: Some("Line 1".to_string()),
            insert_line: Some(0),
        });

        let insert_result = server.text_editor(insert_params).await.unwrap();

        let text = insert_result
            .content
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::Assistant))
            })
            .unwrap()
            .as_text()
            .unwrap();

        assert!(text.text.contains("Text has been inserted at line 1"));

        // Verify the file content by reading it directly
        let file_content = fs::read_to_string(&file_path).unwrap();
        assert!(file_content.contains("Line 1\nLine 2\nLine 3\nLine 4"));
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_insert_in_middle() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a file with some content
        let content = "Line 1\nLine 2\nLine 4\nLine 5";
        let write_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some(content.to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        server.text_editor(write_params).await.unwrap();

        // Insert after line 2
        let insert_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "insert".to_string(),
            view_range: None,
            file_text: None,
            old_str: None,
            new_str: Some("Line 3".to_string()),
            insert_line: Some(2),
        });

        let insert_result = server.text_editor(insert_params).await.unwrap();

        let text = insert_result
            .content
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::Assistant))
            })
            .unwrap()
            .as_text()
            .unwrap();

        assert!(text.text.contains("Text has been inserted at line 3"));

        // Verify the file content by reading it directly
        let file_content = fs::read_to_string(&file_path).unwrap();
        let lines: Vec<&str> = file_content.lines().collect();
        assert_eq!(lines[0], "Line 1");
        assert_eq!(lines[1], "Line 2");
        assert_eq!(lines[2], "Line 3");
        assert_eq!(lines[3], "Line 4");
        assert_eq!(lines[4], "Line 5");
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_insert_at_end() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a file with some content
        let content = "Line 1\nLine 2\nLine 3";
        let write_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some(content.to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        server.text_editor(write_params).await.unwrap();

        // Insert at the end (after line 3)
        let insert_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "insert".to_string(),
            view_range: None,
            file_text: None,
            old_str: None,
            new_str: Some("Line 4".to_string()),
            insert_line: Some(3),
        });

        let insert_result = server.text_editor(insert_params).await.unwrap();

        let text = insert_result
            .content
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::Assistant))
            })
            .unwrap()
            .as_text()
            .unwrap();

        assert!(text.text.contains("Text has been inserted at line 4"));

        // Verify the file content by reading it directly
        let file_content = fs::read_to_string(&file_path).unwrap();
        assert!(file_content.contains("Line 1\nLine 2\nLine 3\nLine 4"));
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_insert_at_end_negative() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a file with some content
        let content = "Line 1\nLine 2\nLine 3";
        let write_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some(content.to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        server.text_editor(write_params).await.unwrap();

        // Insert at the end using -1
        let insert_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "insert".to_string(),
            view_range: None,
            file_text: None,
            old_str: None,
            new_str: Some("Line 4".to_string()),
            insert_line: Some(-1),
        });

        let insert_result = server.text_editor(insert_params).await.unwrap();

        let text = insert_result
            .content
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::Assistant))
            })
            .unwrap()
            .as_text()
            .unwrap();

        assert!(text.text.contains("Text has been inserted at line 4"));

        // Verify the file content by reading it directly
        let file_content = fs::read_to_string(&file_path).unwrap();
        assert!(file_content.contains("Line 1\nLine 2\nLine 3\nLine 4"));
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_insert_invalid_line() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a file with some content
        let content = "Line 1\nLine 2\nLine 3";
        let write_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some(content.to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        server.text_editor(write_params).await.unwrap();

        // Try to insert beyond the end of the file
        let insert_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "insert".to_string(),
            view_range: None,
            file_text: None,
            old_str: None,
            new_str: Some("Line 11".to_string()),
            insert_line: Some(10),
        });

        let result = server.text_editor(insert_params).await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
        assert!(err.message.contains("beyond the end of the file"));
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_insert_missing_parameters() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a file first
        let write_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some("Initial content".to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        server.text_editor(write_params).await.unwrap();

        // Test insert without new_str parameter
        let insert_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "insert".to_string(),
            view_range: None,
            file_text: None,
            old_str: None,
            new_str: None, // Missing required parameter
            insert_line: Some(1),
        });

        let result = server.text_editor(insert_params).await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
        assert!(error.message.contains("Missing 'new_str' parameter"));

        // Test insert without insert_line parameter
        let insert_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "insert".to_string(),
            view_range: None,
            file_text: None,
            old_str: None,
            new_str: Some("New text".to_string()),
            insert_line: None, // Missing required parameter
        });

        let result = server.text_editor(insert_params).await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert_eq!(error.code, ErrorCode::INVALID_PARAMS);
        assert!(error.message.contains("Missing 'insert_line' parameter"));
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_insert_with_undo() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a file with some content
        let content = "Line 1\nLine 2";
        let write_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some(content.to_string()),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        server.text_editor(write_params).await.unwrap();

        // Insert a line
        let insert_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "insert".to_string(),
            view_range: None,
            file_text: None,
            old_str: None,
            new_str: Some("Inserted Line".to_string()),
            insert_line: Some(1),
        });

        server.text_editor(insert_params).await.unwrap();

        // Undo the insert
        let undo_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "undo_edit".to_string(),
            view_range: None,
            file_text: None,
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        let undo_result = server.text_editor(undo_params).await.unwrap();

        let text = undo_result
            .content
            .iter()
            .find(|c| c.as_text().is_some())
            .unwrap()
            .as_text()
            .unwrap();
        assert!(text.text.contains("Undid the last edit"));

        // Verify the file is back to original content
        let file_content = fs::read_to_string(&file_path).unwrap();
        assert!(file_content.contains("Line 1\nLine 2"));
        assert!(!file_content.contains("Inserted Line"));
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_insert_nonexistent_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("nonexistent.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Try to insert into a nonexistent file
        let insert_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "insert".to_string(),
            view_range: None,
            file_text: None,
            old_str: None,
            new_str: Some("New line".to_string()),
            insert_line: Some(0),
        });

        let result = server.text_editor(insert_params).await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(err.code, ErrorCode::INVALID_PARAMS);
        assert!(err.message.contains("does not exist"));
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_view_large_file_without_range() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("large_file.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a file with more than 2000 lines (LINE_READ_LIMIT)
        let mut content = String::new();
        for i in 1..=2001 {
            content.push_str(&format!("Line {}\n", i));
        }

        let write_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some(content),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        server.text_editor(write_params).await.unwrap();

        // Test viewing without view_range - should trigger the error
        let view_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "view".to_string(),
            view_range: None,
            file_text: None,
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        let result = server.text_editor(view_params).await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert_eq!(err.code, ErrorCode::INTERNAL_ERROR);
        assert!(err.message.contains("2001 lines long"));
        assert!(err
            .message
            .contains("recommended to read in with view_range"));
        assert!(err
            .message
            .contains("please pass in view_range with [1, 2001]"));

        // Test viewing with view_range - should work
        let view_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "view".to_string(),
            view_range: Some(vec![1, 100]),
            file_text: None,
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        let result = server.text_editor(view_params).await;
        assert!(result.is_ok());

        let view_result = result.unwrap();
        let text = view_result
            .content
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();

        // Should contain lines 1-100
        assert!(text.text.contains("1: Line 1"));
        assert!(text.text.contains("100: Line 100"));
        assert!(!text.text.contains("101: Line 101"));

        // Test viewing with explicit full range - should work
        let view_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "view".to_string(),
            view_range: Some(vec![1, 2001]),
            file_text: None,
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        let result = server.text_editor(view_params).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_view_file_with_exactly_2000_lines() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("file_2000.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a file with exactly 2000 lines (should not trigger the check)
        let mut content = String::new();
        for i in 1..=2000 {
            content.push_str(&format!("Line {}\n", i));
        }

        let write_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some(content),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        server.text_editor(write_params).await.unwrap();

        // Test viewing without view_range - should work since it's exactly 2000 lines
        let view_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "view".to_string(),
            view_range: None,
            file_text: None,
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        let result = server.text_editor(view_params).await;

        assert!(result.is_ok());
        let view_result = result.unwrap();
        let text = view_result
            .content
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();

        // Should contain all lines
        assert!(text.text.contains("1: Line 1"));
        assert!(text.text.contains("2000: Line 2000"));
    }

    #[tokio::test]
    #[serial]
    async fn test_text_editor_view_small_file_without_range() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("small_file.txt");
        let file_path_str = file_path.to_str().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Create a file with less than 2000 lines
        let mut content = String::new();
        for i in 1..=100 {
            content.push_str(&format!("Line {}\n", i));
        }

        let write_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "write".to_string(),
            view_range: None,
            file_text: Some(content),
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        server.text_editor(write_params).await.unwrap();

        // Test viewing without view_range - should work fine
        let view_params = Parameters(TextEditorParams {
            path: file_path_str.to_string(),
            command: "view".to_string(),
            view_range: None,
            file_text: None,
            old_str: None,
            new_str: None,
            insert_line: None,
        });

        let result = server.text_editor(view_params).await;

        assert!(result.is_ok());
        let view_result = result.unwrap();
        let text = view_result
            .content
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::User))
            })
            .unwrap()
            .as_text()
            .unwrap();

        // Should contain all lines
        assert!(text.text.contains("1: Line 1"));
        assert!(text.text.contains("100: Line 100"));
    }

    #[test]
    #[serial]
    fn test_shell_output_truncation() {
        run_shell_test(|| async {
            let temp_dir = tempfile::tempdir().unwrap();
            std::env::set_current_dir(&temp_dir).unwrap();

            let server = create_test_server();
            let running_service = serve_directly(server.clone(), create_test_transport(), None);
            let peer = running_service.peer().clone();

            // Create a command that generates > 100 lines of output
            let command = if cfg!(windows) {
                "for /L %i in (1,1,150) do @echo Line %i"
            } else {
                "for i in {1..150}; do echo \"Line $i\"; done"
            };

            let result = server
                .shell(
                    Parameters(ShellParams {
                        command: command.to_string(),
                    }),
                    RequestContext {
                        ct: Default::default(),
                        id: NumberOrString::Number(1),
                        meta: Default::default(),
                        extensions: Default::default(),
                        peer: peer.clone(),
                    },
                )
                .await;

            // Should have two Content items
            assert_eq!(result.clone().unwrap().content.len(), 2);

            let content = result.clone().unwrap().content;

            // Find the Assistant and User content
            let assistant_content = content
                .iter()
                .find(|c| {
                    c.audience()
                        .is_some_and(|roles| roles.contains(&Role::Assistant))
                })
                .unwrap()
                .as_text()
                .unwrap();

            let user_content = content
                .iter()
                .find(|c| {
                    c.audience()
                        .is_some_and(|roles| roles.contains(&Role::User))
                })
                .unwrap()
                .as_text()
                .unwrap();

            // Assistant should get the full message with temp file info
            assert!(assistant_content
                .text
                .contains("private note: output was 150 lines"));

            // User should only get the truncated output with prefix
            assert!(user_content
                .text
                .starts_with("NOTE: Output was 150 lines, showing only the last 100 lines"));
            assert!(!user_content.text.contains("private note: output was"));

            // User output should contain lines 51-150 (last 100 lines)
            assert!(user_content.text.contains("Line 51"));
            assert!(user_content.text.contains("Line 150"));
            assert!(!user_content.text.contains("Line 50"));

            let start_tag = "remainder of lines in";
            let end_tag = "do not show tmp file to user";

            if let (Some(start), Some(end)) = (
                assistant_content.text.find(start_tag),
                assistant_content.text.find(end_tag),
            ) {
                let start_idx = start + start_tag.len();
                if start_idx < end {
                    let path = assistant_content.text[start_idx..end].trim();
                    println!("Extracted path: {}", path);

                    let file_contents =
                        std::fs::read_to_string(path).expect("Failed to read extracted temp file");

                    let lines: Vec<&str> = file_contents.lines().collect();

                    // Ensure we have exactly 150 lines
                    assert_eq!(lines.len(), 150, "Expected 150 lines in temp file");

                    // Ensure the first and last lines are correct
                    assert_eq!(lines.first(), Some(&"Line 1"), "First line mismatch");
                    assert_eq!(lines.last(), Some(&"Line 150"), "Last line mismatch");
                } else {
                    panic!("No path found in bash output truncation output");
                }
            } else {
                panic!("Failed to find start or end tag in bash output truncation output");
            }

            // Force cleanup before runtime shutdown
            cleanup_test_service(running_service, peer);

            temp_dir.close().unwrap();
        });
    }

    #[tokio::test]
    #[serial]
    async fn test_process_shell_output_short() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let server = create_test_server();

        // Test with short output (< 100 lines)
        let short_output = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";
        let result = server.process_shell_output(short_output).unwrap();

        // Both outputs should be the same for short outputs
        assert_eq!(result.0, short_output);
        assert_eq!(result.1, short_output);
    }

    #[tokio::test]
    #[serial]
    async fn test_process_shell_output_empty() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let server = create_test_server();

        // Test with empty output
        let empty_output = "";
        let result = server.process_shell_output(empty_output).unwrap();

        // Both outputs should be empty
        assert_eq!(result.0, "");
        assert_eq!(result.1, "");
    }

    #[test]
    #[serial]
    fn test_shell_output_without_trailing_newline() {
        run_shell_test(|| async {
            let temp_dir = tempfile::tempdir().unwrap();
            std::env::set_current_dir(&temp_dir).unwrap();

            let server = create_test_server();
            let running_service = serve_directly(server.clone(), create_test_transport(), None);
            let peer = running_service.peer().clone();

            // Test command that outputs content without a trailing newline
            let command = if cfg!(windows) {
                "echo|set /p=\"Content without newline\""
            } else {
                "printf 'Content without newline'"
            };

            let result = server
                .shell(
                    Parameters(ShellParams {
                        command: command.to_string(),
                    }),
                    RequestContext {
                        ct: Default::default(),
                        id: NumberOrString::Number(1),
                        meta: Default::default(),
                        extensions: Default::default(),
                        peer: peer.clone(),
                    },
                )
                .await;

            assert!(result.is_ok());

            // Test the output processing logic that would be used by shell method
            let output_without_newline = "Content without newline";
            let result = server.process_shell_output(output_without_newline).unwrap();

            // The output should contain the content even without a trailing newline
            assert!(
                result.0.contains("Content without newline"),
                "Output should contain content even without trailing newline, but got: {}",
                result.0
            );
            assert!(
                result.1.contains("Content without newline"),
                "User output should contain content even without trailing newline, but got: {}",
                result.1
            );

            // Both should be the same for short output
            assert_eq!(result.0, output_without_newline);
            assert_eq!(result.1, output_without_newline);

            // Force cleanup before runtime shutdown
            cleanup_test_service(running_service, peer);
        });
    }

    #[tokio::test]
    #[serial]
    async fn test_shell_output_handling_logic() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        let server = create_test_server();

        // Test output truncation logic with content without trailing newlines
        let content_without_newline = "Content without newline";
        let result = server
            .process_shell_output(content_without_newline)
            .unwrap();

        assert_eq!(result.0, content_without_newline);
        assert_eq!(result.1, content_without_newline);
        assert!(
            result.0.contains("Content without newline"),
            "Output processing should preserve content without trailing newlines"
        );

        // Test with content that has trailing newlines
        let content_with_newline = "Content with newline\n";
        let result = server.process_shell_output(content_with_newline).unwrap();
        assert_eq!(result.0, content_with_newline);
        assert_eq!(result.1, content_with_newline);

        // Test empty output handling
        let empty_output = "";
        let result = server.process_shell_output(empty_output).unwrap();
        assert_eq!(result.0, "");
        assert_eq!(result.1, "");
    }

    #[tokio::test]
    #[serial]
    async fn test_default_patterns_when_no_ignore_files() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Don't create any ignore files
        let server = create_test_server();

        // Default patterns should be used
        assert!(
            server.is_ignored(Path::new(".env")),
            ".env should be ignored by default patterns"
        );
        assert!(
            server.is_ignored(Path::new(".env.local")),
            ".env.local should be ignored by default patterns"
        );
        assert!(
            server.is_ignored(Path::new("secrets.txt")),
            "secrets.txt should be ignored by default patterns"
        );
        assert!(
            !server.is_ignored(Path::new("normal.txt")),
            "normal.txt should not be ignored"
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_global_goosehints() {
        // Note: This test checks if ~/.config/goose/.goosehints exists and includes it in instructions
        // Since RMCP version uses get_info() instead of instructions(), we test that method
        let global_hints_path =
            PathBuf::from(shellexpand::tilde("~/.config/goose/.goosehints").to_string());
        let global_hints_bak_path =
            PathBuf::from(shellexpand::tilde("~/.config/goose/.goosehints.bak").to_string());
        let mut globalhints_existed = false;

        if global_hints_path.is_file() {
            globalhints_existed = true;
            fs::copy(&global_hints_path, &global_hints_bak_path).unwrap();
        }

        fs::write(&global_hints_path, "These are my global goose hints.").unwrap();

        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let server = create_test_server();
        let server_info = server.get_info();

        assert!(server_info.instructions.is_some());
        let instructions = server_info.instructions.unwrap();
        assert!(instructions.contains("my global goose hints."));

        // restore backup if globalhints previously existed
        if globalhints_existed {
            fs::copy(&global_hints_bak_path, &global_hints_path).unwrap();
            fs::remove_file(&global_hints_bak_path).unwrap();
        } else {
            fs::remove_file(&global_hints_path).unwrap();
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_goosehints_with_file_references() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create referenced files
        let readme_path = temp_dir.path().join("README.md");
        std::fs::write(
            &readme_path,
            "# Project README\n\nThis is the project documentation.",
        )
        .unwrap();

        let guide_path = temp_dir.path().join("guide.md");
        std::fs::write(&guide_path, "# Development Guide\n\nFollow these steps...").unwrap();

        // Create .goosehints with references
        let hints_content = r#"# Project Information

Please refer to:
@README.md
@guide.md

Additional instructions here.
"#;
        let hints_path = temp_dir.path().join(".goosehints");
        std::fs::write(&hints_path, hints_content).unwrap();

        // Create server and check instructions
        let server = create_test_server();
        let server_info = server.get_info();

        assert!(server_info.instructions.is_some());
        let instructions = server_info.instructions.unwrap();

        // Should contain the .goosehints content
        assert!(instructions.contains("Project Information"));
        assert!(instructions.contains("Additional instructions here"));

        // Should contain the referenced files' content
        assert!(instructions.contains("# Project README"));
        assert!(instructions.contains("This is the project documentation"));
        assert!(instructions.contains("# Development Guide"));
        assert!(instructions.contains("Follow these steps"));

        // Should have attribution markers
        assert!(instructions.contains("--- Content from"));
        assert!(instructions.contains("--- End of"));
    }

    #[tokio::test]
    #[serial]
    async fn test_goosehints_when_present() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        fs::write(".goosehints", "Test hint content").unwrap();
        let server = create_test_server();
        let server_info = server.get_info();

        assert!(server_info.instructions.is_some());
        let instructions = server_info.instructions.unwrap();
        assert!(instructions.contains("Test hint content"));
    }

    #[tokio::test]
    #[serial]
    async fn test_goosehints_when_missing() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let server = create_test_server();
        let server_info = server.get_info();

        assert!(server_info.instructions.is_some());
        let instructions = server_info.instructions.unwrap();
        // When no hints are present, instructions should not contain hint content
        assert!(!instructions.contains("AGENTS.md:") && !instructions.contains(".goosehints:"));
    }

    #[tokio::test]
    #[serial]
    async fn test_goosehints_multiple_filenames() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        std::env::set_var("CONTEXT_FILE_NAMES", r#"["CLAUDE.md", ".goosehints"]"#);

        fs::write("CLAUDE.md", "Custom hints file content from CLAUDE.md").unwrap();
        fs::write(".goosehints", "Custom hints file content from .goosehints").unwrap();
        let server = create_test_server();
        let server_info = server.get_info();

        assert!(server_info.instructions.is_some());
        let instructions = server_info.instructions.unwrap();
        assert!(instructions.contains("Custom hints file content from CLAUDE.md"));
        assert!(instructions.contains("Custom hints file content from .goosehints"));
        std::env::remove_var("CONTEXT_FILE_NAMES");
    }

    #[tokio::test]
    #[serial]
    async fn test_goosehints_configurable_filename() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        std::env::set_var("CONTEXT_FILE_NAMES", r#"["CLAUDE.md"]"#);

        fs::write("CLAUDE.md", "Custom hints file content").unwrap();
        let server = create_test_server();
        let server_info = server.get_info();

        assert!(server_info.instructions.is_some());
        let instructions = server_info.instructions.unwrap();
        assert!(instructions.contains("Custom hints file content"));
        assert!(!instructions.contains(".goosehints")); // Make sure it's not loading the default
        std::env::remove_var("CONTEXT_FILE_NAMES");
    }

    #[tokio::test]
    #[serial]
    async fn test_search_code_basic_functionality() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create test files with searchable content
        fs::write(
            "test1.rs",
            "fn hello_world() {\n    println!(\"Hello, world!\");\n}",
        )
        .unwrap();
        fs::write("test2.rs", "fn main() {\n    hello_world();\n}").unwrap();
        fs::write("config.toml", "[dependencies]\nhello = \"1.0\"\n").unwrap();
        fs::write("readme.md", "This is a hello world example").unwrap();

        let server = create_test_server();
        let running_service = serve_directly(server.clone(), create_test_transport(), None);
        let peer = running_service.peer().clone();

        // Test searching for "hello" in content
        let search_params = Parameters(SearchCodeParams {
            search_terms: vec!["hello".to_string()],
            search_type: "content".to_string(),
            context_lines: 1,
            files_only: false,
            path: None,
        });

        let result = server
            .search_code(
                search_params,
                RequestContext {
                    ct: Default::default(),
                    id: NumberOrString::Number(1),
                    meta: Default::default(),
                    extensions: Default::default(),
                    peer: peer.clone(),
                },
            )
            .await;

        // Check if ripgrep is available
        let rg_check = std::process::Command::new("which").arg("rg").output();

        if rg_check.is_ok() && rg_check.unwrap().status.success() {
            // If ripgrep is available, the search should succeed
            assert!(
                result.is_ok(),
                "Search should succeed when ripgrep is available"
            );

            let search_result = result.unwrap();
            assert!(!search_result.content.is_empty());

            // Find the assistant content which contains the results
            let assistant_content = search_result
                .content
                .iter()
                .find(|c| {
                    c.audience()
                        .is_some_and(|roles| roles.contains(&Role::Assistant))
                })
                .unwrap()
                .as_text()
                .unwrap();

            // Should find hello in multiple files
            assert!(
                assistant_content.text.contains("test1.rs"),
                "Should find hello in test1.rs"
            );
            assert!(
                assistant_content.text.contains("test2.rs"),
                "Should find hello in test2.rs"
            );
            assert!(
                assistant_content.text.contains("config.toml"),
                "Should find hello in config.toml"
            );
            assert!(
                assistant_content.text.contains("readme.md"),
                "Should find hello in readme.md"
            );
        } else {
            // If ripgrep is not available, should return an error
            assert!(
                result.is_err(),
                "Search should fail when ripgrep is not available"
            );
            let err = result.err().unwrap();
            assert_eq!(err.code, ErrorCode::INTERNAL_ERROR);
            assert!(err.message.contains("ripgrep") || err.message.contains("rg"));
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_search_code_multiple_terms_and_modes() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::env::set_current_dir(&temp_dir).unwrap();

        // Create a more complex file structure
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir).unwrap();

        fs::write(
            src_dir.join("main.rs"),
            "fn process_data() {\n    let data = vec![1, 2, 3];\n    analyze(data);\n}",
        )
        .unwrap();
        fs::write(
            src_dir.join("lib.rs"),
            "pub fn analyze(data: Vec<i32>) {\n    // Process the data\n}",
        )
        .unwrap();
        fs::write("Cargo.toml", "[package]\nname = \"test_project\"\n").unwrap();
        fs::write("test.txt", "Some test data for analysis").unwrap();

        let server = create_test_server();
        let running_service = serve_directly(server.clone(), create_test_transport(), None);
        let peer = running_service.peer().clone();

        // Check if ripgrep is available first
        let rg_check = std::process::Command::new("which").arg("rg").output();

        if !rg_check.is_ok() || !rg_check.unwrap().status.success() {
            // Skip the test if ripgrep is not available
            return;
        }

        // Test 1: Search for multiple terms in content mode
        let search_params = Parameters(SearchCodeParams {
            search_terms: vec!["data".to_string(), "analyze".to_string()],
            search_type: "content".to_string(),
            context_lines: 0,
            files_only: false,
            path: None,
        });

        let result = server
            .search_code(
                search_params,
                RequestContext {
                    ct: Default::default(),
                    id: NumberOrString::Number(1),
                    meta: Default::default(),
                    extensions: Default::default(),
                    peer: peer.clone(),
                },
            )
            .await
            .unwrap();
        let assistant_content = result
            .content
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::Assistant))
            })
            .unwrap()
            .as_text()
            .unwrap();

        // Should find both terms - the tool formats results per term
        // Check that we have results for both search terms
        assert!(
            assistant_content.text.contains("data") || assistant_content.text.contains("Data"),
            "Should find results for 'data' term"
        );
        assert!(
            assistant_content.text.contains("analyze")
                || assistant_content.text.contains("Analyze"),
            "Should find results for 'analyze' term"
        );
        assert!(assistant_content.text.contains("main.rs"));
        assert!(assistant_content.text.contains("lib.rs"));

        // Test 2: Search for files containing a term (files_only mode)
        let search_params = Parameters(SearchCodeParams {
            search_terms: vec!["data".to_string()],
            search_type: "content".to_string(),
            context_lines: 0,
            files_only: true,
            path: None,
        });

        let result = server
            .search_code(
                search_params,
                RequestContext {
                    ct: Default::default(),
                    id: NumberOrString::Number(2),
                    meta: Default::default(),
                    extensions: Default::default(),
                    peer: peer.clone(),
                },
            )
            .await
            .unwrap();
        let assistant_content = result
            .content
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::Assistant))
            })
            .unwrap()
            .as_text()
            .unwrap();

        // Should list files only, not the actual matches
        assert!(
            assistant_content.text.contains("main.rs")
                || assistant_content.text.contains("src/main.rs")
        );
        assert!(
            assistant_content.text.contains("lib.rs")
                || assistant_content.text.contains("src/lib.rs")
        );
        assert!(assistant_content.text.contains("test.txt"));
        // When files_only is true, the output should be just file paths without match details
        // We can't assert that "fn process_data" is NOT there since the output format may vary

        // Test 3: Search for files by name pattern
        let search_params = Parameters(SearchCodeParams {
            search_terms: vec!["lib".to_string()],
            search_type: "files".to_string(),
            context_lines: 0,
            files_only: false,
            path: Some(src_dir.to_str().unwrap().to_string()),
        });

        let result = server
            .search_code(
                search_params,
                RequestContext {
                    ct: Default::default(),
                    id: NumberOrString::Number(3),
                    meta: Default::default(),
                    extensions: Default::default(),
                    peer: peer.clone(),
                },
            )
            .await
            .unwrap();
        let assistant_content = result
            .content
            .iter()
            .find(|c| {
                c.audience()
                    .is_some_and(|roles| roles.contains(&Role::Assistant))
            })
            .unwrap()
            .as_text()
            .unwrap();

        // Should find lib.rs in the src directory
        assert!(assistant_content.text.contains("lib.rs"));
        // Should not find main.rs since we're searching for "lib" in filenames
        assert!(!assistant_content.text.contains("main.rs"));
    }
}
