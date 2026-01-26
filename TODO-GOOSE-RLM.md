# TODO: RLM (Recursive Language Models) Implementation for Goose

**Reference Paper**: [Recursive Language Models (arXiv:2512.24601)](https://arxiv.org/abs/2512.24601)
**GitHub Issue**: [#6651](https://github.com/block/goose/issues/6651)

---

## Current Status: Phase 2 Complete ✅

**Last Updated**: 2025-01-25
**Branch**: `feature-recursive-language-models`

### What's Done

| Component | Status | File |
|-----------|--------|------|
| RLM Module Structure | ✅ Done | `crates/goose/src/rlm/mod.rs` |
| Context Store | ✅ Done | `crates/goose/src/rlm/context_store.rs` |
| RLM System Prompts | ✅ Done | `crates/goose/src/rlm/prompts.rs` |
| Test Utilities | ✅ Done | `crates/goose/src/rlm/test_utils.rs` |
| RLM Platform Extension | ✅ Done | `crates/goose/src/agents/rlm_extension.rs` |
| Extension Registration | ✅ Done | Added to `PLATFORM_EXTENSIONS` |
| Unit Tests | ✅ Done | 562 tests passing |
| `rlm_query` with LLM calls | ✅ Done | Makes actual provider calls |
| Auto-detection in Agent | ✅ Done | `maybe_enable_rlm()` in agent.rs |
| Integration Tests | ✅ Done | `tests/rlm_integration.rs` - 9 tests |
| YAML Config Support | ✅ Done | `crates/goose/src/config/rlm.rs` |

### What's Next

| Task | Priority | Notes |
|------|----------|-------|
| CLI flags (`--rlm`) | Low | Optional enhancement |
| End-to-end test with real LLM | Low | Manual testing recommended |
| Performance optimizations | Low | Async sub-agent calls, caching |

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
**File**: `crates/goose/src/rlm/test_utils.rs`

- [x] `generate_needle_haystack()` - Create test data
- [x] `generate_multi_document_context()` - Multi-doc test data
- [x] Unit tests for context store
- [x] Unit tests for extension tools

---

### Phase 2: Integration with Goose ✅ COMPLETE

#### 5. Wire `rlm_query` to Sub-Agents ✅
**File**: `crates/goose/src/agents/rlm_extension.rs`

- [x] Access provider through ExtensionManager
- [x] Make actual LLM calls with context slice
- [x] Return sub-agent's response with usage stats

#### 6. Agent Auto-Detection ✅
**File**: `crates/goose/src/agents/agent.rs` (modifications)

- [x] Add `get_rlm_config()` function (uses RlmConfigManager)
- [x] Add `maybe_enable_rlm()` helper method
- [x] Detect large context in `reply()` method
- [x] Auto-enable RLM extension when threshold exceeded
- [x] Generate modified prompt with RLM instructions

#### 7. Configuration System ✅
**File**: `crates/goose/src/config/rlm.rs`

- [x] Add RLM configuration parsing from YAML
- [x] Config keys: `rlm.enabled`, `rlm.context_threshold`, etc.
- [x] Environment variable overrides (GOOSE_RLM_*)

#### 8. CLI Enhancements (Optional)
**File**: `crates/goose-cli/src/commands/session.rs`

- [ ] Add `--rlm` flag to force RLM mode
- [ ] Add `--rlm-threshold` to override threshold

---

### Phase 3: Testing & Validation ✅ COMPLETE

#### 9. Integration Tests
**File**: `crates/goose/tests/rlm_integration.rs`

- [x] RLM extension tools availability test
- [x] Tool schema validation tests
- [x] RLM candidate detection tests
- [x] Context store tests (needle-in-haystack, multi-document)
- [x] Chunk boundary tests
- [ ] End-to-end test with real LLM (manual testing)

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
│   └── test_utils.rs       # Test utilities, needle-in-haystack
├── agents/
│   ├── rlm_extension.rs    # RlmClient platform extension ✅
│   ├── agent.rs            # maybe_enable_rlm() auto-detection ✅
│   ├── extension.rs        # PLATFORM_EXTENSIONS (modified) ✅
│   └── mod.rs              # Module exports (modified) ✅
├── config/
│   └── rlm.rs              # RlmConfigManager ✅
└── lib.rs                  # Added rlm module ✅
crates/goose/tests/
└── rlm_integration.rs      # Integration tests ✅
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

**Status**: Phase 2 Complete ✅
**Priority**: High
**Tests**: 564 unit tests + 9 integration tests passing
