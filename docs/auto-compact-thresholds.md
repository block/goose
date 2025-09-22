# Auto-Compaction Thresholds

## Overview

Goose now supports separate auto-compaction thresholds for different contexts:

1. **`GOOSE_AUTO_COMPACT_THRESHOLD`** - Used when processing user messages (default: 0.8 or 80%)
2. **`GOOSE_AGENT_COMPACT_THRESHOLD`** - Used during agent loop processing (defaults to `GOOSE_AUTO_COMPACT_THRESHOLD` if not set)

## Why Two Thresholds?

The agent often churns through many iterations without user input, especially when:
- Processing large files
- Making multiple tool calls
- Running complex tasks

During these autonomous operations, context can grow rapidly. Having a separate, potentially more aggressive threshold for the agent loop helps prevent context exhaustion.

## Configuration Examples

### Default Behavior (80% threshold for everything)
```bash
# No configuration needed, or:
export GOOSE_AUTO_COMPACT_THRESHOLD=0.8
```

### More Aggressive Agent Compaction
```bash
# Compact at 80% when processing user messages
export GOOSE_AUTO_COMPACT_THRESHOLD=0.8

# But compact at 60% during agent churning
export GOOSE_AGENT_COMPACT_THRESHOLD=0.6
```

### Very Aggressive Agent Compaction (for limited context models)
```bash
export GOOSE_AUTO_COMPACT_THRESHOLD=0.7
export GOOSE_AGENT_COMPACT_THRESHOLD=0.5
```

### Disable Auto-Compaction
```bash
# Set to 0 or 1.0 to effectively disable
export GOOSE_AUTO_COMPACT_THRESHOLD=0
# or
export GOOSE_AUTO_COMPACT_THRESHOLD=1.0
```

## How It Works

1. **On User Messages**: When you send a message, Goose checks if the conversation exceeds `GOOSE_AUTO_COMPACT_THRESHOLD`
2. **During Agent Processing**: After each turn in the agent loop, Goose checks against `GOOSE_AGENT_COMPACT_THRESHOLD`
3. **Smart Fallback**: If `GOOSE_AGENT_COMPACT_THRESHOLD` is not set, it falls back to `GOOSE_AUTO_COMPACT_THRESHOLD`

## Best Practices

- **Start with defaults**: The 80% default works well for most cases
- **Monitor your usage**: If you see "Context length exceeded" errors, consider lowering the thresholds
- **Model-specific tuning**: Models with smaller context windows may benefit from more aggressive thresholds
- **Task-specific tuning**: Long-running background tasks might benefit from lower agent thresholds

## Implementation Details

The auto-compaction:
- Uses actual token counts from session metadata when available
- Falls back to estimated counts when metadata is unavailable
- Preserves the most recent user message during compaction
- Generates a summary that maintains context while reducing tokens
- Notifies you when compaction occurs with a system message
