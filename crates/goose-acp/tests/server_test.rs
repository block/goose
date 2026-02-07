mod common_tests;
use common_tests::fixtures::initialize_agent;
use common_tests::fixtures::run_test;
use common_tests::fixtures::server::ClientToAgentSession;
use common_tests::{
    run_basic_completion, run_builtin_and_mcp, run_configured_extension, run_mcp_http_server,
    run_permission_persistence,
};
use goose::config::GooseMode;
use goose::providers::provider_registry::ProviderConstructor;
use goose_acp::server::GooseAcpAgent;
use std::sync::Arc;

#[test]
fn test_acp_basic_completion() {
    run_test(async { run_basic_completion::<ClientToAgentSession>().await });
}

#[test]
fn test_acp_with_mcp_http_server() {
    run_test(async { run_mcp_http_server::<ClientToAgentSession>().await });
}

#[test]
fn test_acp_with_builtin_and_mcp() {
    run_test(async { run_builtin_and_mcp::<ClientToAgentSession>().await });
}

#[test]
fn test_permission_persistence() {
    run_test(async { run_permission_persistence::<ClientToAgentSession>().await });
}

#[test]
fn test_configured_extension() {
    run_test(async { run_configured_extension::<ClientToAgentSession>().await });
}

#[test]
fn test_initialize_without_provider() {
    run_test(async {
        let temp_dir = tempfile::tempdir().unwrap();

        let provider_factory: ProviderConstructor =
            Arc::new(|_| Box::pin(async { Err(anyhow::anyhow!("no provider configured")) }));

        let agent = Arc::new(
            GooseAcpAgent::new(
                provider_factory,
                vec![],
                temp_dir.path().to_path_buf(),
                temp_dir.path().to_path_buf(),
                GooseMode::Auto,
                false,
            )
            .await
            .unwrap(),
        );

        // Initialization shouldn't fail even though we have a crashing provider factory.
        let resp = initialize_agent(agent).await;
        assert!(!resp.auth_methods.is_empty());
        assert!(resp
            .auth_methods
            .iter()
            .any(|m| &*m.id.0 == "goose-provider"));
    });
}
