use serde::Deserialize;
use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::{env, fs};

use rmcp::model::{CallToolRequestParam, Content};
use rmcp::object;
use tokio_util::sync::CancellationToken;

use goose::agents::extension::{Envs, ExtensionConfig};
use goose::agents::extension_manager::ExtensionManager;

use test_case::test_case;

use once_cell::sync::Lazy;
use std::process::Command;

#[derive(Deserialize)]
struct CargoBuildMessage {
    reason: String,
    target: Target,
    executable: String,
}

#[derive(Deserialize)]
struct Target {
    name: String,
    kind: Vec<String>,
}

fn build_and_get_binary_path() -> PathBuf {
    let output = Command::new("cargo")
        .args([
            "build",
            "--frozen",
            "-p",
            "goose-test",
            "--bin",
            "capture",
            "--message-format=json",
        ])
        .output()
        .expect("failed to build binary");

    if !output.status.success() {
        panic!("build failed: {}", String::from_utf8_lossy(&output.stderr));
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(serde_json::from_str::<CargoBuildMessage>)
        .filter_map(Result::ok)
        .filter(|message| message.reason == "compiler-artifact")
        .filter_map(|message| {
            if message.target.name == "capture"
                && message.target.kind.contains(&String::from("bin"))
            {
                Some(PathBuf::from(message.executable))
            } else {
                None
            }
        })
        .next()
        .expect("failed to parase binary path")
}

static REPLAY_BINARY_PATH: Lazy<PathBuf> = Lazy::new(build_and_get_binary_path);

enum TestMode {
    Record,
    Playback,
}

#[test_case(
    vec!["npx", "-y", "@modelcontextprotocol/server-everything"],
    vec![
        CallToolRequestParam { name: "echo".into(), arguments: Some(object!({"message": "Hello, world!" })) },
        CallToolRequestParam { name: "add".into(), arguments: Some(object!({"a": 1, "b": 2 })) },
        CallToolRequestParam { name: "longRunningOperation".into(), arguments: Some(object!({"duration": 1, "steps": 5 })) },
        CallToolRequestParam { name: "structuredContent".into(), arguments: Some(object!({"location": "11238"})) },
    ],
    vec![]
)]
#[test_case(
    vec!["github-mcp-server", "stdio"],
    vec![
        CallToolRequestParam { name: "get_file_contents".into(), arguments: Some(object!({
            "owner": "block",
            "repo": "goose",
            "path": "README.md",
            "sha": "ab62b863c1666232a67048b6c4e10007a2a5b83c"
        }))},
    ],
    vec!["GITHUB_PERSONAL_ACCESS_TOKEN"]
)]
#[test_case(
    vec!["uvx", "mcp-server-fetch"],
    vec![
        CallToolRequestParam { name: "fetch".into(), arguments: Some(object!({
            "url": "https://example.com",
        })) }
    ],
    vec![]
)]
#[test_case(
    vec!["cargo", "run", "--quiet", "-p", "goose-server", "--bin", "goosed", "--", "mcp", "developer"],
    vec![
        CallToolRequestParam { name: "text_editor".into(), arguments: Some(object!({
            "command": "view",
            "path": "/tmp/goose_test/goose.txt"
        }))},
        CallToolRequestParam { name: "text_editor".into(), arguments: Some(object!({
            "command": "str_replace",
            "path": "/tmp/goose_test/goose.txt",
            "old_str": "# goose",
            "new_str": "# goose (modified by test)"
        }))},
        // Test shell command to verify file was modified
        CallToolRequestParam { name: "shell".into(), arguments: Some(object!({
            "command": "cat /tmp/goose_test/goose.txt"
        })) },
        // Test text_editor tool to restore original content
        CallToolRequestParam { name: "text_editor".into(), arguments: Some(object!({
            "command": "str_replace",
            "path": "/tmp/goose_test/goose.txt",
            "old_str": "# goose (modified by test)",
            "new_str": "# goose"
        }))},
        CallToolRequestParam { name: "list_windows".into(), arguments: Some(object!({})) },
    ],
    vec![]
)]
#[tokio::test]
async fn test_replayed_session(
    command: Vec<&str>,
    tool_calls: Vec<CallToolRequestParam>,
    required_envs: Vec<&str>,
) {
    std::env::set_var("GOOSE_MCP_CLIENT_VERSION", "0.0.0");

    // Setup test file for developer extension tests
    let test_file_path = "/tmp/goose_test/goose.txt";
    if let Some(parent) = std::path::Path::new(test_file_path).parent() {
        fs::create_dir_all(parent).ok();
    }
    fs::write(test_file_path, "# goose\n").ok();
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
    let cmd = REPLAY_BINARY_PATH.to_string_lossy().to_string();
    let mut args = vec!["stdio", mode_arg]
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
        description: "Test".to_string(),
        cmd,
        args,
        envs,
        env_keys: vec![],
        timeout: Some(30),
        bundled: Some(false),
        available_tools: vec![],
    };
    let extension_manager = ExtensionManager::new_without_provider();

    #[allow(clippy::redundant_closure_call)]
    let result = (async || -> Result<(), Box<dyn std::error::Error>> {
        extension_manager.add_extension(extension_config).await?;
        let mut results = Vec::new();
        for tool_call in tool_calls {
            let tool_call = CallToolRequestParam {
                name: format!("test__{}", tool_call.name).into(),
                arguments: tool_call.arguments,
            };
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
        if matches!(mode, TestMode::Playback) {
            let errors =
                fs::read_to_string(format!("{}.errors.txt", replay_file_path.to_string_lossy()))
                    .expect("could not read errors");
            eprintln!("errors from {}", replay_file_path.to_string_lossy());
            eprintln!("{}", errors);
            eprintln!();
        }
        panic!("Test failed: {:?}", err);
    }
}
