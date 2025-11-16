# Dynamic agents.md Loading - Option 2: Tool Result Injection (Legacy)

**Status**: Not Selected - Superseded by Option 1 (System Prompt Extension)

**Date**: 2025-11-16

## Overview

This document contains the implementation plan for Option 2 (Tool Result Injection), which was considered but not selected for implementation. Option 1 (System Prompt Extension with Pruning) was chosen instead due to better token efficiency and persistence.

## Why Option 1 Was Chosen

- Better token efficiency (prompt caching)
- Context persists across entire session
- More authoritative (system-level vs message-level)
- Supports LRU pruning to prevent unbounded growth
- Cleaner integration with existing system prompt patterns

## Option 2 Details

Return agents.md content as part of the text_editor tool result with `.with_audience(vec![Role::Assistant])`, making it visible to the LLM but hidden from the user.

### Advantages
- Simpler implementation (no prompt manager changes)
- Immediate context availability
- Less prompt bloat (only in conversation history)

### Disadvantages
- Higher token cost (content in every message)
- Context not persistent (per-message only)
- Depends on audience filtering support
- Conversation history grows over time

## Implementation Details

See the original plan document for full implementation details. Key phases would have been:

1. State Management (same as Option 1)
2. File Loading Infrastructure (same as Option 1)
3. Minimal text_editor tool changes
4. Agent-side tool result augmentation with Assistant-only audience

## References

- Original combined plan: `docs/plans/2025-11-16-dynamic-agents-md-loading.md` (see Option 2 section)
- Selected approach: `docs/plans/2025-11-16-dynamic-agents-md-loading.md` (Option 1)
