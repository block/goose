mod common_tests;
use common_tests::fixtures::run_test;
use common_tests::fixtures::server::ClientToAgentSession;
use common_tests::{
    run_basic_completion, run_builtin_and_mcp, run_configured_extension, run_mcp_http_server,
    run_permission_persistence,
};

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
