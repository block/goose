# Pi Improvements Implementation Plan for Goose

Based on analysis of Pi (pi-mono) and the gap analysis research, this plan outlines improvements to bring Goose's performance closer to Pi.

## Overview

Key improvements identified:
1. **Structured Compaction Summaries** - Replace free-form summaries with Goal/Progress/Next Steps format
2. **File Operation Tracking** - Track read/modified files for context preservation
3. **Turn-Aware Compaction** - Don't split user/assistant pairs when compacting
4. **Edit Fallback Strategies** - Add Unicode normalization and line-trimmed matching
5. **Read Tool Continuation Guidance** - Improve messaging for large files
6. **Fast Token Estimation** - Use chars/4 heuristic for threshold checks

## Implementation Status

| Item | Status | Location | Notes |
|------|--------|----------|-------|
| I1: Structured Compaction Prompt | ✅ DONE | `crates/goose/src/prompts/compaction.md` | Pi's structured format |
| I2: File Operation Tracking | ✅ DONE | `crates/goose/src/context_mgmt/mod.rs` | FileOperations struct |
| I3: Turn-Aware Cut Point Detection | ⏸️ BLOCKED | `crates/goose/src/context_mgmt/mod.rs` | Requires deeper refactoring |
| I4: Edit Fuzzy Matching Fallback | ✅ DONE | `crates/goose-mcp/src/developer/text_editor.rs` | Unicode normalization |
| I5: Read Tool Continuation Hints | ✅ DONE | `crates/goose-mcp/src/developer/text_editor.rs` | Actionable hints |
| I6: Fast Token Estimation | ✅ DONE | `crates/goose/src/context_mgmt/mod.rs` | chars/4 heuristic |

---

## Completed Items

### I1: Structured Compaction Prompt ✅

**Goal**: Replace free-form compaction summaries with Pi's structured format.

**Changes**:
- Updated `crates/goose/src/prompts/compaction.md` with structured format
- Format: Goal / Constraints / Progress (Done/In Progress/Blocked) / Key Decisions / Next Steps / Critical Context

**Files Changed**:
- `crates/goose/src/prompts/compaction.md`

### I2: File Operation Tracking ✅

**Goal**: Track which files were read/modified during a session for inclusion in compaction summaries.

**Changes**:
- Added `FileOperations` struct to `context_mgmt/mod.rs`
- Extracts file paths from tool requests (read_file, write_file, text_editor, etc.)
- Appends file lists to compaction summary automatically

**Files Changed**:
- `crates/goose/src/context_mgmt/mod.rs` (lines 97-163, 443-447)

**Tests**:
- `test_file_operations_extraction`
- `test_file_operations_format_for_summary`

### I4: Edit Fuzzy Matching Fallback ✅

**Goal**: Add fallback strategies when exact string matching fails.

**Changes**:
- Added `strip_bom()` function for BOM handling
- Added `normalize_for_fuzzy_match()` function (Unicode quotes, dashes, trailing whitespace)
- Added `normalize_to_lf()` for line ending normalization
- Updated `text_editor_replace()` with fallback chain: exact match -> fuzzy match -> error
- Improved error messages with actionable guidance

**Fallback Chain**:
1. Exact string match (after line ending normalization)
2. Fuzzy match (Unicode normalization + trailing whitespace)
3. Error with actionable tips

**Files Changed**:
- `crates/goose-mcp/src/developer/text_editor.rs` (lines 21-75, 903-975)
- `crates/goose-mcp/src/developer/tests/test_fuzzy_match.rs` (new file, 14 tests)

### I5: Read Tool Continuation Hints ✅

**Goal**: Provide actionable guidance when files are truncated.

**Changes**:
- Updated `recommend_read_range()` with specific view_range values
- Added continuation hints in `format_file_content()` when partial file is shown
- Messages now include total line count and suggested next range

**Files Changed**:
- `crates/goose-mcp/src/developer/text_editor.rs` (lines 546-556, 584-598)

### I6: Fast Token Estimation ✅

**Goal**: Use chars/4 heuristic for compaction threshold checks (faster than tiktoken).

**Changes**:
- Added `estimate_message_tokens()` function using chars/4
- Added `estimate_tokens_fast()` for batch estimation
- Handles all MessageContent variants including images at ~1200 tokens

**Files Changed**:
- `crates/goose/src/context_mgmt/mod.rs` (lines 21-92)

**Tests**:
- `test_estimate_tokens_fast`
- `test_estimate_tokens_fast_empty`

---

## Blocked Items

### I3: Turn-Aware Cut Point Detection ⏸️

**Goal**: When compacting, don't split user/assistant message pairs.

**Status**: Blocked - requires deeper refactoring of the compaction flow.

**Rationale**: The current middle-out removal strategy already preserves recent context effectively. Implementing turn-aware cutting would require significant changes to how messages are selected for removal, and the benefit is marginal given the existing approach.

---

## Testing

### Run All New Tests
```bash
# Context management tests
cargo test -p goose --lib context_mgmt

# Fuzzy matching tests
cargo test -p goose-mcp test_fuzzy_match

# All goose-mcp tests
cargo test -p goose-mcp
```

### Manual Testing

1. **Compaction**: Run a long session, trigger compaction (or use `/compact`), verify structured output
2. **Edit with smart quotes**: Create a file with smart quotes, try to edit it
3. **Large file reading**: Read a file >2000 lines, verify actionable hints

---

## Summary

5 of 6 planned improvements have been implemented:

1. ✅ **Structured Compaction** - Better context preservation with Goal/Progress/Next Steps format
2. ✅ **File Tracking** - Compaction summaries now include files read/modified
3. ✅ **Fuzzy Edit Matching** - Handles Unicode smart quotes, dashes, and whitespace differences
4. ✅ **Read Continuation Hints** - Actionable guidance for navigating large files
5. ✅ **Fast Token Estimation** - chars/4 heuristic for quick threshold checks

The turn-aware cut point detection (I3) was deferred as the current middle-out removal strategy already provides good context preservation.

---

## Future Improvements (Not Implemented)

These items were identified in the gap analysis but not implemented in this phase:

### System Prompt Simplification
- Pi's system prompt is ~50 lines, focused on coding tasks
- Goose's is ~70 lines, extension-focused
- Could add conditional guidelines based on available tools
- **Reason not implemented**: Would require broader discussion about Goose's identity and extension model

### Model-Specific Prompts
- OpenCode has different prompts for GPT, Claude, Gemini
- Could optimize for each model's strengths
- **Reason not implemented**: Lower priority, requires extensive testing

### Iterative Summary Updates
- Pi can update existing summaries rather than regenerating
- Preserves historical context across multiple compactions
- **Reason not implemented**: Current approach works well, complexity not justified

### LSP Integration for Edit Validation
- OpenCode validates edits produce valid code via LSP
- Could catch syntax errors immediately after edit
- **Reason not implemented**: High effort, requires LSP infrastructure

---

## Verification

All changes have been verified:

```bash
# Build passes
cargo build -p goose -p goose-mcp --release

# All new tests pass
cargo test -p goose --lib context_mgmt  # 7 tests
cargo test -p goose-mcp test_fuzzy_match  # 14 tests
cargo test -p goose-mcp text_editor  # 29 tests

# Clippy passes (no new warnings)
./scripts/clippy-lint.sh
```
