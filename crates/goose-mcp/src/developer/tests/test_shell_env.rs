#[cfg(test)]
mod tests {
    use crate::developer::rmcp_developer::{DeveloperServer, ShellParams};
    use rmcp::{
        handler::server::wrapper::Parameters,
        model::NumberOrString,
        service::{serve_directly, RequestContext},
        RoleServer,
    };
    use serial_test::serial;
    use tempfile::TempDir;

    fn create_test_server() -> DeveloperServer {
        DeveloperServer::new()
    }

    /// Creates a test transport using in-memory streams instead of stdio
    fn create_test_transport() -> impl rmcp::transport::IntoTransport<
        RoleServer,
        std::io::Error,
        rmcp::transport::async_rw::TransportAdapterAsyncCombinedRW,
    > {
        let (_client, server) = tokio::io::duplex(1024);
        server
    }

    /// Helper function to run shell tests with proper runtime management
    fn run_shell_test<F, Fut, T>(test_fn: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(test_fn());
        rt.shutdown_timeout(std::time::Duration::from_millis(100));
        result
    }

    /// Helper function to clean up test services
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
    fn test_shell_environment_variable_values() {
        run_shell_test(|| async {
            let temp_dir = TempDir::new().unwrap();
            std::env::set_current_dir(&temp_dir).unwrap();

            let server = create_test_server();
            let running_service = serve_directly(server.clone(), create_test_transport(), None);
            let peer = running_service.peer().clone();

            // Test GOOSE_TERMINAL
            let result = server
                .shell(
                    Parameters(ShellParams {
                        command: "printenv GOOSE_TERMINAL".to_string(),
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

            assert!(result.is_ok(), "GOOSE_TERMINAL check should succeed");
            let result_content = result.unwrap();
            let output = result_content
                .content
                .iter()
                .find_map(|c| c.as_text())
                .expect("Should have text content");
            assert_eq!(output.text.trim(), "1", "GOOSE_TERMINAL should be '1'");

            // Test GIT_PAGER
            let result = server
                .shell(
                    Parameters(ShellParams {
                        command: "printenv GIT_PAGER".to_string(),
                    }),
                    RequestContext {
                        ct: Default::default(),
                        id: NumberOrString::Number(2),
                        meta: Default::default(),
                        extensions: Default::default(),
                        peer: peer.clone(),
                    },
                )
                .await;

            assert!(result.is_ok(), "GIT_PAGER check should succeed");
            let result_content = result.unwrap();
            let output = result_content
                .content
                .iter()
                .find_map(|c| c.as_text())
                .expect("Should have text content");
            assert_eq!(output.text.trim(), "cat", "GIT_PAGER should be 'cat'");

            // Test GIT_TERMINAL_PROMPT
            let result = server
                .shell(
                    Parameters(ShellParams {
                        command: "printenv GIT_TERMINAL_PROMPT".to_string(),
                    }),
                    RequestContext {
                        ct: Default::default(),
                        id: NumberOrString::Number(3),
                        meta: Default::default(),
                        extensions: Default::default(),
                        peer: peer.clone(),
                    },
                )
                .await;

            assert!(result.is_ok(), "GIT_TERMINAL_PROMPT check should succeed");
            let result_content = result.unwrap();
            let output = result_content
                .content
                .iter()
                .find_map(|c| c.as_text())
                .expect("Should have text content");
            assert_eq!(output.text.trim(), "0", "GIT_TERMINAL_PROMPT should be '0'");

            // Test GIT_CONFIG_GLOBAL
            let result = server
                .shell(
                    Parameters(ShellParams {
                        command: "printenv GIT_CONFIG_GLOBAL".to_string(),
                    }),
                    RequestContext {
                        ct: Default::default(),
                        id: NumberOrString::Number(4),
                        meta: Default::default(),
                        extensions: Default::default(),
                        peer: peer.clone(),
                    },
                )
                .await;

            assert!(result.is_ok(), "GIT_CONFIG_GLOBAL check should succeed");
            let result_content = result.unwrap();
            let output = result_content
                .content
                .iter()
                .find_map(|c| c.as_text())
                .expect("Should have text content");
            assert_eq!(
                output.text.trim(),
                "/dev/null",
                "GIT_CONFIG_GLOBAL should be '/dev/null'"
            );

            // Test GIT_CONFIG_SYSTEM
            let result = server
                .shell(
                    Parameters(ShellParams {
                        command: "printenv GIT_CONFIG_SYSTEM".to_string(),
                    }),
                    RequestContext {
                        ct: Default::default(),
                        id: NumberOrString::Number(5),
                        meta: Default::default(),
                        extensions: Default::default(),
                        peer: peer.clone(),
                    },
                )
                .await;

            assert!(result.is_ok(), "GIT_CONFIG_SYSTEM check should succeed");
            let result_content = result.unwrap();
            let output = result_content
                .content
                .iter()
                .find_map(|c| c.as_text())
                .expect("Should have text content");
            assert_eq!(
                output.text.trim(),
                "/dev/null",
                "GIT_CONFIG_SYSTEM should be '/dev/null'"
            );

            // Test GIT_CONFIG_NOSYSTEM
            let result = server
                .shell(
                    Parameters(ShellParams {
                        command: "printenv GIT_CONFIG_NOSYSTEM".to_string(),
                    }),
                    RequestContext {
                        ct: Default::default(),
                        id: NumberOrString::Number(6),
                        meta: Default::default(),
                        extensions: Default::default(),
                        peer: peer.clone(),
                    },
                )
                .await;

            assert!(result.is_ok(), "GIT_CONFIG_NOSYSTEM check should succeed");
            let result_content = result.unwrap();
            let output = result_content
                .content
                .iter()
                .find_map(|c| c.as_text())
                .expect("Should have text content");
            assert_eq!(output.text.trim(), "1", "GIT_CONFIG_NOSYSTEM should be '1'");

            // Test GIT_EDITOR contains our error message
            let result = server
                .shell(
                    Parameters(ShellParams {
                        command: "printenv GIT_EDITOR".to_string(),
                    }),
                    RequestContext {
                        ct: Default::default(),
                        id: NumberOrString::Number(7),
                        meta: Default::default(),
                        extensions: Default::default(),
                        peer: peer.clone(),
                    },
                )
                .await;

            assert!(result.is_ok(), "GIT_EDITOR check should succeed");
            let result_content = result.unwrap();
            let output = result_content
                .content
                .iter()
                .find_map(|c| c.as_text())
                .expect("Should have text content");
            assert!(
                output
                    .text
                    .contains("Interactive Git commands are not supported"),
                "GIT_EDITOR should contain error message, got: '{}'",
                output.text
            );

            // Test GIT_SEQUENCE_EDITOR contains our error message
            let result = server
                .shell(
                    Parameters(ShellParams {
                        command: "printenv GIT_SEQUENCE_EDITOR".to_string(),
                    }),
                    RequestContext {
                        ct: Default::default(),
                        id: NumberOrString::Number(8),
                        meta: Default::default(),
                        extensions: Default::default(),
                        peer: peer.clone(),
                    },
                )
                .await;

            assert!(result.is_ok(), "GIT_SEQUENCE_EDITOR check should succeed");
            let result_content = result.unwrap();
            let output = result_content
                .content
                .iter()
                .find_map(|c| c.as_text())
                .expect("Should have text content");
            assert!(
                output
                    .text
                    .contains("Interactive Git commands are not supported"),
                "GIT_SEQUENCE_EDITOR should contain error message, got: '{}'",
                output.text
            );

            // Test EDITOR contains our error message
            let result = server
                .shell(
                    Parameters(ShellParams {
                        command: "printenv EDITOR".to_string(),
                    }),
                    RequestContext {
                        ct: Default::default(),
                        id: NumberOrString::Number(9),
                        meta: Default::default(),
                        extensions: Default::default(),
                        peer: peer.clone(),
                    },
                )
                .await;

            assert!(result.is_ok(), "EDITOR check should succeed");
            let result_content = result.unwrap();
            let output = result_content
                .content
                .iter()
                .find_map(|c| c.as_text())
                .expect("Should have text content");
            assert!(
                output
                    .text
                    .contains("Interactive commands are not supported"),
                "EDITOR should contain error message, got: '{}'",
                output.text
            );

            // Test VISUAL contains our error message
            let result = server
                .shell(
                    Parameters(ShellParams {
                        command: "printenv VISUAL".to_string(),
                    }),
                    RequestContext {
                        ct: Default::default(),
                        id: NumberOrString::Number(10),
                        meta: Default::default(),
                        extensions: Default::default(),
                        peer: peer.clone(),
                    },
                )
                .await;

            assert!(result.is_ok(), "VISUAL check should succeed");
            let result_content = result.unwrap();
            let output = result_content
                .content
                .iter()
                .find_map(|c| c.as_text())
                .expect("Should have text content");
            assert!(
                output
                    .text
                    .contains("Interactive commands are not supported"),
                "VISUAL should contain error message, got: '{}'",
                output.text
            );

            cleanup_test_service(running_service, peer);
        });
    }

    #[test]
    #[serial]
    fn test_shell_git_config_values() {
        run_shell_test(|| async {
            let temp_dir = TempDir::new().unwrap();
            std::env::set_current_dir(&temp_dir).unwrap();

            let server = create_test_server();
            let running_service = serve_directly(server.clone(), create_test_transport(), None);
            let peer = running_service.peer().clone();

            // Test GIT_CONFIG_COUNT is set (only in test mode)
            #[cfg(test)]
            {
                let result = server
                    .shell(
                        Parameters(ShellParams {
                            command: "printenv GIT_CONFIG_COUNT".to_string(),
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

                assert!(result.is_ok(), "GIT_CONFIG_COUNT check should succeed");
                let result_content = result.unwrap();
                let output = result_content
                    .content
                    .iter()
                    .find_map(|c| c.as_text())
                    .expect("Should have text content");
                assert_eq!(output.text.trim(), "3", "GIT_CONFIG_COUNT should be '3'");
            }

            // Test GIT_CONFIG_KEY_0 is set (only in test mode)
            #[cfg(test)]
            {
                let result = server
                    .shell(
                        Parameters(ShellParams {
                            command: "printenv GIT_CONFIG_KEY_0".to_string(),
                        }),
                        RequestContext {
                            ct: Default::default(),
                            id: NumberOrString::Number(2),
                            meta: Default::default(),
                            extensions: Default::default(),
                            peer: peer.clone(),
                        },
                    )
                    .await;

                assert!(result.is_ok(), "GIT_CONFIG_KEY_0 check should succeed");
                let result_content = result.unwrap();
                let output = result_content
                    .content
                    .iter()
                    .find_map(|c| c.as_text())
                    .expect("Should have text content");
                assert_eq!(
                    output.text.trim(),
                    "commit.gpgsign",
                    "GIT_CONFIG_KEY_0 should be 'commit.gpgsign'"
                );
            }

            // Test GIT_CONFIG_VALUE_0 is set (only in test mode)
            #[cfg(test)]
            {
                let result = server
                    .shell(
                        Parameters(ShellParams {
                            command: "printenv GIT_CONFIG_VALUE_0".to_string(),
                        }),
                        RequestContext {
                            ct: Default::default(),
                            id: NumberOrString::Number(3),
                            meta: Default::default(),
                            extensions: Default::default(),
                            peer: peer.clone(),
                        },
                    )
                    .await;

                assert!(result.is_ok(), "GIT_CONFIG_VALUE_0 check should succeed");
                let result_content = result.unwrap();
                let output = result_content
                    .content
                    .iter()
                    .find_map(|c| c.as_text())
                    .expect("Should have text content");
                assert_eq!(
                    output.text.trim(),
                    "false",
                    "GIT_CONFIG_VALUE_0 should be 'false'"
                );
            }

            // Test GIT_CONFIG_KEY_1 is set (only in test mode)
            #[cfg(test)]
            {
                let result = server
                    .shell(
                        Parameters(ShellParams {
                            command: "printenv GIT_CONFIG_KEY_1".to_string(),
                        }),
                        RequestContext {
                            ct: Default::default(),
                            id: NumberOrString::Number(4),
                            meta: Default::default(),
                            extensions: Default::default(),
                            peer: peer.clone(),
                        },
                    )
                    .await;

                assert!(result.is_ok(), "GIT_CONFIG_KEY_1 check should succeed");
                let result_content = result.unwrap();
                let output = result_content
                    .content
                    .iter()
                    .find_map(|c| c.as_text())
                    .expect("Should have text content");
                assert_eq!(
                    output.text.trim(),
                    "tag.gpgsign",
                    "GIT_CONFIG_KEY_1 should be 'tag.gpgsign'"
                );
            }

            // Test GIT_CONFIG_VALUE_1 is set (only in test mode)
            #[cfg(test)]
            {
                let result = server
                    .shell(
                        Parameters(ShellParams {
                            command: "printenv GIT_CONFIG_VALUE_1".to_string(),
                        }),
                        RequestContext {
                            ct: Default::default(),
                            id: NumberOrString::Number(5),
                            meta: Default::default(),
                            extensions: Default::default(),
                            peer: peer.clone(),
                        },
                    )
                    .await;

                assert!(result.is_ok(), "GIT_CONFIG_VALUE_1 check should succeed");
                let result_content = result.unwrap();
                let output = result_content
                    .content
                    .iter()
                    .find_map(|c| c.as_text())
                    .expect("Should have text content");
                assert_eq!(
                    output.text.trim(),
                    "false",
                    "GIT_CONFIG_VALUE_1 should be 'false'"
                );
            }

            // Test GIT_CONFIG_KEY_2 is set (only in test mode)
            #[cfg(test)]
            {
                let result = server
                    .shell(
                        Parameters(ShellParams {
                            command: "printenv GIT_CONFIG_KEY_2".to_string(),
                        }),
                        RequestContext {
                            ct: Default::default(),
                            id: NumberOrString::Number(6),
                            meta: Default::default(),
                            extensions: Default::default(),
                            peer: peer.clone(),
                        },
                    )
                    .await;

                assert!(result.is_ok(), "GIT_CONFIG_KEY_2 check should succeed");
                let result_content = result.unwrap();
                let output = result_content
                    .content
                    .iter()
                    .find_map(|c| c.as_text())
                    .expect("Should have text content");
                assert_eq!(
                    output.text.trim(),
                    "user.signingkey",
                    "GIT_CONFIG_KEY_2 should be 'user.signingkey'"
                );
            }

            // Test GIT_CONFIG_VALUE_2 is set (only in test mode)
            #[cfg(test)]
            {
                let result = server
                    .shell(
                        Parameters(ShellParams {
                            command: "printenv GIT_CONFIG_VALUE_2".to_string(),
                        }),
                        RequestContext {
                            ct: Default::default(),
                            id: NumberOrString::Number(7),
                            meta: Default::default(),
                            extensions: Default::default(),
                            peer: peer.clone(),
                        },
                    )
                    .await;

                assert!(result.is_ok(), "GIT_CONFIG_VALUE_2 check should succeed");
                let result_content = result.unwrap();
                let output = result_content
                    .content
                    .iter()
                    .find_map(|c| c.as_text())
                    .expect("Should have text content");
                assert_eq!(
                    output.text.trim(),
                    "",
                    "GIT_CONFIG_VALUE_2 should be empty string"
                );
            }

            // Test commit.gpgsign is disabled (only in test mode)
            #[cfg(test)]
            {
                let result = server
                    .shell(
                        Parameters(ShellParams {
                            command: "git config --get commit.gpgsign".to_string(),
                        }),
                        RequestContext {
                            ct: Default::default(),
                            id: NumberOrString::Number(8),
                            meta: Default::default(),
                            extensions: Default::default(),
                            peer: peer.clone(),
                        },
                    )
                    .await;

                assert!(
                    result.is_ok(),
                    "git config commit.gpgsign check should succeed"
                );
                let result_content = result.unwrap();
                let output = result_content
                    .content
                    .iter()
                    .find_map(|c| c.as_text())
                    .expect("Should have text content");
                assert_eq!(
                    output.text.trim(),
                    "false",
                    "commit.gpgsign should be 'false'"
                );
            }

            // Test tag.gpgsign is disabled (only in test mode)
            #[cfg(test)]
            {
                let result = server
                    .shell(
                        Parameters(ShellParams {
                            command: "git config --get tag.gpgsign".to_string(),
                        }),
                        RequestContext {
                            ct: Default::default(),
                            id: NumberOrString::Number(9),
                            meta: Default::default(),
                            extensions: Default::default(),
                            peer: peer.clone(),
                        },
                    )
                    .await;

                assert!(
                    result.is_ok(),
                    "git config tag.gpgsign check should succeed"
                );
                let result_content = result.unwrap();
                let output = result_content
                    .content
                    .iter()
                    .find_map(|c| c.as_text())
                    .expect("Should have text content");
                assert_eq!(output.text.trim(), "false", "tag.gpgsign should be 'false'");
            }

            cleanup_test_service(running_service, peer);
        });
    }
}
