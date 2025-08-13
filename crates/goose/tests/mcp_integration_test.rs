use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::{env, fs};

use rmcp::model::Content;
use serde_json::json;
use tokio_util::sync::CancellationToken;

use goose::agents::extension::{Envs, ExtensionConfig};
use goose::agents::extension_manager::ExtensionManager;
use mcp_core::ToolCall;

use test_case::test_case;

enum TestMode {
    Record,
    Playback,
}

#[test_case(
    vec!["npx", "-y", "@modelcontextprotocol/server-everything"],
    vec![
        ToolCall::new("echo", json!({"message": "Hello, world!"})),
        ToolCall::new("add", json!({"a": 1, "b": 2})),
        ToolCall::new("longRunningOperation", json!({"duration": 1, "steps": 5})),
        ToolCall::new("structuredContent", json!({"location": "11238"})),
    ],
    vec![]
)]
#[test_case(
    vec!["github-mcp-server", "stdio"],
    vec![
        ToolCall::new("get_file_contents", json!({
            "owner": "block",
            "repo": "goose",
            "path": "README.md",
            "sha": "48c1ec8afdb7d4d5b4f6e67e623926c884034776"
        })),
    ],
    vec!["GITHUB_PERSONAL_ACCESS_TOKEN"]
)]
#[test_case(
    vec!["uvx", "mcp-server-fetch"],
    vec![
        ToolCall::new("fetch", json!({
            "url": "https://example.com",
        })),
    ],
    vec![]
)]
#[tokio::test]
async fn test_replayed_session(
    command: Vec<&str>,
    tool_calls: Vec<ToolCall>,
    required_envs: Vec<&str>,
) {
    let replay_file_name = command
        .iter()
        .map(|s| s.replace("/", "_"))
        .collect::<Vec<String>>()
        .join("");
    let mut replay_file_path =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("should find the project root"));
    replay_file_path.push("tests");
    replay_file_path.push("mcp_replays");
    replay_file_path.push(&replay_file_name);

    let mode = if env::var("GOOSE_RECORD_MCP").is_ok() {
        TestMode::Record
    } else {
        assert!(replay_file_path.exists(), "replay file doesn't exist");
        TestMode::Playback
    };

    let mode_arg = match mode {
        TestMode::Record => "record",
        TestMode::Playback => "playback",
    };
    let cmd = "cargo".to_string();
    let mut args = vec![
        "run",
        "--quiet",
        "-p",
        "goose-test",
        "--bin",
        "capture",
        "--",
        "stdio",
        mode_arg,
    ]
    .into_iter()
    .map(str::to_string)
    .collect::<Vec<String>>();

    args.push(replay_file_path.to_string_lossy().to_string());

    let mut env = HashMap::new();

    if matches!(mode, TestMode::Record) {
        args.extend(command.into_iter().map(str::to_string));

        for key in required_envs {
            match env::var(key) {
                Ok(v) => {
                    env.insert(key.to_string(), v);
                }
                Err(_) => {
                    eprintln!("skipping due to missing required env variable: {}", key);
                    return;
                }
            }
        }
    }

    let envs = Envs::new(env);
    let extension_config = ExtensionConfig::Stdio {
        name: "test".to_string(),
        description: Some("Test".to_string()),
        cmd,
        args,
        envs,
        env_keys: vec![],
        timeout: Some(30),
        bundled: Some(false),
    };

    let mut extension_manager = ExtensionManager::new();

    let result = extension_manager.add_extension(extension_config).await;
    assert!(result.is_ok(), "Failed to add extension: {:?}", result);

    let result = (async || -> Result<(), Box<dyn std::error::Error>> {
        let mut results = Vec::new();
        for tool_call in tool_calls {
            let tool_call = ToolCall::new(format!("test__{}", tool_call.name), tool_call.arguments);
            let result = extension_manager
                .dispatch_tool_call(tool_call, CancellationToken::default())
                .await;

            let tool_result = result?;
            results.push(tool_result.result.await?);
        }

        let mut results_path = replay_file_path.clone();
        results_path.pop();
        results_path.push(format!("{}.results.json", &replay_file_name));

        match mode {
            TestMode::Record => {
                serde_json::to_writer_pretty(File::create(results_path)?, &results)?
            }
            TestMode::Playback => assert_eq!(
                serde_json::from_reader::<_, Vec<Vec<Content>>>(File::open(results_path)?)?,
                results
            ),
        };

        Ok(())
    })()
    .await;

    if let Err(err) = result {
        let errors =
            fs::read_to_string(format!("{}.errors.txt", replay_file_path.to_string_lossy()))
                .expect("could not read errors");
        eprintln!("errors from {}", replay_file_path.to_string_lossy());
        eprintln!("{}", errors);
        eprintln!();
        panic!("Test failed: {:?}", err);
    }
}
