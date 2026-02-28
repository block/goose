mod common_tests;
use common_tests::fixtures::run_test;
use common_tests::fixtures::server::ClientToAgentConnection;
use common_tests::{
    run_agent_side_write_rejected_when_fs_disabled, run_agent_side_write_when_fs_disabled,
    run_config_mcp, run_fs_read_delegation_without_permission,
    run_fs_write_delegation_with_permission_allowed,
    run_fs_write_delegation_with_permission_rejected, run_initialize_without_provider,
    run_load_model, run_model_list, run_model_set, run_permission_persistence, run_prompt_basic,
    run_prompt_codemode, run_prompt_image, run_prompt_mcp,
};

#[test]
fn test_config_mcp() {
    run_test(async { run_config_mcp::<ClientToAgentConnection>().await });
}

#[test]
fn test_initialize_without_provider() {
    run_test(async { run_initialize_without_provider().await });
}

#[test]
fn test_load_model() {
    run_test(async { run_load_model::<ClientToAgentConnection>().await });
}

#[test]
fn test_model_list() {
    run_test(async { run_model_list::<ClientToAgentConnection>().await });
}

#[test]
fn test_model_set() {
    run_test(async { run_model_set::<ClientToAgentConnection>().await });
}

#[test]
fn test_permission_persistence() {
    run_test(async { run_permission_persistence::<ClientToAgentConnection>().await });
}

#[test]
fn test_prompt_basic() {
    run_test(async { run_prompt_basic::<ClientToAgentConnection>().await });
}

#[test]
fn test_prompt_codemode() {
    run_test(async { run_prompt_codemode::<ClientToAgentConnection>().await });
}

#[test]
fn test_prompt_image() {
    run_test(async { run_prompt_image::<ClientToAgentConnection>().await });
}

#[test]
fn test_prompt_mcp() {
    run_test(async { run_prompt_mcp::<ClientToAgentConnection>().await });
}

#[test]
fn test_fs_read_delegation_without_permission() {
    run_test(async { run_fs_read_delegation_without_permission().await });
}

#[test]
fn test_fs_write_delegation_with_permission_allowed() {
    run_test(async { run_fs_write_delegation_with_permission_allowed().await });
}

#[test]
fn test_fs_write_delegation_with_permission_rejected() {
    run_test(async { run_fs_write_delegation_with_permission_rejected().await });
}

#[test]
fn test_agent_side_write_when_fs_disabled() {
    run_test(async { run_agent_side_write_when_fs_disabled().await });
}

#[test]
fn test_agent_side_write_rejected_when_fs_disabled() {
    run_test(async { run_agent_side_write_rejected_when_fs_disabled().await });
}
