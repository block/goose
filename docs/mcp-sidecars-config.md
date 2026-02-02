# MCP Sidecar Configuration Guide

This document provides configuration examples for integrating external AI tools as MCP sidecars in Goose.

## Prerequisites

- Node.js 18+ for Playwright MCP
- Python 3.11+ for OpenHands and Aider
- Docker for OpenHands sandbox mode

## Extension Configurations

Add these to your `~/.config/goose/config.yaml` under the `extensions` section.

### Playwright MCP (Visual Testing)

```yaml
playwright:
  type: stdio
  enabled: true
  name: playwright
  display_name: "Playwright"
  description: "Browser automation and visual testing"
  cmd: npx
  args:
    - "-y"
    - "@playwright/mcp@latest"
  env: {}
  timeout: 300
```

**Usage:**
- Visual regression testing
- Browser automation
- Screenshot capture
- DOM inspection

### OpenHands (Cloud Coding Agent)

```yaml
openhands:
  type: stdio
  enabled: true
  name: openhands
  display_name: "OpenHands"
  description: "AI coding agent with Docker sandbox"
  cmd: python
  args:
    - "-m"
    - "openhands.server.mcp"
  env:
    SANDBOX_TYPE: "local"
    SANDBOX_CONTAINER_IMAGE: "docker.all-hands.dev/all-hands-ai/runtime:0.20-nikolaik"
    WORKSPACE_BASE: "${WORKSPACE_DIR}"
  timeout: 600
  working_dir: "${WORKSPACE_DIR}"
```

**Environment Variables:**
- `SANDBOX_TYPE`: Set to `local` for Docker sandbox
- `SANDBOX_CONTAINER_IMAGE`: OpenHands runtime image
- `WORKSPACE_BASE`: Directory to mount in sandbox

**Usage:**
- Complex multi-file refactoring
- Autonomous bug fixing
- Large-scale code generation

### Aider (Precision Edits)

```yaml
aider:
  type: stdio
  enabled: true
  name: aider
  display_name: "Aider"
  description: "AI pair programming with precise diffs"
  cmd: python
  args:
    - "-m"
    - "aider.mcp_server"
  env:
    AIDER_MODEL: "gpt-4"
    AIDER_AUTO_COMMITS: "true"
  timeout: 300
  working_dir: "${WORKSPACE_DIR}"
```

**Usage:**
- Surgical code edits
- Git-aware changes
- Automatic commits

## Approval Policy Integration

Use with the approval presets for safety:

```yaml
# In config.yaml
approval_policy: safe  # Options: safe, paranoid, autopilot

# CLI usage
goose --approval-policy paranoid
goose --approval-policy autopilot  # Only in Docker sandbox!
```

### Policy Behaviors

| Policy | Safe Commands | High-Risk Commands | Critical Commands |
|--------|--------------|-------------------|-------------------|
| **safe** | Auto-approve | Prompt user | Block |
| **paranoid** | Prompt user | Prompt user | Block |
| **autopilot** | Auto-approve* | Auto-approve* | Auto-approve* |

*Autopilot only auto-approves inside Docker sandbox. On real filesystem, falls back to `safe` mode.

## StateGraph Integration

The StateGraph provides a self-correcting CODE → TEST → FIX loop:

```rust
use goose::agents::state_graph::{StateGraph, StateGraphConfig};

let config = StateGraphConfig {
    max_iterations: 10,
    max_fix_attempts: 3,
    test_command: Some("cargo test".to_string()),
    working_dir: PathBuf::from("."),
};

let mut graph = StateGraph::new(config);
let success = graph.run(
    "Implement feature X",
    code_generation_fn,
    test_execution_fn,
    fix_application_fn,
).await?;
```

## Done Gate Verification

Before marking a task complete, run the Done Gate:

```rust
use goose::agents::done_gate::{DoneGate, GateResult};

let gate = DoneGate::rust_defaults();  // or node_defaults(), python_defaults()
let (result, checks) = gate.verify(workspace_path)?;

match result {
    GateResult::Done => println!("All checks passed!"),
    GateResult::ReEnterFix { check_name, message } => {
        println!("Fix needed: {} - {}", check_name, message);
    }
    GateResult::Failed { reason } => {
        println!("Task failed: {}", reason);
    }
}
```

## Test Parser Usage

Parse test output from various frameworks:

```rust
use goose::test_parsers::{parse_test_output, TestFramework};

// Auto-detect framework from command
let framework = TestFramework::detect_from_command("pytest tests/");
let results = parse_test_output(&test_output, framework);

for result in results {
    println!("{}: {} - {:?}", result.file, result.test_name, result.status);
}
```

## Full Integration Example

```rust
use goose::agents::{
    state_graph::{StateGraph, StateGraphConfig},
    done_gate::DoneGate,
    shell_guard::ShellGuard,
};
use goose::approval::ApprovalPreset;
use goose::test_parsers::{parse_test_output, TestFramework};

async fn run_task(task: &str, workspace: &Path) -> Result<bool> {
    // 1. Initialize shell guard with approval policy
    let guard = ShellGuard::new(ApprovalPreset::Safe);
    
    // 2. Create StateGraph for self-correcting loop
    let config = StateGraphConfig {
        max_iterations: 10,
        max_fix_attempts: 3,
        test_command: Some("cargo test --no-fail-fast".to_string()),
        working_dir: workspace.to_path_buf(),
    };
    let mut graph = StateGraph::new(config);
    
    // 3. Run the graph with code/test/fix functions
    let success = graph.run(
        task,
        |task, state| {
            // Generate code using LLM
            Ok(vec!["src/main.rs".to_string()])
        },
        |state| {
            // Run tests and parse results
            let output = run_command("cargo test --no-fail-fast")?;
            Ok(parse_test_output(&output, TestFramework::Cargo))
        },
        |failed_tests, state| {
            // Apply fixes based on failures
            Ok(vec!["src/main.rs".to_string()])
        },
    ).await?;
    
    // 4. Verify with Done Gate
    if success {
        let gate = DoneGate::rust_defaults();
        let (result, _) = gate.verify(workspace)?;
        return Ok(matches!(result, GateResult::Done));
    }
    
    Ok(false)
}
```

## Troubleshooting

### Playwright MCP Not Found
```bash
npm install -g @playwright/mcp@latest
npx playwright install chromium
```

### OpenHands Docker Issues
```bash
docker pull docker.all-hands.dev/all-hands-ai/runtime:0.20-nikolaik
docker run --rm -it docker.all-hands.dev/all-hands-ai/runtime:0.20-nikolaik echo "OK"
```

### Aider Installation
```bash
pip install aider-chat
aider --version
```
