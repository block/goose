# TODO: RLM (Recursive Language Models) Implementation for Goose

**Reference Paper**: [Recursive Language Models (arXiv:2512.24601)](https://arxiv.org/abs/2512.24601)
**GitHub Issue**: [#6651](https://github.com/block/goose/issues/6651)

## Overview

Implement RLM support in Goose to handle arbitrarily long prompts by treating them as external environment variables that can be programmatically examined, decomposed, and recursively processed through sub-agent calls.

### Key Benefits
- Handle inputs 100x+ beyond normal context windows
- Dramatically outperform base LLMs on long-context tasks
- Maintain comparable or lower cost per query
- Enable processing of 10M+ token inputs

---

## Goose Architecture Context

> **Important**: Goose is written in **Rust**, not Python. The implementation must use Rust idioms and integrate with the existing crate structure.

### Relevant Crates & Files

| Component | Location | Purpose |
|-----------|----------|---------|
| Agent Core | `crates/goose/src/agents/agent.rs` | Main agent loop, tool dispatch |
| Extensions | `crates/goose/src/agents/extension.rs` | Extension types (Platform, Stdio, etc.) |
| Extension Manager | `crates/goose/src/agents/extension_manager.rs` | Loads/manages extensions |
| Session Manager | `crates/goose/src/session/session_manager.rs` | Session lifecycle, SQLite storage |
| Config | `crates/goose/src/config/` | YAML-based configuration |
| Providers | `crates/goose/src/providers/` | LLM provider implementations |
| Code Execution | `crates/goose/src/agents/code_execution_extension.rs` | Existing JS sandbox (can reference) |
| Sub-Agent Tool | `crates/goose/src/agents/subagent_tool.rs` | Existing sub-agent support |

### Existing Infrastructure to Leverage

1. **Sub-Agent System**: Goose already has `SUBAGENT_TOOL_NAME` for spawning sub-agents
2. **Code Execution Extension**: Existing `code_execution_extension` runs JavaScript in sandbox
3. **Platform Extensions**: Internal extensions with direct agent access
4. **Session Types**: `SessionType::SubAgent` already exists
5. **Provider Abstraction**: Can use any configured LLM provider

---

## Architecture Summary

```
User Input (large context)
    ↓
Context Store (filesystem)
    ↓
REPL Environment (Python via subprocess or JS sandbox)
    ↓
Root Agent (with RLM system prompt)
    ↓
├── Code Execution (filter/chunk context)
├── Sub-Agent Calls (recursive LLM queries via existing subagent_tool)
└── Variable Storage → Final Answer
```

---

## Implementation Checklist

### Phase 1: Core Components

#### 1. Context Storage System
**File**: `crates/goose/src/rlm/context_store.rs`

- [ ] Create `ContextStore` struct
  - [ ] `store_context(content: &str) -> Result<ContextMetadata>` - Write context to file
  - [ ] `get_metadata() -> ContextMetadata` - Return length, path, chunk info
  - [ ] `read_context() -> Result<String>` - Load context from storage
  - [ ] `read_slice(start: usize, end: usize) -> Result<String>` - Load partial context
  - [ ] `get_chunk_boundaries(chunk_size: usize) -> Vec<(usize, usize)>` - Calculate chunk boundaries

**Key Features**:
- Store context as plain text file in session working directory
- Return metadata: `ContextMetadata { length, path, chunk_count, chunk_boundaries }`
- Support chunking by characters (default ~500K per chunk)

```rust
pub struct ContextStore {
    session_dir: PathBuf,
    context_file: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMetadata {
    pub length: usize,
    pub path: PathBuf,
    pub chunk_count: usize,
    pub chunk_boundaries: Vec<(usize, usize)>,
}

impl ContextStore {
    pub fn new(session_dir: PathBuf) -> Self {
        let context_file = session_dir.join("rlm_context.txt");
        Self { session_dir, context_file }
    }

    pub async fn store_context(&self, content: &str) -> Result<ContextMetadata> {
        // Write to file, calculate chunks
        todo!()
    }

    pub async fn read_slice(&self, start: usize, end: usize) -> Result<String> {
        // Read partial content
        todo!()
    }
}
```

---

#### 2. RLM Platform Extension
**File**: `crates/goose/src/agents/rlm_extension.rs`

- [ ] Create `RlmClient` implementing `McpClientTrait`
  - [ ] `read_context_slice` tool - Read portion of stored context
  - [ ] `get_context_metadata` tool - Get context info (length, chunks)
  - [ ] `store_variable` tool - Store intermediate results
  - [ ] `get_variable` tool - Retrieve stored variables
  - [ ] `llm_query` tool - Call sub-agent with context chunk (wraps existing subagent_tool)
  - [ ] `finalize` tool - Signal completion with final answer

**Key Features**:
- Expose context as tools rather than injecting into prompt
- Leverage existing `subagent_tool` for recursive calls
- Track variable state in memory (HashMap)

```rust
pub const EXTENSION_NAME: &str = "rlm_extension";

pub struct RlmClient {
    context_store: ContextStore,
    variables: Arc<Mutex<HashMap<String, String>>>,
    ctx: PlatformExtensionContext,
}

impl RlmClient {
    pub fn new(ctx: PlatformExtensionContext, session_dir: PathBuf) -> Result<Self> {
        Ok(Self {
            context_store: ContextStore::new(session_dir),
            variables: Arc::new(Mutex::new(HashMap::new())),
            ctx,
        })
    }
}

impl McpClientTrait for RlmClient {
    // Implement tool handlers
}
```

---

#### 3. RLM System Prompts
**File**: `crates/goose/src/rlm/prompts.rs`

- [ ] Define `RLM_SYSTEM_PROMPT` constant
- [ ] Include instructions for:
  - [ ] Using `read_context_slice` to access context
  - [ ] Using `llm_query` for recursive sub-agent calls
  - [ ] Chunking strategies (aim for ~500K chars per sub-call)
  - [ ] Using `finalize` tool for final answers
  - [ ] Code execution patterns (regex, filtering, aggregation)

```rust
pub const RLM_SYSTEM_PROMPT: &str = r#"
You are tasked with answering a query with associated context.
The context is stored externally and can be accessed using the provided tools.

## Available Tools

### Context Access
- `get_context_metadata()` - Returns context length, chunk count, and boundaries
- `read_context_slice(start, end)` - Read characters from position start to end

### Recursive Queries
- `llm_query(prompt, context_slice)` - Query a sub-agent with a portion of context
  - Aim for ~500,000 characters per sub-call
  - Sub-agents have the same capabilities as you

### Variable Storage
- `store_variable(name, value)` - Store intermediate results
- `get_variable(name)` - Retrieve stored values

### Completion
- `finalize(answer)` - Return your final answer

## Strategy
1. First, call `get_context_metadata()` to understand the context size
2. If context is small enough (<500K chars), read it directly
3. For large contexts, chunk and delegate to sub-agents
4. Aggregate results and call `finalize(answer)` when done

## Important
- Never try to read more than 500K characters at once
- Use code execution for filtering/processing when helpful
- Store intermediate results in variables for later aggregation
"#;
```

---

#### 4. RLM Mode Detection & Session Handling
**File**: `crates/goose/src/rlm/mod.rs`

- [ ] Create `RlmConfig` struct for configuration
- [ ] Create `is_rlm_candidate(content: &str, config: &RlmConfig) -> bool` function
- [ ] Create `prepare_rlm_session(...)` function to set up RLM mode

```rust
pub mod context_store;
pub mod prompts;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RlmConfig {
    pub enabled: bool,
    pub context_threshold: usize,  // chars, default 100_000
    pub chunk_size: usize,         // chars, default 500_000
    pub max_iterations: u32,       // default 50
    pub max_recursion_depth: u32,  // default 1
}

impl Default for RlmConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            context_threshold: 100_000,
            chunk_size: 500_000,
            max_iterations: 50,
            max_recursion_depth: 1,
        }
    }
}

pub fn is_rlm_candidate(content: &str, config: &RlmConfig) -> bool {
    config.enabled && content.len() > config.context_threshold
}
```

---

### Phase 2: Integration with Goose

#### 5. Agent Modifications
**File**: `crates/goose/src/agents/agent.rs` (modifications)

- [ ] Add RLM config to `AgentConfig`
- [ ] Modify `reply()` to detect large context and enable RLM mode
- [ ] Add `_rlm_prepare()` helper method
  - [ ] Store context to file
  - [ ] Add RLM extension to session
  - [ ] Inject RLM system prompt

```rust
// In AgentConfig
pub struct AgentConfig {
    // ... existing fields ...
    pub rlm_config: RlmConfig,
}

// In Agent impl
impl Agent {
    async fn maybe_enable_rlm(&self, user_input: &str, session: &Session) -> Result<bool> {
        if !is_rlm_candidate(user_input, &self.config.rlm_config) {
            return Ok(false);
        }

        // Store context
        let context_store = ContextStore::new(session.working_dir.clone());
        let metadata = context_store.store_context(user_input).await?;

        // Add RLM extension
        self.add_extension(ExtensionConfig::Platform {
            name: "rlm_extension".to_string(),
            description: "RLM context access tools".to_string(),
            bundled: Some(true),
            available_tools: Vec::new(),
        }).await?;

        // Inject system prompt
        self.extend_system_prompt(format!(
            "{}\n\nContext metadata: {} characters, {} chunks",
            RLM_SYSTEM_PROMPT,
            metadata.length,
            metadata.chunk_count,
        )).await;

        Ok(true)
    }
}
```

---

#### 6. Configuration System
**File**: `crates/goose/src/config/rlm.rs`

- [ ] Add RLM configuration parsing from YAML
- [ ] Add config keys: `rlm.enabled`, `rlm.context_threshold`, etc.

```yaml
# Example config.yaml
rlm:
  enabled: true
  context_threshold: 100000
  chunk_size: 500000
  max_iterations: 50
  max_recursion_depth: 1
```

---

#### 7. CLI Enhancements (Optional)
**File**: `crates/goose-cli/src/commands/session.rs`

- [ ] Add `--rlm` flag to force RLM mode
- [ ] Add `--rlm-threshold` to override threshold
- [ ] Show RLM status in session info

```bash
# Usage examples
goose run --rlm --input large_file.txt
goose run --rlm-threshold 50000 --input medium_file.txt
```

---

### Phase 3: Testing & Validation

#### 8. Unit Tests
**Files**: `crates/goose/src/rlm/tests/`

- [ ] `test_context_store.rs`
  - [ ] Test storing/retrieving context
  - [ ] Test chunk boundary calculation
  - [ ] Test large context handling (1M+ chars)
  - [ ] Test slice reading
- [ ] `test_rlm_extension.rs`
  - [ ] Test tool execution
  - [ ] Test variable storage
  - [ ] Test llm_query delegation

---

#### 9. Integration Tests
**File**: `crates/goose/tests/rlm_integration.rs`

- [ ] Test with paper's benchmark patterns:
  - [ ] S-NIAH (needle in haystack)
  - [ ] Simple multi-document QA
- [ ] Test context sizes: 100K, 1M, 10M chars
- [ ] Test recursion depth limits
- [ ] Test max iteration limits

---

#### 10. Example Benchmarks
**Directory**: `examples/rlm_benchmarks/`

Create simple reproducible tests:

- [ ] `needle_in_haystack.rs` - S-NIAH style task
  ```rust
  // Generate 1M char context with hidden "magic number"
  // Query: "What is the magic number?"
  ```
- [ ] `document_qa.rs` - Multi-document question answering
  ```rust
  // 100 documents, find answer across multiple docs
  ```

---

### Phase 4: Optimization & Polish

#### 11. Performance Improvements

- [ ] **Async Sub-Agent Calls** (paper notes this is critical)
  - [ ] Parallel execution of independent sub-queries
  - [ ] Use `tokio::spawn` for concurrent sub-agent calls
- [ ] **Caching**
  - [ ] Cache sub-query results for identical inputs
  - [ ] Cache context chunks in memory
- [ ] **Smart Chunking**
  - [ ] Detect natural boundaries (paragraphs, sections)
  - [ ] Consider semantic chunking for code

---

#### 12. Monitoring & Debugging

- [ ] **Cost Tracking**
  - [ ] Track total tokens per RLM query
  - [ ] Track sub-agent call count
  - [ ] Add to session metrics
- [ ] **Logging**
  - [ ] Log each RLM iteration via `tracing`
  - [ ] Log sub-agent calls with context
  - [ ] Log final answer extraction
- [ ] **Debug Mode**
  - [ ] `--rlm-debug` flag for verbose output
  - [ ] Show recursion tree in logs

---

## Quick Start Implementation Order

**Recommended sequence for fastest MVP**:

1. [ ] Create `crates/goose/src/rlm/mod.rs` module structure
2. [ ] Implement `ContextStore` (Phase 1.1)
3. [ ] Implement `RlmClient` platform extension (Phase 1.2)
4. [ ] Define `RLM_SYSTEM_PROMPT` (Phase 1.3)
5. [ ] Add RLM detection in Agent (Phase 2.5)
6. [ ] Create simple needle-in-haystack test (Phase 3.10)
7. [ ] Test and iterate
8. [ ] Add config options (Phase 2.6)
9. [ ] Add async sub-calls (Phase 4.11)
10. [ ] Polish and document

---

## Key Design Decisions from Paper

### 1. Context as Environment Variable
- **Why**: Prevents context from overwhelming neural network
- **How**: Store in file, expose via tools instead of prompt

### 2. Recursion Depth = 1 (Default)
- **Why**: Paper found depth=1 sufficient for most tasks
- **How**: Root agent uses main model, sub-agents use same model (Goose doesn't have model switching yet)

### 3. Chunking Strategy
- **Target**: ~500K chars per sub-call (paper recommendation)
- **Balance**: Large enough to be useful, small enough to avoid context rot

### 4. Answer Detection
- **Method**: Use `finalize` tool instead of text tags
- **Why**: More reliable than parsing `FINAL()` tags from output

---

## Common Pitfalls to Avoid (from Paper Appendix A)

1. **Limited output tokens**
   - Thinking models can run out of tokens
   - Set generous output limits in provider config

2. **Synchronous sub-calls only**
   - Makes RLMs very slow
   - Implement async ASAP (Phase 4.11)

3. **Brittle answer detection**
   - Text-based `FINAL()` tag detection can fail
   - Use tool-based `finalize` instead

4. **Same prompt for all models**
   - Different models need different prompting
   - May need model-specific prompt variants

---

## Success Metrics

Track these to validate implementation:

- [ ] **Context Length**: Successfully handle 1M+ char inputs
- [ ] **Accuracy**: Match or exceed base model on test benchmarks
- [ ] **Cost**: Stay within 2x base model cost for equivalent tasks
- [ ] **Speed**: Reasonable completion time (<5min for 1M char inputs with async)

---

## File Structure

```
crates/goose/src/
├── rlm/
│   ├── mod.rs              # Module exports, RlmConfig
│   ├── context_store.rs    # Context storage
│   ├── prompts.rs          # System prompts
│   └── tests/
│       ├── mod.rs
│       ├── test_context_store.rs
│       └── test_rlm_extension.rs
├── agents/
│   ├── rlm_extension.rs    # Platform extension (new)
│   ├── extension.rs        # Add to PLATFORM_EXTENSIONS (modify)
│   └── agent.rs            # Add RLM detection (modify)
└── config/
    └── rlm.rs              # RLM config parsing (new)
```

---

## Resources

- **Paper**: https://arxiv.org/abs/2512.24601
- **GitHub Issue**: https://github.com/block/goose/issues/6651
- **System Prompts**: See paper Appendix D
- **Benchmarks**: OOLONG, S-NIAH, BrowseComp-Plus

---

## Notes

- Goose already has sub-agent support - leverage `subagent_tool.rs`
- Existing `code_execution_extension` runs JS - could add Python support later
- Start with simple implementation, optimize later
- Test frequently with real long-context tasks
- Monitor costs closely during development

---

**Status**: Ready for implementation
**Priority**: High (significant capability improvement)
**Language**: Rust
