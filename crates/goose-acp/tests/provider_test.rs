#![recursion_limit = "256"]

mod common_tests;
use common_tests::fixtures::provider::AcpProviderConnection;
use common_tests::fixtures::run_test;
use common_tests::{
    run_close_session, run_config_mcp, run_config_option_mode_set, run_config_option_model_set,
    run_delete_session, run_fs_read_text_file_true, run_fs_write_text_file_false,
    run_fs_write_text_file_true, run_initialize_doesnt_hit_provider, run_list_sessions,
    run_load_mode, run_load_model, run_load_session_mcp, run_mode_set, run_model_list,
    run_model_set, run_permission_persistence, run_prompt_basic, run_prompt_codemode,
    run_prompt_image, run_prompt_image_attachment, run_prompt_mcp, run_prompt_skill,
    run_shell_terminal_false, run_shell_terminal_true,
};

tests_load_session_error!(AcpProviderConnection);
tests_prompt_error!(AcpProviderConnection);
tests_config_option_set_error!(AcpProviderConnection);
tests_mode_set_error!(AcpProviderConnection);
tests_model_set_error!(AcpProviderConnection);

#[test]
fn test_config_mcp() {
    run_test(async { run_config_mcp::<AcpProviderConnection>().await });
}

#[test]
fn test_config_option_mode_set() {
    run_test(async { run_config_option_mode_set::<AcpProviderConnection>().await });
}

#[test]
fn test_list_sessions() {
    run_test(async { run_list_sessions::<AcpProviderConnection>().await });
}

#[test]
fn test_close_session() {
    run_test(async { run_close_session::<AcpProviderConnection>().await });
}

#[test]
fn test_config_option_model_set() {
    run_test(async { run_config_option_model_set::<AcpProviderConnection>().await });
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
fn test_initialize_doesnt_hit_provider() {
    run_test(async { run_initialize_doesnt_hit_provider::<AcpProviderConnection>().await });
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
fn test_load_session_mcp() {
    run_test(async { run_load_session_mcp::<AcpProviderConnection>().await });
}

#[test]
fn test_mode_set() {
    run_test(async { run_mode_set::<AcpProviderConnection>().await });
}

#[test]
fn test_model_list() {
    run_test(async { run_model_list::<AcpProviderConnection>().await });
}

#[test]
fn test_model_set() {
    run_test(async { run_model_set::<AcpProviderConnection>().await });
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
fn test_prompt_codemode() {
    run_test(async { run_prompt_codemode::<AcpProviderConnection>().await });
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
