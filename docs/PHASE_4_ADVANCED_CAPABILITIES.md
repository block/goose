# Phase 4: Advanced Agent Capabilities

## Overview

Phase 4 introduces sophisticated autonomous agent features that transform Goose into a truly intelligent coding assistant with planning, self-critique, and structured execution capabilities.

## Key Features

### 🧠 **ExecutionMode System**
- **Freeform Mode**: Traditional LLM autonomy (default)
- **Structured Mode**: State graph-driven execution with validation gates

### 📋 **Planning System**
- Multi-step plan creation and execution
- Automatic plan progression based on tool usage
- Context injection for LLM planning awareness

### 🔍 **Self-Critique System**
- Automated work quality assessment
- Issue categorization (blocking vs. warnings)
- Decision support for completion vs. iteration

### 🔗 **Enhanced MCP Integration**
- Shell command extraction from MCP tools
- Security approval for extension-executed commands
- Seamless integration with existing approval policies

## CLI Usage

### Execution Modes

```bash
# Default freeform mode
goose run --text "implement user authentication"

# Structured mode with state graph execution
goose run --execution-mode structured --text "build REST API with tests"

# Combined with approval policies
goose run --execution-mode structured --approval-policy paranoid --text "deploy to production"
```

### Available Execution Modes

| Mode | Description | Use Case |
|------|-------------|----------|
| `freeform` | LLM has full autonomy (default) | Exploratory tasks, prototyping |
| `structured` | Follows CODE→TEST→FIX→DONE flow | Production code, critical systems |

## Programming API

### Agent Execution Mode Control

```rust
use goose::agents::{Agent, ExecutionMode};

// Switch to structured mode
agent.set_execution_mode(ExecutionMode::Structured).await;

// Check current mode
let mode = agent.execution_mode().await;
if agent.is_structured_mode().await {
    println!("Using structured execution with planning");
}
```

### Planning System Integration

```rust
use goose::agents::{PlanContext, PlanManager};

// Create a plan
agent.create_plan(
    "Implement OAuth2 authentication",
    vec!["code_execution".to_string(), "bash".to_string()],
    "/workspace/auth-service"
).await?;

// Check plan status
if agent.has_active_plan().await {
    // Get context for LLM injection
    let context = agent.get_plan_context().await;
    
    // Simulate progress
    agent.process_plan_progress(&["code_execution".to_string()], true).await;
}
```

### Self-Critique Workflow

```rust
use goose::agents::{CritiqueDecision, AggregatedCritique};

// Perform self-critique
let critique = agent.self_critique(
    "Implement user login system", 
    vec!["src/auth.rs".to_string(), "tests/auth_test.rs".to_string()],
    "/workspace",
    Some("Build successful".to_string()),
    Some("All tests passed".to_string())
).await?;

// Make decision based on critique
let decision = agent.critique_and_decide(&critique).await;
match decision {
    CritiqueDecision::Complete => {
        println!("Work completed successfully");
    }
    CritiqueDecision::CompleteWithWarnings { warnings } => {
        println!("Completed with {} non-blocking issues", warnings);
    }
    CritiqueDecision::NeedsWork { blocking_issues } => {
        println!("Needs improvement: {:?}", blocking_issues);
        // Continue working...
    }
}
```

## StateGraph Enhanced Configuration

```rust
use goose::agents::state_graph::{StateGraphConfig, ProjectType};
use std::path::PathBuf;

let config = StateGraphConfig {
    max_iterations: 10,
    max_fix_attempts: 3,
    test_command: Some("cargo test --no-fail-fast".to_string()),
    working_dir: PathBuf::from("/workspace"),
    use_done_gate: true,           // Enable final validation
    project_type: Some(ProjectType::Rust), // Auto-configure for Rust
};

let graph = StateGraph::new(config);
```

### Project Type Configuration

| ProjectType | Test Command | Done Gate | Use Case |
|-------------|--------------|-----------|----------|
| `Rust` | `cargo test` | Cargo + rustc checks | Rust applications |
| `Node` | `npm test` | npm + ESLint checks | JavaScript/TypeScript |
| `Python` | `pytest` | pytest + pylint checks | Python applications |
| `Custom` | User-defined | User-defined checks | Other languages |

## Enhanced MCP Tool Security

### Shell Command Extraction

The system automatically detects and applies approval policies to shell commands executed by MCP tools:

```yaml
# MCP extension configuration
extensions:
  playwright:
    type: stdio
    cmd: npx
    args: ["-y", "@playwright/mcp@latest"]
```

Commands executed through MCP tools like:
- `bash__execute` → `docker run ubuntu:latest`
- `shell__run` → `git push --force origin main`
- `terminal__exec` → `rm -rf /important/data`

Are automatically subject to the configured approval policy.

### Detected Tool Patterns

```rust
// Shell-executing tool patterns detected:
let shell_tools = [
    "bash", "shell", "execute", "run_command",
    "run_bash_command", "execute_command", 
    "shell_execute", "terminal", "exec"
];
```

## Complete Workflow Example

### Structured Development Session

```bash
# Start structured session with safe approval
goose run --execution-mode structured --approval-policy safe \
  --text "Create a web service with authentication, tests, and documentation"
```

**Agent Behavior:**
1. **Planning Phase**: Creates multi-step plan
   - Step 1: Set up project structure
   - Step 2: Implement core authentication
   - Step 3: Add comprehensive tests
   - Step 4: Generate documentation
   - Step 5: Validate and deploy

2. **Execution Phase**: Follows CODE→TEST→FIX loop for each step
   - Automatic plan progression based on tool execution
   - Shell command approval for security
   - Continuous state validation

3. **Critique Phase**: Self-assessment after each major step
   - Code quality analysis
   - Test coverage verification
   - Security vulnerability scanning
   - Performance concern identification

4. **Done Gate Validation**: Final comprehensive check
   - Build success verification
   - Test suite completion
   - Lint compliance
   - Documentation completeness

## Integration with Existing Features

### Approval Policies
```rust
// Combined execution mode and approval policy
agent.set_execution_mode(ExecutionMode::Structured).await;
agent.set_approval_policy(ApprovalPreset::Paranoid).await;
```

### Session Management
- Execution mode persists across session resume
- Plan state maintained during interruptions
- Critique history preserved for context

### Extension Compatibility
- All existing MCP extensions work seamlessly
- Enhanced security for shell-executing tools
- Automatic command pattern detection

## Development and Testing

### Running Phase 4 Tests

```bash
# Core functionality tests
cargo test --package goose --test phase4_integration_test

# Full integration with MCP sidecars
cargo test --package goose --test mcp_sidecar_integration_test

# Comprehensive agent behavior
cargo test --package goose --test state_graph_integration_test
```

### Adding Custom Critics

```rust
use goose::agents::{Critic, CritiqueResult, CritiqueContext};

#[derive(Debug)]
struct SecurityCritic;

impl Critic for SecurityCritic {
    fn name(&self) -> &str { "security" }
    
    async fn critique(&self, context: &CritiqueContext) -> Result<CritiqueResult> {
        // Custom security analysis logic
        // Return issues with severity levels
    }
}

// Add to critic manager
let mut manager = CriticManager::new();
manager.add_critic(Box::new(SecurityCritic));
```

### Custom Planning Strategies

```rust
use goose::agents::{Planner, PlanContext, Plan};

#[derive(Debug)]
struct DomainSpecificPlanner;

impl Planner for DomainSpecificPlanner {
    async fn create_plan(&self, context: &PlanContext) -> Result<Plan> {
        // Domain-specific planning logic
        // Return structured plan with dependencies
    }
}
```

## Configuration Examples

### Production Configuration

```yaml
# ~/.config/goose/config.yaml
execution:
  default_mode: structured
  planning: enabled
  critique: enabled

approval:
  default_policy: safe
  environment_detection: true

state_graph:
  max_iterations: 15
  use_done_gate: true
  project_auto_detection: true
```

### Development Configuration

```yaml
execution:
  default_mode: freeform
  planning: disabled
  critique: optional

approval:
  default_policy: autopilot  # Only in sandboxes
  
state_graph:
  max_iterations: 5
  use_done_gate: false
```

## Performance and Scaling

### Memory Usage
- Planning state: ~10-50KB per active plan
- Critique history: ~5-20KB per critique session
- No significant overhead in freeform mode

### Execution Speed
- Structured mode: +20-30% execution time (due to validation)
- Planning overhead: ~1-2 seconds per plan creation
- Critique analysis: ~2-5 seconds per assessment

### Parallelization
- Multiple agents can run simultaneously
- Plan execution supports parallel tool calls
- Critique analysis runs in background

## Troubleshooting

### Common Issues

**Q: Structured mode not enabling planning**
```rust
// Ensure proper initialization
agent.set_execution_mode(ExecutionMode::Structured).await;
assert!(agent.is_planning_enabled().await);
```

**Q: MCP tool commands not being caught by approval**
```rust
// Verify tool name patterns
let is_shell_tool = tool_name.ends_with("bash") || 
                   tool_name.contains("execute");
```

**Q: StateGraph not using done gate**
```rust
let config = StateGraphConfig {
    use_done_gate: true,  // Must be explicitly enabled
    project_type: Some(ProjectType::Rust),
    // ... other config
};
```

### Debug Information

```rust
// Enable detailed logging
RUST_LOG=goose::agents=debug cargo test

// Check agent state
println!("Mode: {:?}", agent.execution_mode().await);
println!("Planning: {}", agent.is_planning_enabled().await);
println!("Has Plan: {}", agent.has_active_plan().await);
```

## Migration from Phase 3

Phase 4 is fully backward compatible. Existing code continues to work in freeform mode by default.

**Optional Migration Steps:**
1. Add execution mode configuration to CLI usage
2. Enable structured mode for critical workflows
3. Configure project-specific done gates
4. Add custom critics for domain-specific validation

## Conclusion

Phase 4 transforms Goose into a sophisticated autonomous agent with:
- **Strategic Planning**: Multi-step execution with progress tracking
- **Quality Assurance**: Self-critique and validation systems
- **Security Controls**: Enhanced MCP tool command approval
- **Flexible Execution**: Structured vs. freeform modes

This represents a significant advancement in AI agent capabilities while maintaining full compatibility with existing Goose workflows.
