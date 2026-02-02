# Goose Compound-AI Audit + Seamless Integration Plan (REVISED)

This plan performs a multi-layer audit of the Goose codebase and defines a staged strategy to integrate OpenHands, Aider, PydanticAI, PraisonAI, and LangGraph as **native-feeling Goose features** using MCP-first boundaries. **Revised based on GPT-5.2 review and line-by-line Goose audit.**

---

## ⚠️ KEY CORRECTIONS FROM GPT-5.2 REVIEW

### ✅ What GPT-5.2 Got Right (and we must fix)

1. **Playwright MCP package name** — Our examples used `@anthropic/playwright-mcp` but Goose docs specify:
   ```
   npx -y @playwright/mcp@latest
   ```
   All config examples must use `@playwright/mcp@latest`.

2. **Goose already has robust subagent plumbing** — Do NOT create a parallel "AgentRegistry":
   - `crates/goose/src/agents/subagent_tool.rs` — full tool with params, settings, extensions
   - `crates/goose/src/agents/subagent_handler.rs` — task execution with cancellation tokens
   - Recipe-based subrecipes with parameters
   - **Action:** Build specialist runners as a thin layer ON TOP of existing subagent system

3. **StateGraph exists only in docs/** — It's proposed architecture, NOT implemented in Rust yet
   - The `CODE → TEST → FIX` loop needs to be built from scratch
   - Keep it minimal at first; don't over-engineer the node system

4. **Test parsing is incomplete** — Tree-sitter parsing exists for code structure, but:
   - NO pytest JSON output parsing
   - NO jest JSON output parsing
   - Need resilient parsing that falls back to raw output

### ✅ What Goose Already Has (leverage, don't reinvent)

| Component                | Location                                  | Status                           |
| ------------------------ | ----------------------------------------- | -------------------------------- |
| Subagent system          | `subagent_tool.rs`, `subagent_handler.rs` | ✅ Production-ready               |
| Security threat patterns | `security/patterns.rs`                    | ✅ 40+ patterns, risk levels      |
| Tree-sitter code parsing | `goose-mcp/src/developer/analyze/`        | ✅ 9 languages                    |
| Extension manager        | `agents/extension_manager.rs`             | ✅ MCP client orchestration       |
| Permission system        | `security/`                               | ✅ Pattern matching + risk levels |

---

## 0) Scope

### In scope
- **Codebase audit (Rust + Desktop/UI + docs)**
  - `crates/goose/` (agent loop, extensions/MCP plumbing, permissions, session mgmt)
  - `crates/goose-mcp/` (tools, parsers, developer/editor APIs)
  - `crates/goose-cli/`, `crates/goose-server/`, `ui/desktop/`
  - `docs/` and `documentation/` Markdown (accuracy vs implementation)
- **Integration design + implementation plan** for:
  - **OpenHands** (sandboxed heavy-lift execution)
  - **Aider** (surgical editing + git-safe diffs)
  - **PydanticAI** (typed tool-call guardrails + approval)
  - **PraisonAI** (QA swarm / reviewer agents)
  - **LangGraph** (durable state + checkpoint/resume)

### Out of scope
- Vendoring entire upstream repos into `crates/`
- Building a full OpenHands UI inside Goose

---

## 1) Current Architecture (VERIFIED)

### Goose MCP Bus (production-ready)
```
crates/goose/src/agents/
├── extension_manager.rs      # MCP client orchestration
├── extension.rs              # ExtensionConfig enum (Stdio, Http, Builtin)
├── subagent_tool.rs          # Full subagent tool implementation
├── subagent_handler.rs       # Task execution with cancellation
└── subagent_task_config.rs   # Provider/model/extension overrides
```

### Security Infrastructure (use this!)
```rust
// crates/goose/src/security/patterns.rs - ALREADY EXISTS
pub const THREAT_PATTERNS: &[ThreatPattern] = &[
    ThreatPattern { name: "rm_rf_root", risk_level: RiskLevel::High, ... },
    ThreatPattern { name: "curl_bash_execution", risk_level: RiskLevel::Critical, ... },
    ThreatPattern { name: "reverse_shell", risk_level: RiskLevel::Critical, ... },
    // 40+ patterns covering filesystem, network, privilege escalation
];
```

### Design principle
- **MCP-first**: integrate external projects by exposing them as MCP tools/resources
- **Leverage existing subagent system**: don't create parallel orchestration
- **Capability-based behavior**: features activate if the corresponding MCP server is healthy

## 2) Multi-Layer Audit Plan (file-by-file, line-by-line on critical paths)

### Layer A — Repo baseline + invariants
- **Build/lint/test baseline** (record as “known good”):
  - `cargo fmt`, clippy lint script, `cargo test --workspace`
- **Dependency + security snapshot**:
  - identify high-risk dependencies, network-facing code, and shell execution paths

### Layer B — Critical path line-by-line audit (highest ROI)
- **Agent request lifecycle**
  - `crates/goose/src/agents/agent.rs` (mode selection, tool routing, retries, error surfacing)
- **MCP execution + permissions**
  - `extension_manager.rs`, `mcp_client.rs`, `permission/*`, malware check, allowlist logic
- **Session + memory surfaces**
  - `session/*`, any “recall” extensions (e.g., chat recall) and how they’re invoked
- **UI event handling (desktop app)**
  - audit button click + event bus plumbing so UI elements reliably respond to user interactions

### Layer C — Broad file-by-file audit (automated + sampled manual)
- Generate a **module inventory** with:
  - purpose, entrypoints, key structs/traits, error boundaries, IO/network touchpoints
- Flag:
  - duplicated logic
  - unsafe shell/path handling
  - hidden state / weak error messages
  - missing tests for critical flows

### Layer D — Docs/Markdown correctness audit
- For each doc:
  - mark statements as **Implemented / Partially Implemented / Proposed**
  - ensure CLI flags, config keys, and tool names match reality

### Audit artifacts we’ll produce
- `AUDIT_REPORT.md` (top findings, severity, owners, next actions)
- `INTEGRATION_GAP_MATRIX.md` (feature-by-feature “what exists / what’s missing”)
- “Critical files” annotated checklist (not code comments; a separate report)

## 3) Integration Strategy (the “seamless” part)

### What “seamless integration” means concretely
- **Single Goose UX** (CLI flags + recipes) that internally routes to MCP tools:
  - `goose --visual …` → Playwright headful mode + slow-mo
  - `goose --specialists security,performance …` → PraisonAI (or Goose-native registry) reviewer loop
  - `goose --sandbox …` → OpenHands delegated execution
  - `goose --safe-tools …` → PydanticAI validation/approval layer
  - `goose resume …` → LangGraph checkpoint restore

### Where the orchestration should live
- **Goose core**: selects mode, composes prompts, mediates permissions, persists session metadata.
- **Sidecars**: run heavyweight execution (Docker sandbox), typed validation, multi-agent swarms, durable checkpoints.

## 4) Project-by-Project Integration Workstreams

### 4.1 OpenHands (heavy-lift + sandbox)
**Goal:** Safely delegate large multi-file refactors/build-fix loops into an isolated environment.
- **Research/selection**
  - confirm best integration surface (CLI vs SDK) and whether a maintained MCP server exists
- **MCP tool surface (proposed)**
  - `sandbox_create`, `sandbox_run`, `sandbox_status`, `sandbox_export_patch`, `sandbox_merge`
- **Security controls**
  - path allowlist for mounts
  - explicit network policy (on/off)
  - never run destructive commands on host
- **Goose UX**
  - “delegation heuristic”: auto-suggest sandbox when task scope > N files or involves dependency upgrades

### 4.2 Aider (surgical diffs + git hygiene)
**Goal:** Use Aider for high-precision edits and safe diffs/commits.
- Use an existing Aider MCP server (evaluate maintained options) and standardize:
  - `aider_ai_code(prompt, files[]) -> patch/commit metadata`
- **Goose integration point**
  - StateGraph `CodeNode` can optionally call Aider for patch generation when enabled
- **Safety**
  - constrain file access (repo root)
  - enforce non-interactive execution

### 4.3 PydanticAI (typed guardrails + approvals)
**Goal:** Reduce hallucinated tool parameters and enforce structured, validated tool calls.
- Implement as **a guardrail sidecar** that:
  - reads Goose tool schemas
  - validates planned tool calls before execution
  - optionally requires approval for risky operations
- **Observability**
  - integrate with OpenTelemetry-compatible tracing (Pydantic Logfire or existing stack)

### 4.4 PraisonAI (QA department / reviewer swarm)
**Goal:** Add an automated “review gate” before changes land.
- Run as MCP sidecar that exposes:
  - `review_change_set(diff, context) -> findings[] + approve/block`
  - optional `workflow_run(name, inputs)` for parallel/loop patterns
- **Goose integration**
  - before final response or before applying patches: request review
  - aggregate findings into Goose’s final output (severity + locations)

### 4.5 LangGraph (durable execution + resume)
**Goal:** Persist long-running workflows and resume after crash/restart.
- Implement a LangGraph sidecar providing:
  - `checkpoint_save(session_id, state)`
  - `checkpoint_load(session_id)`
  - `checkpoint_history(session_id)`
- **Mapping**
  - Goose session/conversation IDs map to LangGraph thread IDs
- **UX**
  - `goose resume` restores last checkpoint and continues StateGraph

## 5) Cross-Cutting Feature Targets (from your docs + requested behavior)

### Memory recall without explicit prompting
- Ensure Goose performs an **automatic recall step** (ChatRecall and/or Mem0 sidecar) during context assembly.
- Define rules:
  - recall triggers (topic similarity, active repo, recent failures)
  - max injected tokens + summarization strategy

### Real-time Playwright visual testing
- Provide a single “visual testing profile”:
  - `PLAYWRIGHT_HEADLESS=false`
  - `PLAYWRIGHT_SLOW_MO=500`
  - viewport defaults
- Ensure:
  - reruns remain headful
  - screenshots captured + linked to failures

### Test framework parsing
- Normalize outputs for pytest/jest/cargo/go into a shared `TestResult` schema.
- Use parsed results to drive precise “fix targeting” in StateGraph.

## 6) Proactive Code Quality + Automated Repair Framework (non-interactive)

### Quality gates
- Rust: `cargo fmt`, clippy lint script, `cargo test --workspace`
- UI: lint/test scripts (if `ui/desktop/` exists)
- Security: dependency audit + denylist (if adopted)

### Automated repair scripts (policy)
- Must be **fully automatic** (no interactive prompts).
- Windows-friendly execution + CRLF for `.ps1/.bat`.
- Repairs limited to:
  - formatting
  - trivial clippy autofixes
  - safe config normalization
  - docs consistency fixes

## 7) CORRECTED Implementation Phases (per GPT-5.2 guidance)

### Phase 1 — Fastest Payoff (do first)

#### 1a. Visual Playwright Config (CORRECTED)
```yaml
# extensions.yaml - CORRECT package name
playwright:
  type: stdio
  cmd: npx
  args: ["-y", "@playwright/mcp@latest"]
  env:
    PLAYWRIGHT_HEADLESS: "false"
    PLAYWRIGHT_SLOW_MO: "500"
```
- Ensure headed mode is enforced even if env vars ignored (fallback to tool options)
- Add viewport defaults: `{ width: 1280, height: 720 }`

#### 1b. StateGraph Minimal (BUILD FROM SCRATCH)
```rust
// crates/goose/src/agents/state_graph/mod.rs - NEW FILE
pub struct StateGraph {
    current_state: State,
    max_iterations: usize,
}

pub enum State {
    Code,
    Test,
    Fix,
    Done,
}

impl StateGraph {
    pub async fn run(&mut self, task: &str) -> Result<()> {
        for _ in 0..self.max_iterations {
            match self.current_state {
                State::Code => { /* generate code */ self.current_state = State::Test; }
                State::Test => { /* run tests, parse results */ 
                    if tests_pass { self.current_state = State::Done; }
                    else { self.current_state = State::Fix; }
                }
                State::Fix => { /* feed failures to fix prompt */ self.current_state = State::Test; }
                State::Done => break,
            }
        }
        Ok(())
    }
}
```
- Keep it tiny; don't over-engineer nodes until looping works reliably

### Phase 2 — Precision (test parsing)

#### 2a. Pytest JSON Parsing (NEW)
```rust
// crates/goose/src/test_parsers/pytest.rs - NEW FILE
use serde::Deserialize;

#[derive(Deserialize)]
pub struct PytestReport {
    pub tests: Vec<PytestTest>,
    #[serde(default)]
    pub summary: Option<PytestSummary>,
}

#[derive(Deserialize)]
pub struct PytestTest {
    pub nodeid: String,
    pub outcome: String,  // "passed", "failed", "skipped"
    #[serde(default)]
    pub longrepr: Option<String>,
    #[serde(default)]
    pub lineno: Option<u32>,
}

impl PytestReport {
    pub fn parse(json: &str) -> Result<Self, ParseError> {
        // Try pytest-json-report format first
        if let Ok(report) = serde_json::from_str::<Self>(json) {
            return Ok(report);
        }
        // Fallback: store raw output
        Err(ParseError::FallbackToRaw(json.to_string()))
    }
}
```
- Support `pytest-json-report` plugin output
- Fallback parsing when JSON doesn't match
- Store raw output when parsing fails

#### 2b. Jest JSON Parsing (NEW)
```rust
// crates/goose/src/test_parsers/jest.rs - NEW FILE
#[derive(Deserialize)]
pub struct JestReport {
    pub testResults: Vec<JestTestResult>,
    pub numFailedTests: u32,
    pub numPassedTests: u32,
}

#[derive(Deserialize)]
pub struct JestTestResult {
    pub name: String,
    pub status: String,  // "passed", "failed"
    #[serde(default)]
    pub message: Option<String>,
}
```

#### 2c. Unified TestResult Schema
```rust
// crates/goose/src/test_parsers/mod.rs
pub struct TestResult {
    pub file: String,
    pub line: Option<u32>,
    pub test_name: String,
    pub status: TestStatus,
    pub message: Option<String>,
    pub expected: Option<String>,
    pub actual: Option<String>,
}

pub enum TestStatus { Passed, Failed, Skipped, Error }
```

### Phase 3 — Multi-Agent (built on existing subagents)

#### 3a. Specialist Runner Wrapper (uses existing subagent system)
```rust
// crates/goose/src/agents/specialist_runner.rs - NEW FILE
use crate::agents::subagent_tool::{SubagentParams, handle_subagent_tool};

pub struct SpecialistRunner {
    specialists: Vec<SpecialistConfig>,
}

pub struct SpecialistConfig {
    pub name: String,
    pub role: String,  // "security", "performance", "style"
    pub subrecipe: String,
}

impl SpecialistRunner {
    /// Runs specialists using EXISTING subagent infrastructure
    pub async fn run_review(&self, diff: &str) -> Vec<Finding> {
        let mut findings = Vec::new();
        for specialist in &self.specialists {
            // Use existing subagent tool - DON'T create parallel system
            let params = SubagentParams {
                instructions: Some(format!("Review this diff as a {} specialist:\n{}", specialist.role, diff)),
                subrecipe: Some(specialist.subrecipe.clone()),
                ..Default::default()
            };
            // ... invoke via existing handle_subagent_tool
        }
        findings
    }
}
```

### Phase 4 — Definition of Done Gate (CRITICAL for "no stubs")

```rust
// crates/goose/src/agents/done_gate.rs - NEW FILE
pub struct DoneGate {
    checks: Vec<Box<dyn DoneCheck>>,
}

pub trait DoneCheck: Send + Sync {
    fn name(&self) -> &str;
    fn check(&self, workspace: &Path) -> Result<CheckResult>;
}

pub struct CheckResult {
    pub passed: bool,
    pub message: String,
}

// Built-in checks:
pub struct BuildSucceeds;      // cargo build / npm run build
pub struct TestsPass;          // cargo test / npm test
pub struct LinterPasses;       // cargo fmt --check / eslint
pub struct NoStubMarkers;      // grep for TODO, STUB, unimplemented!(), todo!(), pass
pub struct NoEmptyFunctions;   // AST check for empty fn bodies
pub struct ChangedFilesUsed;   // no dead code in changed files

impl DoneGate {
    pub fn default_checks() -> Self {
        Self {
            checks: vec![
                Box::new(BuildSucceeds),
                Box::new(TestsPass),
                Box::new(LinterPasses),
                Box::new(NoStubMarkers),
            ],
        }
    }
    
    pub async fn verify(&self, workspace: &Path) -> GateResult {
        for check in &self.checks {
            let result = check.check(workspace)?;
            if !result.passed {
                return GateResult::ReEnterFix(check.name(), result.message);
            }
        }
        GateResult::Done
    }
}
```

### Phase 5 — Dangerous Ops Sandbox Policy (even without OpenHands)

Leverage existing `security/patterns.rs`:
```rust
// In agent.rs or shell execution path
use crate::security::patterns::{PatternMatcher, RiskLevel};

pub async fn execute_command(&self, cmd: &str) -> Result<()> {
    let matcher = PatternMatcher::new();
    let matches = matcher.scan_for_patterns(cmd);
    
    if matcher.has_critical_threats(&matches) {
        // BLOCK or require explicit sandbox
        return Err(anyhow!("Command blocked: {:?}", matches));
    }
    
    if matches.iter().any(|m| m.threat.risk_level >= RiskLevel::High) {
        // Require user approval
        self.request_approval(cmd, &matches).await?;
    }
    
    // Execute
    ...
}
```

## 8) Verification Criteria
- Every integration has:
  - Health check endpoint/command
  - Minimal demo command
  - Graceful failure when tool missing
- Goose workspace build remains clean:
  - `cargo build --workspace`
  - `cargo test --workspace`
  - `./scripts/clippy-lint.sh`

## 9) Deterministic Logging + Artifacts

Each run should produce a "run folder":
```
.goose/runs/<session_id>/
├── screenshots/          # Playwright captures
├── test_results.json     # Parsed test output
├── diffs/                # Generated patches
├── command_log.jsonl     # All shell commands + outputs
├── done_gate_report.json # Which checks passed/failed
└── summary.md            # Human-readable summary
```

## 10) DECISIONS (CONFIRMED)

| Question             | Decision                                                         |
| -------------------- | ---------------------------------------------------------------- |
| Integration approach | **MCP-only** — do not vendor into Goose core                     |
| UI priority          | **CLI-first** — ship working loop + logs first; UI toggles later |
| OpenHands sandbox    | **Local Docker only** — no remote providers                      |
| Approval policy      | **Three presets** (see below)                                    |

---

## 11) Approval Presets (CRITICAL)

Three presets so you never have to think:

### SAFE (default)
```rust
pub struct SafeMode;

impl ApprovalPolicy for SafeMode {
    fn requires_approval(&self, cmd: &str, context: &ExecutionContext) -> bool {
        let matcher = PatternMatcher::new();
        let matches = matcher.scan_for_patterns(cmd);
        // Only require approval for destructive/high-risk patterns
        matches.iter().any(|m| m.threat.risk_level >= RiskLevel::High)
    }
    
    fn auto_block(&self, cmd: &str) -> bool {
        let matcher = PatternMatcher::new();
        matcher.has_critical_threats(&matcher.scan_for_patterns(cmd))
    }
}
```
- Uses existing `security/patterns.rs` threat patterns
- Auto-blocks: `rm -rf /`, reverse shells, privilege escalation
- Requires approval: high-risk patterns (network access, process manipulation)
- Auto-approves: safe commands (ls, cat, cargo build, npm install, etc.)

### PARANOID
```rust
pub struct ParanoidMode;

impl ApprovalPolicy for ParanoidMode {
    fn requires_approval(&self, _cmd: &str, _context: &ExecutionContext) -> bool {
        true  // EVERY command requires approval
    }
    
    fn auto_block(&self, cmd: &str) -> bool {
        let matcher = PatternMatcher::new();
        matcher.has_critical_threats(&matcher.scan_for_patterns(cmd))
    }
}
```
- Every shell command requires explicit user approval
- Still auto-blocks critical threats (can't approve `rm -rf /`)
- For maximum control / untrusted tasks

### AUTOPILOT
```rust
pub struct AutopilotMode;

impl ApprovalPolicy for AutopilotMode {
    fn requires_approval(&self, cmd: &str, context: &ExecutionContext) -> bool {
        match context.environment {
            Environment::DockerSandbox => false,  // Auto-approve inside sandbox
            Environment::RealFilesystem => {
                // Same as SAFE mode on real filesystem
                let matcher = PatternMatcher::new();
                let matches = matcher.scan_for_patterns(cmd);
                matches.iter().any(|m| m.threat.risk_level >= RiskLevel::High)
            }
        }
    }
    
    fn auto_block(&self, cmd: &str) -> bool {
        // NEVER auto-block inside Docker sandbox
        // (the sandbox IS the protection)
        false
    }
}
```
- **Inside Docker sandbox**: auto-approve everything (sandbox is the protection)
- **On real filesystem**: same as SAFE mode
- For unattended execution with OpenHands

### CLI Usage
```bash
# Default (SAFE)
goose run --task "fix the tests"

# Paranoid mode
goose run --task "fix the tests" --approval-mode paranoid

# Autopilot (requires sandbox)
goose run --task "fix the tests" --approval-mode autopilot --sandbox
```

### Config File
```yaml
# ~/.config/goose/config.yaml
approval_mode: safe  # safe | paranoid | autopilot

# Override per-project
# .goose/config.yaml
approval_mode: paranoid  # this project is sensitive
```

---

## Summary: What Changed from GPT-5.2 Review

| Issue              | Original Plan               | Corrected                               |
| ------------------ | --------------------------- | --------------------------------------- |
| Playwright package | `@anthropic/playwright-mcp` | `@playwright/mcp@latest`                |
| AgentRegistry      | New parallel system         | Use existing subagent system            |
| StateGraph         | Assumed implemented         | Build from scratch, keep minimal        |
| Test parsing       | Assumed complete            | Add pytest/jest JSON parsers            |
| Done Gate          | Not mentioned               | Added as enforcement mechanism          |
| Dangerous ops      | Not enforced                | Leverage existing security/patterns.rs  |
| Run artifacts      | Not mentioned               | Added deterministic logging             |
| Approval modes     | Single policy               | **SAFE / PARANOID / AUTOPILOT** presets |

---

## 12) Implementation Order (CLI-First)

### Week 1: Core Loop
1. **StateGraph minimal** — `CODE → TEST → FIX` loop in Rust
2. **Approval presets** — `ApprovalPolicy` trait + 3 implementations
3. **Run artifacts** — `.goose/runs/<session_id>/` folder structure

### Week 2: Test Precision
4. **Pytest JSON parser** — with fallback to raw output
5. **Jest JSON parser** — with fallback to raw output
6. **Unified TestResult** — normalized schema for fix targeting

### Week 3: Safety + Quality
7. **Done Gate** — build/test/lint/no-stubs checks
8. **Dangerous ops integration** — wire `security/patterns.rs` into shell execution
9. **CLI flags** — `--approval-mode`, `--sandbox`

### Week 4: MCP Sidecars
10. **Playwright MCP config** — `@playwright/mcp@latest` with headful defaults
11. **OpenHands MCP** — local Docker sandbox only
12. **Aider MCP** — surgical diffs (optional)

### Week 5+: Polish
13. **Specialist runner** — thin wrapper on existing subagent system
14. **LangGraph checkpoint** — session persistence
15. **PydanticAI guardrails** — typed tool validation
16. **Desktop UI toggles** — after CLI is stable

---

## 13) Files to Create

| File                                           | Purpose                               |
| ---------------------------------------------- | ------------------------------------- |
| `crates/goose/src/agents/state_graph/mod.rs`   | Minimal CODE→TEST→FIX loop            |
| `crates/goose/src/agents/state_graph/state.rs` | CodeTestFixState struct               |
| `crates/goose/src/approval/mod.rs`             | ApprovalPolicy trait                  |
| `crates/goose/src/approval/presets.rs`         | SafeMode, ParanoidMode, AutopilotMode |
| `crates/goose/src/test_parsers/mod.rs`         | TestResult + TestStatus               |
| `crates/goose/src/test_parsers/pytest.rs`      | Pytest JSON parsing                   |
| `crates/goose/src/test_parsers/jest.rs`        | Jest JSON parsing                     |
| `crates/goose/src/agents/done_gate.rs`         | DoneGate + DoneCheck trait            |
| `crates/goose/src/agents/specialist_runner.rs` | Thin wrapper on subagents             |

---

## Ready to Implement

All decisions confirmed. Plan is complete. Ready to begin with:

1. **StateGraph minimal** (`crates/goose/src/agents/state_graph/mod.rs`)
2. **Approval presets** (`crates/goose/src/approval/`)

Awaiting your go-ahead to start implementation.
