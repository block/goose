#![recursion_limit = "256"]

mod common_tests;
use common_tests::fixtures::provider::ClientToProviderConnection;
use common_tests::fixtures::run_test;
use common_tests::{
    run_config_mcp, run_fs_read_text_file_false, run_fs_read_text_file_true,
    run_fs_write_text_file_false, run_fs_write_text_file_true, run_initialize_doesnt_hit_provider,
    run_load_model, run_model_list, run_model_set, run_permission_persistence, run_prompt_basic,
    run_prompt_codemode, run_prompt_image, run_prompt_image_attachment, run_prompt_mcp,
};

#[test]
fn test_provider_config_mcp() {
    run_test(async { run_config_mcp::<ClientToProviderConnection>().await });
}

#[test]
fn test_provider_fs_read_text_file_false() {
    run_test(async { run_fs_read_text_file_false::<ClientToProviderConnection>().await });
}

#[test]
#[ignore = "TODO: AcpProvider does not yet send ClientCapabilities with FileSystemCapability"]
fn test_provider_fs_read_text_file_true() {
    run_test(async { run_fs_read_text_file_true::<ClientToProviderConnection>().await });
}

#[test]
fn test_provider_fs_write_text_file_false() {
    run_test(async { run_fs_write_text_file_false::<ClientToProviderConnection>().await });
}

#[test]
#[ignore = "TODO: AcpProvider does not yet send ClientCapabilities with FileSystemCapability"]
fn test_provider_fs_write_text_file_true() {
    run_test(async { run_fs_write_text_file_true::<ClientToProviderConnection>().await });
}

#[test]
fn test_provider_initialize_doesnt_hit_provider() {
    run_test(async { run_initialize_doesnt_hit_provider::<ClientToProviderConnection>().await });
}

#[test]
#[ignore = "TODO: implement load_session in ACP provider"]
fn test_provider_load_model() {
    run_test(async { run_load_model::<ClientToProviderConnection>().await });
}

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
fn test_provider_prompt_image_attachment() {
    run_test(async { run_prompt_image_attachment::<ClientToProviderConnection>().await });
}

#[test]
fn test_provider_prompt_mcp() {
    run_test(async { run_prompt_mcp::<ClientToProviderConnection>().await });
}
