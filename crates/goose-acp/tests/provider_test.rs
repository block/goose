#![recursion_limit = "256"]

mod common_tests;
use common_tests::fixtures::provider::ClientToProviderConnection;
use common_tests::fixtures::run_test;
use common_tests::{
    run_config_mcp, run_fs_read_text_file_true, run_fs_write_text_file_false,
    run_fs_write_text_file_true, run_initialize_doesnt_hit_provider, run_load_mode, run_load_model,
    run_load_session_mcp, run_mode_set, run_model_list, run_model_set, run_permission_persistence,
    run_prompt_basic, run_prompt_codemode, run_prompt_image, run_prompt_image_attachment,
    run_prompt_mcp, run_prompt_skill,
};

tests_mode_set_error!(ClientToProviderConnection);

#[test]
fn test_config_mcp() {
    run_test(async { run_config_mcp::<ClientToProviderConnection>().await });
}

#[test]
#[ignore = "provider is a plug-in to the goose CLI, UI and terminal clients, none of which handle buffered changes to files"]
fn test_fs_read_text_file_true() {
    run_test(async { run_fs_read_text_file_true::<ClientToProviderConnection>().await });
}

#[test]
fn test_fs_write_text_file_false() {
    run_test(async { run_fs_write_text_file_false::<ClientToProviderConnection>().await });
}

#[test]
#[ignore = "provider is a plug-in to the goose CLI, UI and terminal clients, none of which handle buffered changes to files"]
fn test_fs_write_text_file_true() {
    run_test(async { run_fs_write_text_file_true::<ClientToProviderConnection>().await });
}

#[test]
fn test_initialize_doesnt_hit_provider() {
    run_test(async { run_initialize_doesnt_hit_provider::<ClientToProviderConnection>().await });
}

#[test]
#[ignore = "TODO: implement load_session in ACP provider"]
fn test_load_mode() {
    run_test(async { run_load_mode::<ClientToProviderConnection>().await });
}

#[test]
#[ignore = "TODO: implement load_session in ACP provider"]
fn test_load_model() {
    run_test(async { run_load_model::<ClientToProviderConnection>().await });
}

#[test]
#[ignore = "TODO: implement load_session in ACP provider"]
fn test_load_session_mcp() {
    run_test(async { run_load_session_mcp::<ClientToProviderConnection>().await });
}

#[test]
#[ignore = "TODO: on_set_mode is a no-op until mode is threaded per-session (#7603)"]
fn test_mode_set() {
    run_test(async { run_mode_set::<ClientToProviderConnection>().await });
}

#[test]
fn test_model_list() {
    run_test(async { run_model_list::<ClientToProviderConnection>().await });
}

#[test]
fn test_model_set() {
    run_test(async { run_model_set::<ClientToProviderConnection>().await });
}

#[test]
fn test_permission_persistence() {
    run_test(async { run_permission_persistence::<ClientToProviderConnection>().await });
}

#[test]
fn test_prompt_basic() {
    run_test(async { run_prompt_basic::<ClientToProviderConnection>().await });
}

#[test]
fn test_prompt_codemode() {
    run_test(async { run_prompt_codemode::<ClientToProviderConnection>().await });
}

#[test]
fn test_prompt_image() {
    run_test(async { run_prompt_image::<ClientToProviderConnection>().await });
}

#[test]
fn test_prompt_image_attachment() {
    run_test(async { run_prompt_image_attachment::<ClientToProviderConnection>().await });
}

#[test]
fn test_prompt_mcp() {
    run_test(async { run_prompt_mcp::<ClientToProviderConnection>().await });
}

#[test]
fn test_prompt_skill() {
    run_test(async { run_prompt_skill::<ClientToProviderConnection>().await });
}
