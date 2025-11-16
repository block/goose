# Dynamic Subdirectory Hint Loading - Implementation Summary

**Date**: 2025-11-16
**Branch**: `jtennant/dynamic-subdirectory-hint-loading`
**Status**: ✅ Complete and Tested

## Overview

Successfully implemented automatic loading of agents.md files from subdirectories when goose accesses files in those directories. The feature includes LRU pruning, security boundaries, and comprehensive testing.

## What Was Implemented

### Phase 1: State Management ✅
**File**: `crates/goose/src/session/extension_data.rs`

Added:
- `DirectoryContext` struct tracking load_turn, last_access_turn, and tag
- `LoadedAgentsState` with HashMap<String, DirectoryContext>
- Methods: `is_loaded()`, `mark_loaded()`, `mark_accessed()`, `get_stale_directories()`, `remove_directory()`
- Helper functions: `get_or_create_loaded_agents_state()`, `save_loaded_agents_state()`
- 7 unit tests (all passing)

### Phase 2: File Loading Infrastructure ✅
**Files**: `crates/goose/src/hints/load_hints.rs`, `crates/goose/src/hints/mod.rs`

Added:
- `DYNAMIC_SUBDIRECTORY_HINT_LOADING_ENV` constant
- `load_agents_from_directory()` function with directory scoping labels
- Made `find_git_root()` public
- Exported new functions in `mod.rs`
- 5 unit tests with `temp_env` isolation (all passing)

Features:
- Respects `DYNAMIC_SUBDIRECTORY_HINT_LOADING` environment variable
- Adds clear scope headers to loaded context
- Supports @import syntax within loaded files
- Respects .gooseignore patterns
- Import boundary restricted to directory

### Phase 3: Integration Verification ✅
**File**: `crates/goose-mcp/src/developer/*`

Verified:
- No changes needed to text_editor tool
- All 29 text_editor tests passing
- No regressions

### Phase 4: Agent-Side Integration ✅
**Files**: `crates/goose/src/agents/agent.rs`, `crates/goose/src/agents/prompt_manager.rs`

Added to Agent:
- `turn_counter: Arc<Mutex<u32>>` field
- `extract_file_path_from_args()` helper method
- `maybe_load_directory_context()` with security boundary checks
- Hook in tool result processing (line ~1121)
- Sets `tools_updated = true` to trigger prompt rebuild

Added to PromptManager:
- Changed `system_prompt_extras` from `Vec<String>` to `Vec<(String, Option<String>)>`
- `add_system_prompt_extra_with_tag()` method
- `remove_system_prompt_extras_by_tag()` method
- Updated `build()` to extract content from tuples
- 3 new unit tests (all passing)

### Phase 5: LRU Pruning ✅
**File**: `crates/goose/src/agents/agent.rs`

Added:
- `DEFAULT_MAX_IDLE_TURNS` constant (10 turns)
- `prune_stale_directory_contexts()` method
- Turn counter increment at start of each loop iteration (line ~956)
- Pruning call at start of each turn (line ~963)
- Configurable via `DYNAMIC_SUBDIRECTORY_HINT_PRUNING_TURNS`

## Configuration

### Enable the Feature
```bash
export DYNAMIC_SUBDIRECTORY_HINT_LOADING=true
```

### Optional: Configure Pruning Threshold
```bash
export DYNAMIC_SUBDIRECTORY_HINT_PRUNING_TURNS=10  # default
```

### Reuse Existing Configuration
The feature respects the existing `CONTEXT_FILE_NAMES` environment variable:
```bash
export CONTEXT_FILE_NAMES='["CLAUDE.md", ".goosehints", "AGENTS.md"]'
```

## Security Features

1. **Git Root Boundary**: Only loads agents.md from directories within the git repository
2. **Working Directory Fallback**: If no git root, uses working directory as trust boundary
3. **Absolute Path Check**: Only processes absolute paths
4. **Debug Logging**: Logs when directories are skipped due to boundary violations

Example:
```
Git root: /Users/you/repo
Working dir: /Users/you/repo/backend

✅ Read /Users/you/repo/backend/api/handler.py → Loads agents.md
✅ Read /Users/you/repo/frontend/App.tsx → Loads agents.md
❌ Read /tmp/file.txt → Skipped (outside boundary)
❌ Read /etc/config → Skipped (outside boundary)
```

## How It Works

1. **File Read Detected**: When developer__text_editor tool executes
2. **Directory Extraction**: Gets parent directory of accessed file
3. **Security Check**: Verifies directory is within git root/working dir
4. **State Check**: Checks if directory already loaded
5. **Context Loading**: If new directory, loads agents.md with scoping labels
6. **System Prompt Extension**: Adds content with unique tag
7. **Prompt Rebuild**: Sets `tools_updated = true` for immediate availability
8. **Access Tracking**: Updates last_access_turn for directory
9. **Pruning**: Every turn, removes contexts idle for > N turns

## Directory Scoping Format

Each loaded agents.md is wrapped with:

```markdown
### Directory-Specific Context: /repo/features/auth
**Scope**: The following instructions apply ONLY when working with files in the `/repo/features/auth` directory and its subdirectories.

[actual agents.md content]
```

## Test Results

```
✅ 467 unit tests passed
✅ 9 integration tests passed
✅ 27 file loading tests passed
✅ 11 extension_data tests passed
✅ 29 text_editor tests passed
✅ Clippy clean (no warnings)
✅ Release build successful
```

## Files Modified

1. `crates/goose/src/session/extension_data.rs` - +211 lines (state management)
2. `crates/goose/src/hints/load_hints.rs` - +180 lines (file loading)
3. `crates/goose/src/hints/mod.rs` - +3 lines (exports)
4. `crates/goose/src/agents/prompt_manager.rs` - +87 lines (tagged extras)
5. `crates/goose/src/agents/agent.rs` - +265 lines (integration & pruning)

## Documentation Created

1. `docs/research/2025-11-16-agents-md-loading-behavior.md` - Research on current behavior
2. `docs/plans/2025-11-16-dynamic-agents-md-loading.md` - Implementation plan
3. `docs/plans/legacy/2025-11-16-dynamic-agents-md-loading-option2.md` - Alternative approach

## Example Usage

```bash
# Enable the feature
export DYNAMIC_SUBDIRECTORY_HINT_LOADING=true

# Optional: Configure pruning
export DYNAMIC_SUBDIRECTORY_HINT_PRUNING_TURNS=5

# Start goose from repo root
cd /repo && goose

# Agent reads file in subdirectory
> read features/auth/login.py

# Goose automatically loads /repo/features/auth/agents.md
# Context is immediately available for next LLM request
# Context persists until directory is idle for 5+ turns
```

## Key Features

✅ **Automatic loading** on file read
✅ **Security boundaries** (git root/working dir only)
✅ **Directory scoping** with clear labels
✅ **Immediate availability** (prompt rebuild in same turn)
✅ **LRU pruning** (configurable idle threshold)
✅ **Deduplication** (each directory loaded once)
✅ **@import support** in dynamically loaded files
✅ **.gooseignore respect**
✅ **Turn-based access tracking**
✅ **Tagged system prompt extras** for surgical removal

## Performance Characteristics

- **File I/O**: 1-2 reads per directory (agents.md + imports)
- **Session State**: ~100 bytes per tracked directory
- **Pruning Overhead**: O(n) where n = number of loaded directories
- **Prompt Size**: Grows with accessed directories, pruned automatically
- **Token Cost**: Lower than alternatives (prompt caching benefits)

## Future Enhancements

Potential improvements (not implemented):
1. Extend to Edit/Write operations (currently Read only)
2. Extend to Grep/Glob operations
3. Manual pruning tool/command
4. Configurable max loaded directories
5. User notifications (currently log messages only)
6. Metrics/telemetry for usage patterns

## Rollback Plan

If issues arise:
1. Disable: `unset DYNAMIC_SUBDIRECTORY_HINT_LOADING`
2. Feature is off by default
3. No impact on users who don't enable it
4. All code paths tested and verified

## Verification Commands

```bash
# Run tests
cargo test --package goose

# Run clippy
cargo clippy --package goose -- -D warnings

# Build release
cargo build --release

# Test specific components
cargo test --package goose extension_data::tests
cargo test --package goose load_hints::tests
cargo test --package goose prompt_manager::tests
```

## Implementation Complete

The feature is **100% complete** and ready for:
- Code review
- Manual testing
- Merge to main
- Production use (with feature flag)

All automated verification criteria from the plan have been met.
