mod common_tests;
use common_tests::fixtures::initialize_agent;
use common_tests::fixtures::run_test;
use common_tests::fixtures::server::ClientToAgentSession;
use common_tests::{
    run_config_mcp, run_model_list, run_permission_persistence, run_prompt_basic,
    run_prompt_codemode, run_prompt_image, run_prompt_mcp, run_set_model,
};
use goose::config::GooseMode;
use goose::providers::provider_registry::ProviderConstructor;
use goose_acp::server::GooseAcpAgent;
use std::sync::Arc;

#[test]
fn test_config_mcp() {
    run_test(async { run_config_mcp::<ClientToAgentSession>().await });
}

#[test]
fn test_model_list() {
    run_test(async { run_model_list::<ClientToAgentSession>().await });
}

#[test]
fn test_set_model() {
    run_test(async { run_set_model::<ClientToAgentSession>().await });
}

#[test]
fn test_permission_persistence() {
    run_test(async { run_permission_persistence::<ClientToAgentSession>().await });
}

#[test]
fn test_prompt_basic() {
    run_test(async { run_prompt_basic::<ClientToAgentSession>().await });
}

#[test]
fn test_prompt_codemode() {
    run_test(async { run_prompt_codemode::<ClientToAgentSession>().await });
}

#[test]
fn test_prompt_image() {
    run_test(async { run_prompt_image::<ClientToAgentSession>().await });
}

#[test]
fn test_prompt_mcp() {
    run_test(async { run_prompt_mcp::<ClientToAgentSession>().await });
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
