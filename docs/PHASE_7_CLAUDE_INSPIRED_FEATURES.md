# Phase 7: Claude-Inspired Advanced Features

## Executive Summary

Based on comprehensive research of Anthropic's Claude Code, Claude Agent SDK, and the claude-code-hooks-mastery repository, this document outlines the **Phase 7 implementation roadmap** for Goose. These features will transform Goose into a **game-changing enterprise AI agent platform** with capabilities matching or exceeding Claude Code.

**Key Findings:**
- Claude Code has 13 lifecycle hooks with deterministic enforcement
- First-class Task system with DAG dependencies and parallel execution
- Builder/Validator agent pairing for quality assurance
- Skills as installable enforcement modules
- Tool Search Tool for dynamic tool discovery (85% token reduction)
- Programmatic Tool Calling for structured outputs

---

## 🔴 CRITICAL MISSING FEATURES

### 1. Task Graph System (HIGH PRIORITY)

**What Claude Has:**
- `TaskCreate`, `TaskList`, `TaskGet`, `TaskUpdate` operations
- DAG-based dependencies with blocking/unblocking
- Parallel execution with concurrency limits
- Event streaming (queued → running → done/failed)
- Cross-session task persistence

**What Goose Needs:**

```rust
// crates/goose/src/tasks/mod.rs

pub struct TaskGraph {
    tasks: HashMap<TaskId, Task>,
    dependencies: HashMap<TaskId, Vec<TaskId>>,
    concurrency_limit: usize,
    event_tx: broadcast::Sender<TaskEvent>,
}

pub struct Task {
    id: TaskId,
    subject: String,
    description: String,
    owner: Option<AgentRole>,      // Builder, Validator, etc.
    status: TaskStatus,
    dependencies: Vec<TaskId>,
    blockers: Vec<TaskId>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    result: Option<TaskResult>,
}

pub enum TaskStatus {
    Queued,
    Blocked,
    Running,
    Done,
    Failed,
    Cancelled,
}

pub enum TaskEvent {
    Created(TaskId),
    StatusChanged { id: TaskId, old: TaskStatus, new: TaskStatus },
    DependencyUnblocked { id: TaskId, unblocked_by: TaskId },
    Completed { id: TaskId, result: TaskResult },
    Failed { id: TaskId, error: String },
}

impl TaskGraph {
    pub fn create(&mut self, task: Task) -> TaskId;
    pub fn list(&self) -> Vec<&Task>;
    pub fn get(&self, id: TaskId) -> Option<&Task>;
    pub fn update(&mut self, id: TaskId, update: TaskUpdate) -> Result<()>;
    pub fn run_parallel(&mut self, executor: impl TaskExecutor) -> JoinHandle<()>;
    pub fn subscribe(&self) -> broadcast::Receiver<TaskEvent>;
}
```

**Storage Options:**
- SQLite (default for persistence)
- In-memory (for testing)
- Shared filesystem JSON (for multi-agent coordination)

---

### 2. Hook System with Deterministic Validators (HIGH PRIORITY)

**What Claude Has:**
- 13 lifecycle events: `SessionStart`, `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `PermissionRequest`, `Notification`, `SubagentStart`, `SubagentStop`, `Stop`, `PreCompact`, `SessionEnd`, `Setup`
- Exit code flow control (0=success, 2=blocking error)
- JSON output for decisions (`approve`, `block`, `ask`)
- Async hooks for background operations

**What Goose Needs:**

```rust
// crates/goose/src/hooks/mod.rs

pub struct HookManager {
    handlers: HashMap<HookEvent, Vec<HookHandler>>,
    log_dir: PathBuf,
    run_id: String,
}

#[derive(Clone, Debug)]
pub enum HookEvent {
    SessionStart { source: SessionSource },
    UserPromptSubmit { prompt: String },
    PreToolUse { tool_name: String, tool_input: Value },
    PostToolUse { tool_name: String, tool_input: Value, tool_response: Value },
    PostToolUseFailure { tool_name: String, error: String },
    PermissionRequest { tool_name: String, tool_input: Value },
    SubagentStart { agent_id: String, agent_type: String },
    SubagentStop { agent_id: String },
    Stop { stop_hook_active: bool },
    PreCompact { trigger: CompactTrigger },
    SessionEnd { reason: SessionEndReason },
}

pub struct HookHandler {
    event_type: HookEvent,
    matcher: Option<HookMatcher>,      // Filter by tool name, etc.
    command: String,                    // Shell command or script path
    timeout: Duration,
    async_execution: bool,
}

pub struct HookResult {
    exit_code: i32,
    stdout: String,
    stderr: String,
    decision: Option<HookDecision>,
    additional_context: Option<String>,
}

pub enum HookDecision {
    Approve { reason: String },
    Block { reason: String },
    Ask { reason: String },
    Continue,
}

impl HookManager {
    pub async fn fire(&self, event: HookEvent) -> Vec<HookResult>;
    pub fn should_block(&self, results: &[HookResult]) -> bool;
    pub fn get_context(&self, results: &[HookResult]) -> String;
}
```

**Per-Hook Logging:**
```
logs/runs/<run_id>/hooks/
├── session_start/
│   ├── hook.log
│   ├── hook.jsonl
│   └── inputs.json
├── pre_tool_use/
│   ├── hook.log
│   ├── hook.jsonl
│   └── evidence/
├── post_tool_use/
│   └── ...
└── stop/
    ├── hook.log
    ├── hook.jsonl
    └── verdict.json
```

---

### 3. Deterministic Validators (HIGH PRIORITY)

**What Claude Has:**
- Ruff validator for Python linting
- Type checker (ty) for Python
- File existence validators
- File content validators (required sections)
- No-TODOs validator
- Build validators (cargo build, cargo test)

**What Goose Needs:**

```rust
// crates/goose/src/validators/mod.rs

pub trait Validator: Send + Sync {
    fn name(&self) -> &str;
    fn validate(&self, context: &ValidationContext) -> ValidationResult;
}

pub struct ValidationContext {
    pub changed_files: Vec<PathBuf>,
    pub tool_name: String,
    pub tool_input: Value,
    pub working_dir: PathBuf,
    pub project_type: ProjectType,
}

pub struct ValidationResult {
    pub ok: bool,
    pub fail_reason: Option<String>,
    pub actions_taken: Vec<String>,
    pub evidence_paths: Vec<PathBuf>,
    pub next_recommendation: Option<String>,
}

// Built-in validators
pub struct RustValidator;      // cargo build, cargo test, cargo clippy, cargo fmt --check
pub struct PythonValidator;    // ruff, mypy/pyright
pub struct JavaScriptValidator; // eslint, tsc
pub struct NoTodosValidator;   // ripgrep TODO/FIXME/XXX
pub struct FileExistsValidator;
pub struct FileContainsValidator;
pub struct SecurityValidator;  // secret scanning, dangerous patterns
```

---

### 4. Skills Pack Architecture (HIGH PRIORITY)

**What Claude Has:**
- Skills as markdown files with YAML frontmatter
- Prompt templates
- Runnable validators
- Default gates (what must pass before task completion)
- Auto-loading from `.claude/skills/`

**What Goose Needs:**

```yaml
# .goose/skills/compound-engineering/skill.yaml
name: compound-engineering
description: Enterprise team-based build/validate workflow
version: 1.0.0

prompts:
  - plan_with_team.md
  - build_with_team.md
  - validate_with_team.md

validators:
  - validate_new_file.py
  - validate_file_contains.py
  - validate_no_todos.py
  - validate_build.sh
  - validate_artifacts.py

gates:
  pre_complete:
    - cargo build --release
    - cargo test --no-fail-fast
    - cargo clippy -D warnings
    - cargo fmt --check
  post_tool_use:
    - validate_no_todos.py

hooks:
  stop:
    - command: validate_build.sh
    - command: validate_artifacts.py
```

```rust
// crates/goose/src/skills/mod.rs

pub struct SkillPack {
    pub name: String,
    pub description: String,
    pub version: String,
    pub prompts: Vec<PromptTemplate>,
    pub validators: Vec<Box<dyn Validator>>,
    pub gates: GateConfig,
    pub hooks: Vec<HookConfig>,
}

pub struct SkillManager {
    skills: HashMap<String, SkillPack>,
    skill_dirs: Vec<PathBuf>,
}

impl SkillManager {
    pub fn install(&mut self, skill_path: &Path) -> Result<()>;
    pub fn load(&mut self, name: &str) -> Result<&SkillPack>;
    pub fn list(&self) -> Vec<&SkillPack>;
    pub fn get_gates(&self, name: &str) -> Option<&GateConfig>;
}
```

---

### 5. Builder/Validator Agent Pairing (HIGH PRIORITY)

**What Claude Has:**
- Builder agent: Full tool access, implements features
- Validator agent: Read-only, verifies work
- Mandatory pairing enforcement
- Validator has authority to fail/rollback

**What Goose Needs:**

```rust
// crates/goose/src/agents/team/mod.rs

pub struct AgentTeam {
    pub builder: Box<dyn SpecialistAgent>,
    pub validator: Box<dyn SpecialistAgent>,
    pub orchestrator: AgentOrchestrator,
}

pub struct BuilderAgent {
    pub tools: Vec<Tool>,  // All tools including Write, Edit, Bash
    pub validators: Vec<Box<dyn Validator>>,  // Ruff, type checker on .py files
}

pub struct ValidatorAgent {
    pub tools: Vec<Tool>,  // Read-only: Read, Glob, Grep (no Write/Edit)
    pub acceptance_criteria: Vec<String>,
}

impl AgentTeam {
    pub async fn execute_with_validation(
        &self,
        task: &Task,
    ) -> Result<ValidationResult> {
        // 1. Builder implements
        let build_result = self.builder.execute(task).await?;
        
        // 2. Validator verifies
        let validation = self.validator.validate(&build_result).await?;
        
        // 3. If validation fails, feedback to builder
        if !validation.ok {
            return Err(ValidationError::new(validation.fail_reason));
        }
        
        Ok(validation)
    }
}
```

---

### 6. Tool Search Tool (MEDIUM PRIORITY)

**What Claude Has:**
- Dynamic tool discovery instead of loading all upfront
- 85% reduction in token usage
- Only loads tools needed for current task
- Significant accuracy improvements (49% → 74% for Opus 4)

**What Goose Needs:**

```rust
// crates/goose/src/tools/search.rs

pub struct ToolSearchTool {
    tool_registry: Arc<ToolRegistry>,
    embeddings: Option<EmbeddingModel>,
}

impl ToolSearchTool {
    pub fn search(&self, query: &str, limit: usize) -> Vec<ToolDefinition>;
    pub fn get_relevant_tools(&self, task: &str) -> Vec<Tool>;
}

// Instead of loading all 50+ MCP tools (55K tokens):
// - Load only ToolSearchTool (~500 tokens)
// - Discover 3-5 relevant tools on-demand (~3K tokens)
// - 85% token reduction
```

---

### 7. Programmatic Tool Calling (MEDIUM PRIORITY)

**What Claude Has:**
- Structured outputs for tool calls
- Schema validation
- Tool use examples for parameter accuracy

**What Goose Needs:**

```rust
// crates/goose/src/tools/programmatic.rs

pub struct ProgrammaticToolCall {
    pub tool_name: String,
    pub schema: JsonSchema,
    pub examples: Vec<ToolExample>,
}

pub struct ToolExample {
    pub input: Value,
    pub expected_output: Option<Value>,
    pub description: String,
}
```

---

### 8. Template Metaprompts (MEDIUM PRIORITY)

**What Claude Has:**
- Prompts that generate prompts
- Repeatable format generating plan docs, team rosters, task lists
- Self-validating with embedded hooks

**What Goose Needs:**

```markdown
# .goose/prompts/plan_with_team.md
---
name: plan_with_team
description: Create a plan with team-based build/validate workflow
hooks:
  stop:
    - command: validate_new_file.py specs/*.md
    - command: validate_file_contains.py
---

## Objective
Create a detailed implementation plan with team orchestration.

## Output Format
### {{PLAN_NAME}}
**Task:** {{TASK_DESCRIPTION}}
**Objective:** {{OBJECTIVE}}

### Team Orchestration
{{TEAM_MEMBERS}}

### Step-by-Step Tasks
{{TASKS}}

### Validation Commands
{{VALIDATION_COMMANDS}}
```

---

## 🟡 IMPORTANT ENHANCEMENTS

### 9. Artifact-First Auditing

**Every task produces:**
- Execution logs
- Validator evidence
- Machine-readable JSON summaries
- Human-readable markdown reports

```
logs/runs/<run_id>/
├── run_index.json           # Pointers to all artifacts
├── run_summary.md           # Human-readable summary
├── tasks/
│   ├── task_001/
│   │   ├── execution.log
│   │   ├── validator_evidence/
│   │   └── result.json
├── hooks/
│   └── ...
└── audit.json               # Final audit pack
```

---

### 10. Status Lines (Real-Time Terminal Display)

**What Claude Has:**
- 9 status line variants
- Git info, cost tracking, context window usage
- Token stats, session duration, agent sessions

**What Goose Needs:**

```rust
// crates/goose-cli/src/status_line.rs

pub struct StatusLine {
    pub model: String,
    pub cost: f64,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub context_usage: f32,
    pub session_duration: Duration,
    pub git_branch: String,
    pub latest_prompt: String,
}

impl StatusLine {
    pub fn render(&self) -> String {
        format!(
            "🤖 {} | 💰 ${:.4} | 📊 {}% | ⏱️ {} | 🌿 {}",
            self.model,
            self.cost,
            (self.context_usage * 100.0) as u32,
            format_duration(self.session_duration),
            self.git_branch
        )
    }
}
```

---

### 11. Output Styles

**What Claude Has:**
- genui (beautiful HTML)
- table-based, yaml-structured, bullet-points
- ultra-concise, html-structured, markdown-focused
- tts-summary (audio feedback)

**What Goose Needs:**

```rust
// crates/goose/src/output/styles.rs

pub enum OutputStyle {
    Default,
    TableBased,
    YamlStructured,
    BulletPoints,
    UltraConcise,
    HtmlStructured,
    MarkdownFocused,
}

impl OutputStyle {
    pub fn format(&self, content: &str) -> String;
}
```

---

### 12. Slash Commands

**What Claude Has:**
- Custom commands in `.claude/commands/*.md`
- `/plan_w_team`, `/build`, `/prime`, etc.

**What Goose Needs:**

```rust
// crates/goose-cli/src/commands/slash.rs

pub struct SlashCommandManager {
    commands: HashMap<String, SlashCommand>,
}

pub struct SlashCommand {
    pub name: String,
    pub description: String,
    pub prompt_template: String,
    pub hooks: Vec<HookConfig>,
}
```

---

## 📋 IMPLEMENTATION ACTION PLAN

### Phase 7.1: Task Graph System (2-3 weeks)
1. Create `crates/goose/src/tasks/` module
2. Implement `TaskGraph` with DAG dependencies
3. Add parallel execution with concurrency limits
4. Add event streaming
5. Add SQLite persistence
6. Create CLI commands: `goose task create/list/get/update`
7. Write comprehensive tests

### Phase 7.2: Hook System (2-3 weeks)
1. Create `crates/goose/src/hooks/` module
2. Implement all 13 lifecycle events
3. Add hook handler execution with timeouts
4. Implement exit code flow control
5. Add JSON decision output parsing
6. Implement per-hook logging with correlation IDs
7. Write comprehensive tests

### Phase 7.3: Validators (1-2 weeks)
1. Create `crates/goose/src/validators/` module
2. Implement `Validator` trait
3. Add built-in validators (Rust, Python, JS, security)
4. Integrate with hook system (PostToolUse)
5. Add validator evidence collection
6. Write comprehensive tests

### Phase 7.4: Skills Pack (1-2 weeks)
1. Enhance `crates/goose/src/agents/skills_extension.rs`
2. Add YAML skill pack format
3. Implement skill installation/loading
4. Add gate configuration
5. Create `compound-engineering` skill pack
6. Write comprehensive tests

### Phase 7.5: Builder/Validator Teams (1-2 weeks)
1. Create `crates/goose/src/agents/team/` module
2. Implement `BuilderAgent` and `ValidatorAgent`
3. Add mandatory pairing enforcement
4. Integrate with task graph
5. Write comprehensive tests

### Phase 7.6: Tool Search & Programmatic Calling (1 week)
1. Implement `ToolSearchTool`
2. Add tool examples support
3. Implement programmatic tool calling
4. Write comprehensive tests

### Phase 7.7: CLI & UX Enhancements (1 week)
1. Add status line support
2. Implement output styles
3. Add slash commands
4. Enhance workflow commands
5. Write comprehensive tests

---

## 🎯 SUCCESS METRICS

| Metric | Target |
|--------|--------|
| Task Graph Tests | 30+ |
| Hook System Tests | 40+ |
| Validator Tests | 25+ |
| Skills Pack Tests | 20+ |
| Token Reduction | 50%+ with Tool Search |
| Build Validation | 100% before completion |
| Zero Stubs Policy | Enforced by validators |

---

## 📁 NEW FILES TO CREATE

```
crates/goose/src/
├── tasks/
│   ├── mod.rs
│   ├── graph.rs
│   ├── persistence.rs
│   └── events.rs
├── hooks/
│   ├── mod.rs
│   ├── manager.rs
│   ├── events.rs
│   ├── handlers.rs
│   └── logging.rs
├── validators/
│   ├── mod.rs
│   ├── rust.rs
│   ├── python.rs
│   ├── javascript.rs
│   ├── security.rs
│   └── content.rs
├── skills/
│   ├── mod.rs
│   ├── pack.rs
│   └── manager.rs
├── agents/team/
│   ├── mod.rs
│   ├── builder.rs
│   └── validator.rs
└── tools/
    ├── search.rs
    └── programmatic.rs

crates/goose-cli/src/
├── status_line.rs
├── output_styles.rs
└── commands/
    ├── slash.rs
    └── task.rs

.goose/skills/compound-engineering/
├── skill.yaml
├── prompts/
│   ├── plan_with_team.md
│   ├── build_with_team.md
│   └── validate_with_team.md
└── validators/
    ├── validate_new_file.py
    ├── validate_file_contains.py
    ├── validate_no_todos.py
    ├── validate_build.sh
    └── validate_artifacts.py
```

---

## 🔗 REFERENCES

- [Claude Code Hooks Documentation](https://code.claude.com/docs/en/hooks)
- [Claude Agent SDK Overview](https://platform.claude.com/docs/en/agent-sdk/overview)
- [Advanced Tool Use](https://www.anthropic.com/engineering/advanced-tool-use)
- [claude-code-hooks-mastery Repository](https://github.com/disler/claude-code-hooks-mastery)
- [Building Agents with Claude Agent SDK](https://claude.com/blog/building-agents-with-the-claude-agent-sdk)

---

**Document Version:** 1.0.0
**Created:** February 2, 2026
**Status:** Ready for Implementation
