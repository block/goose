use std::env;
use std::path::PathBuf;

use serde_json::json;
use tokio_util::sync::CancellationToken;

use goose::agents::extension::{Envs, ExtensionConfig};
use goose::agents::extension_manager::ExtensionManager;
use mcp_core::ToolCall;

use test_case::test_case;

enum TestMode {
    Record,
    Replay,
}

const LOGGER_BINARY: &str = "stdio_logger";
const REPLAY_BINARY: &str = "stdio_replayer";

#[test_case(
    vec!["npx", "-y", "@modelcontextprotocol/server-everything"],
    vec![
        ToolCall::new("echo", json!({"message": "Hello, world!"})),
    ]
)]
#[tokio::test]
async fn test_replayed_session(command: Vec<&str>, tool_calls: Vec<ToolCall>) {
    let replay_file_name = command
        .iter()
        .map(|s| s.replace("/", "_"))
        .collect::<Vec<String>>()
        .join("");
    let mut replay_file_path =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("should find the project root"));
    replay_file_path.push("tests");
    replay_file_path.push("mcp_replays");
    replay_file_path.push(replay_file_name);

    let mode = if env::var("GOOSE_RECORD_MCP").is_ok() {
        TestMode::Record
    } else {
        assert!(replay_file_path.exists(), "replay file doesn't exist");
        TestMode::Replay
    };

    let bin = match mode {
        TestMode::Record => LOGGER_BINARY,
        TestMode::Replay => REPLAY_BINARY,
    };
    let cmd = "cargo".to_string();
    let mut args = vec!["run", "--quiet", "-p", "goose-test", "--bin", bin, "--"]
        .into_iter()
        .map(str::to_string)
        .collect::<Vec<String>>();

    args.push(replay_file_path.to_string_lossy().to_string());

    if matches!(mode, TestMode::Record) {
        args.extend(command.into_iter().map(str::to_string));
    }

    let extension_config = ExtensionConfig::Stdio {
        name: "test".to_string(),
        description: Some("Test".to_string()),
        cmd,
        args,
        envs: Envs::default(),
        env_keys: vec![],
        timeout: Some(30),
        bundled: Some(false),
    };

    let mut extension_manager = ExtensionManager::new();
    let result = extension_manager.add_extension(extension_config).await;
    assert!(result.is_ok(), "Failed to add extension: {:?}", result);

    for tool_call in tool_calls {
        let tool_call = ToolCall::new(format!("test__{}", tool_call.name), tool_call.arguments);
        let result = extension_manager
            .dispatch_tool_call(tool_call, CancellationToken::default())
            .await;

        let tool_result = result.expect("tool dispatch should succeed");
        tool_result.result.await.expect("should get a result");
    }
}
