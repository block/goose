# ACP Test Client

A minimal but functional ACP (Agent Client Protocol) test client TUI for testing ACP-compatible agents.

Uses the official [`@agentclientprotocol/sdk`](https://www.npmjs.com/package/@agentclientprotocol/sdk) for protocol handling.

## Features

- **Official SDK**: Uses the official ACP TypeScript SDK for spec-compliant protocol handling
- **Streaming support**: Shows text as it arrives from the agent
- **Tool call display**: Shows tool calls with status indicators (○ pending, ◐ in progress, ✓ completed, ✗ failed)
- **Permission handling**: Interactive permission prompts with keyboard navigation
- **Configurable agents**: Easy switching between goose2, original goose, or custom agents

## Installation

```bash
cd test-client
npm install
```

## Usage

### Quick Start

```bash
# Use default agent (goose2)
npm start

# Use original goose (for comparison testing)
npm start -- --agent goose

# List available agent presets
npm start -- --list
```

### Agent Presets

The test client comes with built-in presets for common agents:

| Preset | Command | Description |
|--------|---------|-------------|
| `goose2` | `cargo run --manifest-path ../Cargo.toml` | goose2 Rust agent (default) |
| `goose2-release` | `cargo run --release --manifest-path ../Cargo.toml` | goose2 release build |
| `goose` | `goose acp` | Original goose CLI |
| `goose-local` | `$GOOSE_PATH acp` | Original goose from custom path |

### Custom Commands

You can also run any ACP-compatible agent directly:

```bash
# Custom command
npm start -- /path/to/my-agent --some-flag

# Or with tsx directly
npx tsx src/index.tsx /path/to/my-agent --some-flag
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `AGENT` | Default agent preset to use (e.g., `AGENT=goose npm start`) |
| `GOOSE_PATH` | Path to goose binary for the `goose-local` preset |

### Examples

```bash
# Test goose2 (default)
npm start

# Test original goose to compare behavior
npm start -- --agent goose

# Test with a specific goose binary
GOOSE_PATH=/usr/local/bin/goose npm start -- --agent goose-local

# Set default to original goose for a session
export AGENT=goose
npm start

# Custom agent command
npm start -- ./target/release/my-custom-agent --verbose
```

## Keyboard Shortcuts

- **Ctrl+C**: Exit the client
- **Enter**: Submit message
- **↑/↓**: Navigate permission options
- **c** or **Esc**: Cancel permission prompt

## Architecture

```
┌─────────────────────────────────────────┐
│  index.tsx - Ink TUI                    │
│  └─ React components for UI             │
├─────────────────────────────────────────┤
│  config.ts - Agent Configuration        │
│  └─ Presets and CLI parsing             │
├─────────────────────────────────────────┤
│  @agentclientprotocol/sdk               │
│  └─ Official ACP TypeScript SDK         │
│  └─ JSON-RPC, types, validation         │
└─────────────────────────────────────────┘
         │
         │ stdin/stdout (ndjson)
         ▼
┌─────────────────────────────────────────┐
│  ACP Agent (goose2, goose, etc.)        │
└─────────────────────────────────────────┘
```

## ACP Protocol Flow

1. **Initialize**: Client sends `initialize` request with protocol version
2. **New Session**: Client creates a session with `session/new`
3. **Prompt**: Client sends prompts via `session/prompt`
4. **Updates**: Agent sends `session/update` notifications with:
   - `agent_message_chunk`: Streaming text content
   - `tool_call`: Tool execution status
   - `tool_call_update`: Tool status changes
5. **Permissions**: Agent may request permissions via `session/request_permission`

## Adding New Presets

Edit `src/config.ts` to add new agent presets:

```typescript
export const AGENT_PRESETS: Record<string, AgentConfig> = {
  // ... existing presets ...
  
  "my-agent": {
    name: "my-agent",
    command: "/path/to/my-agent",
    args: ["--acp-mode"],
    description: "My custom ACP agent",
  },
};
```
