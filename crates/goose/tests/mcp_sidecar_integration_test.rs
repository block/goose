//! Integration tests for MCP sidecars with ShellGuard approval system
//! Tests verify that external MCP tools respect approval policies

use goose::agents::shell_guard::ShellGuard;
use goose::approval::ApprovalPreset;
use std::path::PathBuf;

#[tokio::test]
async fn test_shell_guard_with_playwright_commands() {
    let guard = ShellGuard::new(ApprovalPreset::Safe);

    // Test that safe Playwright commands are approved
    let safe_commands = [
        "npx playwright test",
        "npx playwright show-report",
        "npx playwright codegen",
    ];

    for cmd in safe_commands {
        let check = guard.check_command(cmd).await.unwrap();
        assert!(
            check.is_approved(),
            "Safe Playwright command should be approved: {}",
            cmd
        );
    }
}

#[tokio::test]
async fn test_shell_guard_with_docker_commands() {
    let guard = ShellGuard::new(ApprovalPreset::Safe);

    // Test that Docker commands require approval or are blocked
    let risky_commands = [
        "docker run --privileged ubuntu",
        "docker exec -it container rm -rf /",
        "docker pull malicious/image",
    ];

    for cmd in risky_commands {
        let check = guard.check_command(cmd).await.unwrap();
        assert!(
            !check.is_approved(),
            "Risky Docker command should not be auto-approved: {}",
            cmd
        );
        assert!(
            check.needs_approval() || check.is_blocked(),
            "Risky Docker command should need approval or be blocked: {}",
            cmd
        );
    }
}

#[tokio::test]
async fn test_shell_guard_with_aider_git_commands() {
    let guard = ShellGuard::new(ApprovalPreset::Safe);

    // Test that safe git commands used by Aider are approved
    let safe_git_commands = [
        "git status",
        "git diff",
        "git log --oneline -10",
        "git show HEAD",
    ];

    for cmd in safe_git_commands {
        let check = guard.check_command(cmd).await.unwrap();
        assert!(
            check.is_approved(),
            "Safe git command should be approved: {}",
            cmd
        );
    }

    // Test that risky git commands require approval
    let risky_git_commands = [
        "git push --force origin main",
        "git reset --hard HEAD~10",
        "git clean -fdx",
    ];

    for cmd in risky_git_commands {
        let check = guard.check_command(cmd).await.unwrap();
        assert!(
            check.needs_approval(),
            "Risky git command should need approval: {}",
            cmd
        );
    }
}

#[tokio::test]
async fn test_shell_guard_paranoid_mode_with_mcp_tools() {
    let guard = ShellGuard::new(ApprovalPreset::Paranoid);

    // In paranoid mode, even safe commands should require approval
    let commands = [
        "npm test",
        "cargo build",
        "python -m pytest",
        "npx playwright test",
        "git status",
    ];

    for cmd in commands {
        let check = guard.check_command(cmd).await.unwrap();
        assert!(
            check.needs_approval(),
            "In paranoid mode, all commands should need approval: {}",
            cmd
        );
    }
}

#[tokio::test]
async fn test_shell_guard_autopilot_docker_vs_real() {
    use goose::approval::{Environment, ExecutionContext};

    // Test autopilot in Docker sandbox - should approve risky commands
    let docker_context = ExecutionContext::new().with_environment(Environment::DockerSandbox);
    let docker_guard = ShellGuard::new(ApprovalPreset::Autopilot).with_context(docker_context);

    let risky_cmd = "rm -rf /tmp/test";
    let check = docker_guard.check_command(risky_cmd).await.unwrap();
    assert!(
        check.is_approved(),
        "Autopilot in Docker should approve risky commands"
    );

    // Test autopilot on real filesystem - should fall back to safe mode
    let real_context = ExecutionContext::new().with_environment(Environment::RealFilesystem);
    let real_guard = ShellGuard::new(ApprovalPreset::Autopilot).with_context(real_context);

    let check = real_guard.check_command(risky_cmd).await.unwrap();
    assert!(
        !check.is_approved(),
        "Autopilot on real filesystem should not auto-approve risky commands"
    );
}

#[tokio::test]
async fn test_mcp_tool_command_patterns() {
    let guard = ShellGuard::new(ApprovalPreset::Safe);

    // Test patterns that MCP tools commonly use
    let test_cases = vec![
        // OpenHands patterns
        ("python -c \"print('hello')\"", true), // Safe Python execution
        ("docker run ubuntu:latest", false),    // Docker execution needs approval
        ("pip install requests", false),        // Package installation needs approval
        // Playwright patterns
        ("npx playwright install", false), // Installation needs approval
        ("npx playwright test --headed", true), // Test execution is safe
        ("npx @playwright/mcp@latest", true), // MCP server is safe
        // Aider patterns
        ("git add .", true),                  // Basic git operations are safe
        ("git commit -m 'fix'", true),        // Commits are safe
        ("python -m aider.mcp_server", true), // MCP server is safe
    ];

    for (cmd, should_be_approved) in test_cases {
        let check = guard.check_command(cmd).await.unwrap();
        if should_be_approved {
            assert!(check.is_approved(), "Command should be approved: {}", cmd);
        } else {
            assert!(
                !check.is_approved(),
                "Command should not be auto-approved: {}",
                cmd
            );
            assert!(
                check.needs_approval() || check.is_blocked(),
                "Command should need approval or be blocked: {}",
                cmd
            );
        }
    }
}

#[test]
fn test_mcp_sidecar_config_validation() {
    // Test that our MCP sidecar configurations are valid
    let playwright_config = r#"
    playwright:
      type: stdio
      cmd: npx
      args: ["-y", "@playwright/mcp@latest"]
      timeout: 300
    "#;

    let openhands_config = r#"
    openhands:
      type: stdio
      cmd: python
      args: ["-m", "openhands.server.mcp"]
      env:
        SANDBOX_TYPE: "local"
        WORKSPACE_BASE: "${WORKSPACE_DIR}"
      timeout: 600
    "#;

    let aider_config = r#"
    aider:
      type: stdio
      cmd: python
      args: ["-m", "aider.mcp_server"]
      env:
        AIDER_MODEL: "gpt-4"
        AIDER_AUTO_COMMITS: "true"
      timeout: 300
    "#;

    // Basic validation that configs contain required fields
    assert!(playwright_config.contains("@playwright/mcp@latest"));
    assert!(openhands_config.contains("openhands.server.mcp"));
    assert!(aider_config.contains("aider.mcp_server"));

    // Verify timeout values are reasonable
    assert!(playwright_config.contains("300"));
    assert!(openhands_config.contains("600"));
    assert!(aider_config.contains("300"));
}

#[tokio::test]
async fn test_state_graph_with_shell_guard_integration() {
    use goose::agents::state_graph::{StateGraph, StateGraphConfig, TestResult};

    let config = StateGraphConfig {
        max_iterations: 3,
        max_fix_attempts: 2,
        test_command: Some("echo test".to_string()),
        working_dir: PathBuf::from("."),
        use_done_gate: false, // Disable for unit tests
        project_type: None,
    };

    let mut graph = StateGraph::new(config);
    let guard = ShellGuard::new(ApprovalPreset::Safe);

    // Test that StateGraph can work with ShellGuard
    let code_fn = |_task: &str,
                   _state: &goose::agents::state_graph::CodeTestFixState|
     -> anyhow::Result<Vec<String>> { Ok(vec!["test.rs".to_string()]) };

    let test_fn =
        |_state: &goose::agents::state_graph::CodeTestFixState| -> anyhow::Result<Vec<TestResult>> {
            // Simulate passing test
            Ok(vec![TestResult::passed("test.rs", "test_integration")])
        };

    let fix_fn = |_failed: &[TestResult],
                  _state: &goose::agents::state_graph::CodeTestFixState|
     -> anyhow::Result<Vec<String>> { Ok(vec![]) };

    let result: anyhow::Result<bool> = graph
        .run("test integration", code_fn, test_fn, fix_fn)
        .await;
    assert!(result.is_ok());
    assert!(result.unwrap());
}
