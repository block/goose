# TODO: RLM (Recursive Language Models) Implementation for Goose

**Reference Paper**: [Recursive Language Models (arXiv:2512.24601)](https://arxiv.org/abs/2512.24601)
**GitHub Issue**: [#6651](https://github.com/block/goose/issues/6651)

---

## Current Status: Phase 1 Complete ✅

**Last Updated**: 2025-01-25
**Commit**: `946c1cee feat: implement RLM (Recursive Language Models) extension`

### What's Done

| Component | Status | File |
|-----------|--------|------|
| RLM Module Structure | ✅ Done | `crates/goose/src/rlm/mod.rs` |
| Context Store | ✅ Done | `crates/goose/src/rlm/context_store.rs` |
| RLM System Prompts | ✅ Done | `crates/goose/src/rlm/prompts.rs` |
| Test Utilities | ✅ Done | `crates/goose/src/rlm/tests.rs` |
| RLM Platform Extension | ✅ Done | `crates/goose/src/agents/rlm_extension.rs` |
| Extension Registration | ✅ Done | Added to `PLATFORM_EXTENSIONS` |
| Unit Tests | ✅ Done | 21 tests passing |

### What's Next (Phase 2)

| Task | Priority | Notes |
|------|----------|-------|
| Wire `rlm_query` to actual sub-agents | High | Currently returns context for parent to process |
| Add auto-detection in Agent | High | Enable RLM mode automatically for large inputs |
| Full integration test with LLM | High | Need running LLM to test end-to-end |
| Add YAML config support | Medium | `rlm.enabled`, `rlm.context_threshold`, etc. |
| CLI flags (`--rlm`) | Low | Optional enhancement |

---

## Overview

Implement RLM support in Goose to handle arbitrarily long prompts by treating them as external environment variables that can be programmatically examined, decomposed, and recursively processed through sub-agent calls.

### Key Benefits
- Handle inputs 100x+ beyond normal context windows
- Dramatically outperform base LLMs on long-context tasks
- Maintain comparable or lower cost per query
- Enable processing of 10M+ token inputs

---

## Goose Architecture Context

> **Important**: Goose is written in **Rust**, not Python.

### Relevant Crates & Files

| Component | Location | Purpose |
|-----------|----------|---------|
| Agent Core | `crates/goose/src/agents/agent.rs` | Main agent loop, tool dispatch |
| Extensions | `crates/goose/src/agents/extension.rs` | Extension types (Platform, Stdio, etc.) |
| RLM Extension | `crates/goose/src/agents/rlm_extension.rs` | **NEW** - RLM tools |
| RLM Module | `crates/goose/src/rlm/` | **NEW** - Config, context store, prompts |
| Sub-Agent Tool | `crates/goose/src/agents/subagent_tool.rs` | Existing sub-agent support |

---

## Implementation Checklist

### Phase 1: Core Components ✅ COMPLETE

#### 1. Context Storage System ✅
**File**: `crates/goose/src/rlm/context_store.rs`

- [x] Create `ContextStore` struct
- [x] `store_context()` - Write context to file
- [x] `get_metadata()` - Return length, path, chunk info
- [x] `read_context()` - Load context from storage
- [x] `read_slice()` - Load partial context
- [x] Chunk boundary calculation

#### 2. RLM Platform Extension ✅
**File**: `crates/goose/src/agents/rlm_extension.rs`

- [x] Create `RlmClient` implementing `McpClientTrait`
- [x] `rlm_get_context_metadata` tool
- [x] `rlm_read_context_slice` tool
- [x] `rlm_query` tool (placeholder - returns context for parent)
- [x] `rlm_store_variable` / `rlm_get_variable` / `rlm_list_variables` tools
- [x] `rlm_finalize` tool
- [x] Register in `PLATFORM_EXTENSIONS` (disabled by default)

#### 3. RLM System Prompts ✅
**File**: `crates/goose/src/rlm/prompts.rs`

- [x] Define `RLM_SYSTEM_PROMPT` constant
- [x] Instructions for all RLM tools
- [x] Chunking strategy guidance
- [x] Example workflow

#### 4. RLM Config & Detection ✅
**File**: `crates/goose/src/rlm/mod.rs`

- [x] `RlmConfig` struct with defaults
- [x] `is_rlm_candidate()` function

#### 5. Test Utilities ✅
**File**: `crates/goose/src/rlm/tests.rs`

- [x] `generate_needle_haystack()` - Create test data
- [x] `generate_multi_document_context()` - Multi-doc test data
- [x] Unit tests for context store
- [x] Unit tests for extension tools

---

### Phase 2: Integration with Goose 🔄 IN PROGRESS

#### 5. Wire `rlm_query` to Sub-Agents
**File**: `crates/goose/src/agents/rlm_extension.rs`

- [ ] Import and use `subagent_tool` or `subagent_handler`
- [ ] Create sub-agent with context slice injected
- [ ] Return sub-agent's response

#### 6. Agent Auto-Detection
**File**: `crates/goose/src/agents/agent.rs` (modifications)

- [ ] Add RLM config to `AgentConfig`
- [ ] Modify `reply()` to detect large context
- [ ] Auto-enable RLM extension when threshold exceeded
- [ ] Inject RLM system prompt

#### 7. Configuration System
**File**: `crates/goose/src/config/rlm.rs` (new)

- [ ] Add RLM configuration parsing from YAML
- [ ] Config keys: `rlm.enabled`, `rlm.context_threshold`, etc.

#### 8. CLI Enhancements (Optional)
**File**: `crates/goose-cli/src/commands/session.rs`

- [ ] Add `--rlm` flag to force RLM mode
- [ ] Add `--rlm-threshold` to override threshold

---

### Phase 3: Testing & Validation

#### 9. Integration Tests
**File**: `crates/goose/tests/rlm_integration.rs`

- [ ] End-to-end test with real LLM
- [ ] Test needle-in-haystack with 1M+ chars
- [ ] Test recursion depth limits
- [ ] Test max iteration limits

---

### Phase 4: Optimization & Polish

#### 10. Performance Improvements

- [ ] Async sub-agent calls (parallel execution)
- [ ] Caching for repeated sub-queries
- [ ] Smart chunking (natural boundaries)

#### 11. Monitoring & Debugging

- [ ] Cost tracking per RLM session
- [ ] Logging via `tracing`
- [ ] `--rlm-debug` flag

---

## RLM Tools Reference

| Tool | Purpose |
|------|---------|
| `rlm_get_context_metadata` | Get context size, chunk count, boundaries |
| `rlm_read_context_slice(start, end)` | Read characters from `start` to `end` |
| `rlm_query(prompt, start, end)` | Query sub-agent with context slice |
| `rlm_store_variable(name, value)` | Store intermediate result |
| `rlm_get_variable(name)` | Retrieve stored value |
| `rlm_list_variables` | List all stored variable names |
| `rlm_finalize(answer)` | Complete RLM session with final answer |

---

## File Structure (Current)

```
crates/goose/src/
├── rlm/
│   ├── mod.rs              # RlmConfig, is_rlm_candidate()
│   ├── context_store.rs    # ContextStore, ContextMetadata
│   ├── prompts.rs          # RLM_SYSTEM_PROMPT
│   └── tests.rs            # Test utilities, needle-in-haystack
├── agents/
│   ├── rlm_extension.rs    # RlmClient platform extension ✅
│   ├── extension.rs        # PLATFORM_EXTENSIONS (modified) ✅
│   └── mod.rs              # Module exports (modified) ✅
└── lib.rs                  # Added rlm module ✅
```

---

## Key Design Decisions

1. **Context as Environment Variable**: Store in file, expose via tools
2. **Tool-based finalization**: Use `rlm_finalize` tool instead of text tags
3. **Extension disabled by default**: Enable explicitly or via auto-detection
4. **Recursion depth = 1**: Paper found this sufficient for most tasks

---

## Resources

- **Paper**: https://arxiv.org/abs/2512.24601
- **GitHub Issue**: https://github.com/block/goose/issues/6651
- **System Prompts**: See paper Appendix D

---

**Status**: Phase 1 Complete, Phase 2 In Progress
**Priority**: High
**Tests**: 21 passing
