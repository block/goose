#![recursion_limit = "256"]

#[allow(dead_code)]
#[path = "acp_common_tests/mod.rs"]
mod common_tests;
use common_tests::fixtures::provider::AcpProviderConnection;
use common_tests::fixtures::{nested_terminal_agent, run_test};
#[cfg(feature = "code-mode")]
use common_tests::run_prompt_codemode;
use common_tests::{
    run_close_session, run_config_mcp, run_delete_session, run_fs_read_text_file_true,
    run_fs_write_text_file_false, run_fs_write_text_file_true, run_load_mode, run_load_model,
    run_load_session_error, run_load_session_mcp, run_model_list, run_permission_persistence,
    run_prompt_basic, run_prompt_error, run_prompt_image, run_prompt_image_attachment,
    run_prompt_mcp, run_prompt_model_mismatch, run_prompt_skill, run_shell_terminal_false,
    run_shell_terminal_true,
};
use futures::StreamExt;
use goose::acp::{AcpProvider, AcpProviderConfig};
use goose::config::GooseMode;
use goose::conversation::message::{Message, MessageContent};
use goose::providers::base::Provider;
use goose_providers::model::ModelConfig;
use std::collections::HashMap;

#[test]
fn test_nested_terminal_notifications_follow_tool_call_start() {
    run_test(async {
        let config = AcpProviderConfig {
            command: "unused".into(),
            args: Vec::new(),
            env: Vec::new(),
            env_remove: Vec::new(),
            work_dir: std::path::PathBuf::new(),
            mcp_servers: Vec::new(),
            session_mode_id: None,
            session_config_options: Vec::new(),
            model_config_option_id: None,
            mode_mapping: HashMap::new(),
            notification_callback: None,
        };
        let provider = AcpProvider::connect_with_transport(
            "nested-acp".to_string(),
            GooseMode::Auto,
            config,
            agent_client_protocol::DynConnectTo::new(nested_terminal_agent()),
        )
        .await
        .unwrap();
        let (_session_id, mut terminal_notifications) = provider
            .take_acp_notification_receiver()
            .expect("ACP provider should expose terminal notifications");
        let mut stream = provider
            .stream(
                &ModelConfig::new("nested-model"),
                "",
                &[Message::user().with_text("run a foreground command")],
                &[],
            )
            .await
            .unwrap();

        let first = stream
            .next()
            .await
            .expect("expected a tool-call start")
            .unwrap()
            .0
            .expect("expected a message");
        assert!(matches!(
            first.content.as_slice(),
            [MessageContent::ToolRequest(_)]
        ));
        assert!(terminal_notifications.try_recv().is_err());

        let completion = stream.next();
        tokio::pin!(completion);
        let terminal_delta = tokio::select! {
            biased;
            update = terminal_notifications.recv() => update.expect("expected terminal output"),
            _ = &mut completion => panic!("tool call completed before terminal output"),
        };
        assert!(terminal_delta["update"]["_meta"]
            .get("terminal_output_delta")
            .is_some());
        let terminal_exit = tokio::select! {
            biased;
            update = terminal_notifications.recv() => update.expect("expected terminal exit"),
            _ = &mut completion => panic!("tool call completed before terminal exit"),
        };
        assert!(terminal_exit["update"]["_meta"]
            .get("terminal_exit")
            .is_some());

        let completion = completion
            .await
            .expect("expected a tool-call completion")
            .unwrap()
            .0
            .expect("expected a message");
        assert!(matches!(
            completion.content.as_slice(),
            [MessageContent::ToolResponse(_)]
        ));
    });
}

#[test]
fn test_config_mcp() {
    run_test(async { run_config_mcp::<AcpProviderConnection>().await });
}

#[test]
fn test_close_session() {
    run_test(async { run_close_session::<AcpProviderConnection>().await });
}

#[test]
#[ignore = "delete is a server-side custom method not routed through the provider"]
fn test_delete_session() {
    run_test(async { run_delete_session::<AcpProviderConnection>().await });
}

#[test]
#[ignore = "provider is a plug-in to the goose CLI, UI and terminal clients, none of which handle buffered changes to files"]
fn test_fs_read_text_file_true() {
    run_test(async { run_fs_read_text_file_true::<AcpProviderConnection>().await });
}

#[test]
fn test_fs_write_text_file_false() {
    run_test(async { run_fs_write_text_file_false::<AcpProviderConnection>().await });
}

#[test]
#[ignore = "provider is a plug-in to the goose CLI, UI and terminal clients, none of which handle buffered changes to files"]
fn test_fs_write_text_file_true() {
    run_test(async { run_fs_write_text_file_true::<AcpProviderConnection>().await });
}

#[test]
#[ignore = "TODO: implement load_session in ACP provider"]
fn test_load_mode() {
    run_test(async { run_load_mode::<AcpProviderConnection>().await });
}

#[test]
#[ignore = "TODO: implement load_session in ACP provider"]
fn test_load_model() {
    run_test(async { run_load_model::<AcpProviderConnection>().await });
}

#[test]
#[ignore = "TODO: implement load_session in ACP provider"]
fn test_load_session_error_session_not_found() {
    run_test(async { run_load_session_error::<AcpProviderConnection>().await });
}

#[test]
#[ignore = "TODO: implement load_session in ACP provider"]
fn test_load_session_mcp() {
    run_test(async { run_load_session_mcp::<AcpProviderConnection>().await });
}

#[test]
fn test_model_list() {
    run_test(async { run_model_list::<AcpProviderConnection>().await });
}

#[test]
fn test_permission_persistence() {
    run_test(async { run_permission_persistence::<AcpProviderConnection>().await });
}

#[test]
fn test_prompt_basic() {
    run_test(async { run_prompt_basic::<AcpProviderConnection>().await });
}

#[test]
#[cfg(feature = "code-mode")]
fn test_prompt_codemode() {
    run_test(async { run_prompt_codemode::<AcpProviderConnection>().await });
}

#[test]
#[ignore = "ensure_session lazy-creates sessions so deleted ones reappear"]
fn test_prompt_error_session_not_found() {
    run_test(async { run_prompt_error::<AcpProviderConnection>().await });
}

#[test]
fn test_prompt_image() {
    run_test(async { run_prompt_image::<AcpProviderConnection>().await });
}

#[test]
fn test_prompt_image_attachment() {
    run_test(async { run_prompt_image_attachment::<AcpProviderConnection>().await });
}

#[test]
fn test_prompt_mcp() {
    run_test(async { run_prompt_mcp::<AcpProviderConnection>().await });
}

#[test]
fn test_prompt_model_mismatch() {
    run_test(async { run_prompt_model_mismatch::<AcpProviderConnection>().await });
}

#[test]
fn test_prompt_skill() {
    run_test(async { run_prompt_skill::<AcpProviderConnection>().await });
}

#[test]
fn test_shell_terminal_false() {
    run_test(async { run_shell_terminal_false::<AcpProviderConnection>().await });
}

#[test]
#[ignore = "provider does not handle terminal delegation requests"]
fn test_shell_terminal_true() {
    run_test(async { run_shell_terminal_true::<AcpProviderConnection>().await });
}
