mod common_tests;
use common_tests::fixtures::run_test;
use common_tests::fixtures::server::ClientToAgentSession;
use common_tests::{
    run_config_mcp, run_permission_persistence, run_prompt_basic, run_prompt_codemode,
    run_prompt_image, run_prompt_mcp,
};

#[test]
fn test_config_mcp() {
    run_test(async { run_config_mcp::<ClientToAgentSession>().await });
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
