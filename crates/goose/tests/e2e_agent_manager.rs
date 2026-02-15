//! E2E integration test for AgentClientManager.
//!
//! Builds and spawns the echo_acp_agent example binary,
//! connects via AgentClientManager, and round-trips a prompt.

use agent_client_protocol_schema::NewSessionRequest;
use goose::agent_manager::client::AgentClientManager;
use goose::agent_manager::spawner::current_platform_key;
use goose::registry::manifest::{AgentDistribution, BinaryTarget};
use std::collections::HashMap;
use std::path::PathBuf;

fn echo_agent_distribution() -> AgentDistribution {
    let mut binary_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    binary_path.pop(); // crates/
    binary_path.pop(); // root
    binary_path.push("target");
    binary_path.push("debug");
    binary_path.push("examples");
    binary_path.push("echo_acp_agent");

    let key = current_platform_key();
    let mut binary = HashMap::new();
    binary.insert(
        key,
        BinaryTarget {
            archive: String::new(),
            cmd: binary_path.to_string_lossy().to_string(),
            args: vec![],
            env: HashMap::new(),
        },
    );

    AgentDistribution {
        binary,
        npx: None,
        uvx: None,
        cargo: None,
        docker: None,
    }
}

fn build_echo_agent() {
    let status = std::process::Command::new("cargo")
        .args(["build", "--example", "echo_acp_agent", "-p", "goose"])
        .status()
        .expect("failed to run cargo build");
    assert!(status.success(), "Failed to build echo_acp_agent example");
}

#[test]
fn test_e2e_connect_and_create_session() {
    build_echo_agent();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        let manager = AgentClientManager::default();
        let dist = echo_agent_distribution();

        manager
            .connect_with_distribution("echo-test".to_string(), &dist)
            .await
            .expect("should connect to echo agent");

        let agents = manager.list_agents().await;
        assert!(agents.contains(&"echo-test".to_string()));

        let cwd = std::env::current_dir().unwrap();
        let req = NewSessionRequest::new(cwd);
        let resp = manager
            .new_session("echo-test", req)
            .await
            .expect("should create session");

        let session_id = &resp.session_id;
        assert!(!session_id.0.is_empty());

        manager
            .disconnect_agent("echo-test")
            .await
            .expect("should disconnect");
    });
}

#[test]
fn test_e2e_prompt_round_trip() {
    build_echo_agent();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        let manager = AgentClientManager::default();
        let dist = echo_agent_distribution();

        manager
            .connect_with_distribution("echo-prompt".to_string(), &dist)
            .await
            .expect("should connect");

        let cwd = std::env::current_dir().unwrap();
        let req = NewSessionRequest::new(cwd);
        let resp = manager
            .new_session("echo-prompt", req)
            .await
            .expect("should create session");

        let session_id = resp.session_id.clone();

        let response_text = manager
            .prompt_agent_text("echo-prompt", &session_id, "Hello, echo agent!")
            .await
            .expect("should get response");

        assert!(
            response_text.contains("echo:"),
            "Expected echo text, got: {response_text:?}"
        );
        assert!(
            response_text.contains("Hello, echo agent!"),
            "Expected prompt text in echo, got: {response_text:?}"
        );

        manager.shutdown_all().await;
    });
}
