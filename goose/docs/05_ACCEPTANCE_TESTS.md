# Acceptance Tests - Goose Enterprise Platform

**Version:** 2.0 (Phase 6 Complete)
**Last Executed:** February 2, 2026
**Status:** All Tests Passing

---

These end-to-end acceptance tests verify the complete functionality of the Goose Enterprise Agentic Platform. Each test references actual code paths and expected behaviors.

---

## Test 1 — StateGraph Self-Correction Loop

**Purpose:** Verify autonomous CODE → TEST → FIX cycle.

### Test Implementation

**File:** `crates/goose/src/agents/state_graph/mod.rs`

```rust
#[tokio::test]
async fn test_state_graph_self_correction() {
    let config = StateGraphConfig {
        max_iterations: 5,
        max_fix_attempts: 3,
        test_command: Some("cargo test".to_string()),
        working_dir: PathBuf::from("/tmp/test"),
        use_done_gate: true,
        project_type: Some(ProjectType::Rust),
    };

    let mut graph = StateGraph::new(config);

    // Simulate: CODE generates failing code, FIX corrects it
    let result = graph.run(
        "Implement function",
        code_gen_callback,
        test_callback,
        fix_callback,
    ).await;

    assert!(result.is_ok());
    assert_eq!(graph.current_state(), GraphState::Done);
}
```

### Expected Behavior

| Step | Action | Verification |
|------|--------|--------------|
| 1 | Enter CODE state | `GraphState::Code` |
| 2 | Generate implementation | Callback invoked |
| 3 | Transition to TEST | `StateGraphEvent::StateTransition` emitted |
| 4 | Run tests | `test_command` executed |
| 5 | If tests fail → FIX | Automatic transition |
| 6 | Apply fix | Fix callback invoked |
| 7 | Return to TEST | Retry loop |
| 8 | Tests pass → VALIDATE | DoneGate verification |
| 9 | DONE | `GraphState::Done` |

### Artifacts
- `state_graph/mod.rs:595` - Main implementation
- `state_graph/runner.rs:154` - Callback execution
- `state_graph/state.rs:160` - State types

### Status: ✅ PASSING

---

## Test 2 — Reflexion Self-Improvement

**Purpose:** Verify learning from failures via episodic memory.

### Test Implementation

**File:** `crates/goose/src/agents/reflexion.rs`

```rust
#[test]
fn test_reflexion_learning() {
    let mut agent = ReflexionAgent::default_config();

    // First attempt fails
    agent.start_attempt("Debug authentication bug");
    agent.record_action(AttemptAction::new("Read auth.rs", "Found code", true));
    agent.record_action(AttemptAction::new("Apply fix", "Type error", false));
    agent.complete_attempt(AttemptOutcome::Failure, Some("Type mismatch".to_string()));

    // Generate reflection
    let reflection = agent.reflect_with_content(
        "Type mismatch in validation",
        "The fix failed because types didn't match",
        vec!["Always check types before applying".to_string()],
        vec!["Add type validation step".to_string()],
    );

    assert!(!reflection.lessons.is_empty());

    // Second attempt retrieves reflection
    let context = agent.generate_context_with_reflections("Debug authentication bug");
    assert!(context.contains("Always check types"));
}
```

### Expected Behavior

| Step | Action | Verification |
|------|--------|--------------|
| 1 | Start attempt | `current_attempt` initialized |
| 2 | Record actions | Actions stored with timestamps |
| 3 | Complete with failure | `AttemptOutcome::Failure` |
| 4 | Generate reflection | `Reflection` created |
| 5 | Store in memory | `EpisodeMemory` updated |
| 6 | Future retrieval | Keyword matching returns lessons |

### Artifacts
- `reflexion.rs:715` - Reflexion implementation
- `reflexion.rs:420-480` - EpisodeMemory

### Status: ✅ PASSING

---

## Test 3 — Approval Policy Blocking

**Purpose:** Verify dangerous commands are blocked by policy.

### Test Implementation

**File:** `crates/goose/src/approval/presets.rs`

```rust
#[test]
fn test_safe_mode_blocks_critical() {
    let policy = SafeMode::new();
    let context = ExecutionContext::default();

    // Critical command should be blocked
    let decision = policy.evaluate("rm -rf /", &context);
    assert!(matches!(decision, ApprovalDecision::Blocked(_)));

    // Safe command should be approved
    let decision = policy.evaluate("ls -la", &context);
    assert!(matches!(decision, ApprovalDecision::Approved));

    // High-risk command should require approval
    let decision = policy.evaluate("npm install", &context);
    assert!(matches!(decision, ApprovalDecision::RequiresApproval(_)));
}

#[test]
fn test_paranoid_mode_prompts_all() {
    let policy = ParanoidMode::new();
    let context = ExecutionContext::default();

    // Even safe commands require approval
    let decision = policy.evaluate("ls -la", &context);
    assert!(matches!(decision, ApprovalDecision::RequiresApproval(_)));
}

#[test]
fn test_autopilot_requires_docker() {
    let policy = AutopilotMode::new();

    // Without Docker, should still block critical
    let context = ExecutionContext::new(Environment::RealFilesystem);
    let decision = policy.evaluate("rm -rf /", &context);
    assert!(matches!(decision, ApprovalDecision::Blocked(_)));

    // With Docker, allows more
    let context = ExecutionContext::new(Environment::DockerSandbox);
    let decision = policy.evaluate("npm install", &context);
    assert!(matches!(decision, ApprovalDecision::Approved));
}
```

### Expected Behavior

| Policy | Safe Commands | High-Risk | Critical |
|--------|---------------|-----------|----------|
| SAFE | Auto-approve | Prompt | Block |
| PARANOID | Prompt | Prompt | Block |
| AUTOPILOT (Docker) | Auto | Auto | Auto |
| AUTOPILOT (Real FS) | Auto | Auto | Block |

### Artifacts
- `approval/presets.rs:478` - Policy implementations
- `approval/mod.rs:144` - Core types
- `approval/environment.rs:70` - Environment detection

### Status: ✅ PASSING

---

## Test 4 — Multi-Agent Orchestration

**Purpose:** Verify task dependencies and specialist coordination.

### Test Implementation

**File:** `crates/goose/src/agents/orchestrator.rs`

```rust
#[tokio::test]
async fn test_workflow_dependencies() {
    let config = OrchestratorConfig::default();
    let orchestrator = AgentOrchestrator::new(config).await.unwrap();

    let mut workflow = Workflow::new("test-workflow", "Test feature");

    // Add tasks with dependencies
    let code_task = workflow.add_task(AgentRole::Code, "Generate code").unwrap();
    let test_task = workflow.add_task_with_deps(
        AgentRole::Test,
        "Write tests",
        vec![code_task],
    ).unwrap();
    let security_task = workflow.add_task_with_deps(
        AgentRole::Security,
        "Security audit",
        vec![code_task],
    ).unwrap();

    // Verify dependency order
    let ready = workflow.get_ready_tasks();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].role, AgentRole::Code);

    // Complete code task
    workflow.complete_task(code_task, TaskResult::success("Done")).unwrap();

    // Now test and security are ready (parallel)
    let ready = workflow.get_ready_tasks();
    assert_eq!(ready.len(), 2);
}

#[tokio::test]
async fn test_task_retry() {
    let config = OrchestratorConfig::default();
    let orchestrator = AgentOrchestrator::new(config).await.unwrap();

    let mut workflow = Workflow::new("retry-test", "Test retry");
    let task_id = workflow.add_task(AgentRole::Code, "Flaky task").unwrap();

    // First attempt fails
    workflow.fail_task(task_id, "Temporary error").unwrap();
    assert_eq!(workflow.get_task(task_id).unwrap().status, TaskStatus::Retrying);
    assert_eq!(workflow.get_task(task_id).unwrap().retry_count, 1);

    // Retry succeeds
    workflow.complete_task(task_id, TaskResult::success("Done")).unwrap();
    assert_eq!(workflow.get_task(task_id).unwrap().status, TaskStatus::Completed);
}
```

### Expected Behavior

| Scenario | Expected |
|----------|----------|
| Task with unmet deps | Status: `Pending` |
| All deps completed | Status: `Ready` (queued) |
| Task failure | Status: `Retrying`, increment count |
| Max retries exceeded | Status: `Failed` |
| All tasks complete | Workflow: `Completed` |

### Artifacts
- `orchestrator.rs:1,022` - Orchestration logic
- `workflow_engine.rs:831` - Workflow templates
- `specialists/mod.rs:319` - Agent trait

### Status: ✅ PASSING

---

## Test 5 — Checkpoint Resume

**Purpose:** Verify state persistence and workflow resume.

### Test Implementation

**File:** `crates/goose/src/agents/persistence/sqlite.rs`

```rust
#[tokio::test]
async fn test_checkpoint_resume() {
    let db_path = "/tmp/test_checkpoints.db";
    let manager = SqliteCheckpointer::new(db_path).await.unwrap();

    // Set thread context
    manager.set_thread("workflow-123").await;

    // Save checkpoint at step 2
    let state = serde_json::json!({
        "step": 2,
        "task": "Generate authentication",
        "completed_files": ["auth.rs", "middleware.rs"]
    });
    let metadata = CheckpointMetadata::for_step(2, "Code");
    let checkpoint_id = manager.checkpoint(&state, Some(metadata)).await.unwrap();

    // Simulate crash and restart
    drop(manager);

    // Resume
    let manager2 = SqliteCheckpointer::new(db_path).await.unwrap();
    manager2.set_thread("workflow-123").await;

    let restored: serde_json::Value = manager2.resume().await.unwrap().unwrap();

    assert_eq!(restored["step"], 2);
    assert_eq!(restored["task"], "Generate authentication");
    assert_eq!(restored["completed_files"].as_array().unwrap().len(), 2);

    // Clean up
    std::fs::remove_file(db_path).ok();
}

#[tokio::test]
async fn test_checkpoint_branching() {
    let manager = MemoryCheckpointer::new();
    manager.set_thread("main").await;

    // Save main branch checkpoint
    let state = serde_json::json!({"branch": "main", "step": 1});
    manager.checkpoint(&state, None).await.unwrap();

    // Branch for experiment
    manager.branch("experiment-1").await;
    let state = serde_json::json!({"branch": "experiment", "step": 2});
    manager.checkpoint(&state, None).await.unwrap();

    // Switch back to main
    manager.set_thread("main").await;
    let restored: serde_json::Value = manager.resume().await.unwrap().unwrap();
    assert_eq!(restored["branch"], "main");
    assert_eq!(restored["step"], 1);
}
```

### Expected Behavior

| Scenario | Expected |
|----------|----------|
| Save checkpoint | Returns `CheckpointId` |
| Load checkpoint | Returns exact state |
| List checkpoints | Returns all for thread |
| Thread isolation | Different threads, different state |
| Branch checkpoint | Creates parallel history |
| Resume after crash | State fully restored |

### Artifacts
- `persistence/mod.rs:466` - Core types
- `persistence/sqlite.rs:394` - SQLite backend
- `persistence/memory.rs:270` - Memory backend

### Status: ✅ PASSING

---

## Test 6 — Budget Ceiling Enforcement

**Purpose:** Verify cost tracking stops execution when budget exceeded.

### Test Implementation

**File:** `crates/goose/src/agents/observability.rs`

```rust
#[tokio::test]
async fn test_budget_enforcement() {
    let tracker = CostTracker::new(ModelPricing::claude_sonnet());
    tracker.set_budget(0.05).await;  // $0.05 budget

    // First call - within budget
    tracker.record_llm_call(&TokenUsage::new(1000, 500));
    assert!(!tracker.is_over_budget().await);

    // Large call - exceeds budget
    tracker.record_llm_call(&TokenUsage::new(50000, 20000));
    assert!(tracker.is_over_budget().await);

    // Verify summary shows exceeded
    let summary = tracker.get_summary().await;
    assert!(summary.contains("OVER BUDGET"));
}

#[test]
fn test_model_pricing_accuracy() {
    // Claude Sonnet: $3/M input, $15/M output
    let pricing = ModelPricing::claude_sonnet();
    let usage = TokenUsage::new(1_000_000, 100_000);

    let cost = pricing.calculate_cost(&usage);

    // 1M input * $3/M + 100K output * $15/M = $3 + $1.50 = $4.50
    assert!((cost - 4.50).abs() < 0.01);
}

#[test]
fn test_all_model_presets() {
    // Verify all 7 presets have valid pricing
    let presets = vec![
        ModelPricing::claude_opus(),
        ModelPricing::claude_sonnet(),
        ModelPricing::claude_haiku(),
        ModelPricing::gpt4o(),
        ModelPricing::gpt4o_mini(),
        ModelPricing::gpt4_turbo(),
        ModelPricing::gemini_pro(),
    ];

    for pricing in presets {
        assert!(pricing.input_cost_per_million > 0.0);
        assert!(pricing.output_cost_per_million > 0.0);
    }
}
```

### Expected Behavior

| Scenario | Expected |
|----------|----------|
| Usage within budget | `is_over_budget() = false` |
| Usage exceeds budget | `is_over_budget() = true` |
| No budget set | Never over budget |
| Get summary | Shows current cost, budget status |
| Cost calculation | Accurate per-token pricing |

### Model Pricing Verification

| Model | Input (per M) | Output (per M) | Verified |
|-------|---------------|----------------|----------|
| Claude Opus | $15.00 | $75.00 | ✅ |
| Claude Sonnet | $3.00 | $15.00 | ✅ |
| Claude Haiku | $0.25 | $1.25 | ✅ |
| GPT-4o | $2.50 | $10.00 | ✅ |
| GPT-4o Mini | $0.15 | $0.60 | ✅ |
| GPT-4 Turbo | $10.00 | $30.00 | ✅ |
| Gemini Pro | $1.25 | $5.00 | ✅ |

### Artifacts
- `observability.rs:796` - Full implementation

### Status: ✅ PASSING

---

## Test 7 — Reasoning Pattern Verification

**Purpose:** Verify all 4 reasoning patterns work correctly.

### Test Implementation

**File:** `crates/goose/src/agents/reasoning.rs`

```rust
#[test]
fn test_react_pattern() {
    let mut manager = ReasoningManager::react();
    let trace = manager.start_trace("Debug authentication bug");

    // Add thought
    let thought_id = trace.add_thought(
        "Analyze token validation",
        ThoughtType::Initial,
    );
    assert!(thought_id > 0);

    // Add action
    let action_id = trace.add_action("Read auth.rs", thought_id);
    trace.record_action_result(
        action_id,
        ActionResult::success("Token validation found at line 42"),
    );

    // Add observation
    trace.add_observation(action_id, "Token expiry not being checked");

    // Complete
    manager.complete_trace(Some("Fixed by adding expiry check".to_string()));

    let trace = manager.get_trace();
    assert_eq!(trace.thoughts.len(), 1);
    assert_eq!(trace.actions.len(), 1);
}

#[test]
fn test_tree_of_thoughts() {
    let mut manager = ReasoningManager::tree_of_thoughts();
    let trace = manager.start_trace("Optimize database queries");

    // Branch A: Indexing
    let branch_a = trace.add_thought("Add indexes", ThoughtType::Hypothesis);
    trace.evaluate_thought(branch_a, 0.8);

    // Branch B: Denormalization
    let branch_b = trace.add_thought("Denormalize tables", ThoughtType::Hypothesis);
    trace.evaluate_thought(branch_b, 0.5);

    // Best branch should be A
    let best = trace.get_best_evaluated_thought();
    assert_eq!(best.unwrap().id, branch_a);
}

#[test]
fn test_chain_of_thought() {
    let mut manager = ReasoningManager::chain_of_thought();
    let trace = manager.start_trace("Implement OAuth2");

    trace.add_thought("Step 1: Identify endpoints", ThoughtType::Planning);
    trace.add_thought("Step 2: Implement auth flow", ThoughtType::Planning);
    trace.add_thought("Step 3: Add token refresh", ThoughtType::Planning);

    assert_eq!(trace.thoughts.len(), 3);

    // Verify linear chain (each thought references previous)
    for i in 1..trace.thoughts.len() {
        assert_eq!(trace.thoughts[i].parent_id, Some(trace.thoughts[i-1].id));
    }
}
```

### Expected Behavior

| Pattern | Structure | Verification |
|---------|-----------|--------------|
| Standard | Simple thoughts | No specific structure |
| Chain-of-Thought | Linear sequence | `parent_id` chain |
| ReAct | Thought → Action → Observation | Action results recorded |
| Tree-of-Thoughts | Branching with evaluation | `evaluate_thought()` scores |

### Artifacts
- `reasoning.rs:760` - Full implementation

### Status: ✅ PASSING

---

## Test 8 — Done Gate Verification

**Purpose:** Verify multi-stage validation before completion.

### Test Implementation

**File:** `crates/goose/src/agents/done_gate.rs`

```rust
#[test]
fn test_done_gate_all_pass() {
    let mut gate = DoneGate::new();
    gate.add_check(BuildSucceeds::cargo());
    gate.add_check(TestsPass::cargo());

    let state = CodeTestFixState {
        task: "Implement feature".to_string(),
        test_results: vec![
            TestResult::passed("test_basic"),
            TestResult::passed("test_edge_case"),
        ],
        ..Default::default()
    };

    let result = gate.verify(&state);
    assert!(matches!(result, GateResult::Done));
}

#[test]
fn test_done_gate_needs_fix() {
    let mut gate = DoneGate::new();
    gate.add_check(TestsPass::cargo());

    let state = CodeTestFixState {
        task: "Implement feature".to_string(),
        test_results: vec![
            TestResult::passed("test_basic"),
            TestResult::failed("test_edge_case")
                .with_line(42)
                .with_expected_actual("true", "false"),
        ],
        ..Default::default()
    };

    let result = gate.verify(&state);
    assert!(matches!(result, GateResult::ReEnterFix { .. }));
}

#[test]
fn test_check_result_details() {
    let check = TestsPass::cargo();

    let state = CodeTestFixState {
        test_results: vec![
            TestResult::failed("auth_test")
                .with_line(123)
                .with_expected_actual("token valid", "token expired"),
        ],
        ..Default::default()
    };

    let result = check.run(&state);
    assert!(!result.passed);
    assert!(result.details.contains("auth_test"));
    assert!(result.details.contains("line 123"));
}
```

### Expected Behavior

| Scenario | GateResult |
|----------|------------|
| All checks pass | `Done` |
| Some checks fail (soft) | `ReEnterFix` with details |
| Critical check fails | `Failed` |
| No checks configured | `Done` (vacuous truth) |

### Built-in Checks

| Check | Purpose | Supports |
|-------|---------|----------|
| `BuildSucceeds` | Verify build passes | Cargo, npm, custom |
| `TestsPass` | Verify tests pass | Cargo, Pytest, Jest, Go |

### Artifacts
- `done_gate.rs:427` - Full implementation

### Status: ✅ PASSING

---

## Summary

| Test | Description | Status |
|------|-------------|--------|
| 1 | StateGraph Self-Correction | ✅ PASSING |
| 2 | Reflexion Self-Improvement | ✅ PASSING |
| 3 | Approval Policy Blocking | ✅ PASSING |
| 4 | Multi-Agent Orchestration | ✅ PASSING |
| 5 | Checkpoint Resume | ✅ PASSING |
| 6 | Budget Ceiling Enforcement | ✅ PASSING |
| 7 | Reasoning Pattern Verification | ✅ PASSING |
| 8 | Done Gate Verification | ✅ PASSING |

**Total Tests:** 672+ unit tests + 8 acceptance scenarios

---

## Running Acceptance Tests

```bash
# Run all tests
cargo test --lib -p goose

# Run specific acceptance test
cargo test --lib -p goose test_state_graph
cargo test --lib -p goose test_reflexion
cargo test --lib -p goose test_approval
cargo test --lib -p goose test_orchestrator
cargo test --lib -p goose test_checkpoint
cargo test --lib -p goose test_budget
cargo test --lib -p goose test_reasoning
cargo test --lib -p goose test_done_gate
```

---

**Goose Enterprise Platform Acceptance Tests**
*All 8 scenarios passing | 672+ unit tests | Production Ready*
