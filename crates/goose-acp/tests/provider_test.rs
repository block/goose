#![recursion_limit = "256"]

mod common_tests;
use common_tests::fixtures::provider::ClientToProviderConnection;
use common_tests::fixtures::run_test;
use common_tests::{
    run_config_mcp, run_initialize_doesnt_hit_provider, run_model_list, run_model_set,
    run_permission_persistence, run_prompt_basic, run_prompt_codemode, run_prompt_image,
    run_prompt_mcp,
};

#[test]
fn test_provider_config_mcp() {
    run_test(async { run_config_mcp::<ClientToProviderConnection>().await });
}

#[test]
fn test_provider_initialize_doesnt_hit_provider() {
    run_test(async { run_initialize_doesnt_hit_provider::<ClientToProviderConnection>().await });
}

// TODO: run_load_model requires ACP-level session persistence (load_session)

#[test]
fn test_provider_model_list() {
    run_test(async { run_model_list::<ClientToProviderConnection>().await });
}

#[test]
fn test_provider_model_set() {
    run_test(async { run_model_set::<ClientToProviderConnection>().await });
}

#[test]
fn test_provider_permission_persistence() {
    run_test(async { run_permission_persistence::<ClientToProviderConnection>().await });
}

#[test]
fn test_provider_prompt_basic() {
    run_test(async { run_prompt_basic::<ClientToProviderConnection>().await });
}

#[test]
fn test_provider_prompt_codemode() {
    run_test(async { run_prompt_codemode::<ClientToProviderConnection>().await });
}

#[test]
fn test_provider_prompt_image() {
    run_test(async { run_prompt_image::<ClientToProviderConnection>().await });
}

#[test]
fn test_provider_prompt_mcp() {
    run_test(async { run_prompt_mcp::<ClientToProviderConnection>().await });
}
