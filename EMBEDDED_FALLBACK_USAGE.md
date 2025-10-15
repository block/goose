# Embedded Provider Tool Calling Fallback - Usage Guide

## How to Know Which Mode is Being Used

The embedded provider will log which mode it's using. Look for these log messages:

### Detection Phase (First tool request)
```
INFO  Model supports native tool calling
```
or
```
INFO  Model does not support native tool calling, will use emulation mode
```

### Each Request
```
INFO  Using tool emulation mode
```

### Forced Emulation
```
INFO  Tool emulation forced via EMBEDDED_FORCE_TOOL_EMULATION
```

### Where to See Logs

Logs are written to timestamped files in the logs directory. To find the exact location:

```bash
goose info
# Shows: Logs dir: /Users/YOU/.local/state/goose/logs
```

Logs go to: `~/.local/state/goose/logs/cli/YYYY-MM-DD/TIMESTAMP.log`

To watch logs in real-time:
```bash
# In one terminal, start goose
GOOSE_PROVIDER=embedded goose session

# In another terminal, tail the latest log
tail -f $(ls -t ~/.local/state/goose/logs/cli/*/*.log | head -1)
```

The logs are in JSON format. To make them readable:
```bash
# View formatted logs
tail -f $(ls -t ~/.local/state/goose/logs/cli/*/*.log | head -1) | jq -r '.fields.message // .message // .'
```

## System Prompt Handling (Matching PR #4813)

### Native Mode (Model supports tools)
```
Request structure:
- system: <your system prompt>
- messages: [conversation history]
- tools: [tool definitions]
```

### Emulation Mode (Model doesn't support tools)
```
Request structure:
- system: "" (empty)
- messages: [
    { role: "user", content: EMULATION_SYSTEM_PROMPT },  ← Added as first message
    ...conversation history...
  ]
- tools: [] (no tools)
```

This matches PR #4813's approach:
- System prompt goes IN the messages as a user message
- NOT as a separate system parameter
- Prevents "jamming everything" into both system and messages

### Why This Approach?

From PR #4813, small models work better when:
1. The emulation instructions are part of the conversation (user message)
2. There's no separate system parameter competing for attention
3. The model sees: instruction → conversation → generate response

## Testing the Implementation

### 1. Test Native Tool Support Detection
```bash
# Start goose with a tool-capable model
GOOSE_PROVIDER=embedded GOOSE_MODEL=qwen2.5-7b-instruct goose session

# In another terminal, watch logs
tail -f $(ls -t ~/.local/state/goose/logs/cli/*/*.log | head -1) | \
  jq -r 'select(.fields.message) | .fields.message'

# Look for: "Model supports native tool calling"
```

### 2. Test Emulation Fallback
```bash
# Download TinyLlama (small, no tool support)
mkdir -p ~/.models
cd ~/.models
curl -L -o tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf \
  https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf

# Start goose
GOOSE_PROVIDER=embedded GOOSE_MODEL=tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf goose session

# Watch logs
tail -f $(ls -t ~/.local/state/goose/logs/cli/*/*.log | head -1) | \
  jq -r 'select(.fields.message) | .fields.message'

# Look for:
# "Model does not support native tool calling, will use emulation mode"
# "Using tool emulation mode"
# "Executing shell command: ..." (when you ask it to run a command)
```

### 3. Force Emulation Mode
```bash
# Force emulation even with a tool-capable model (for testing)
EMBEDDED_FORCE_TOOL_EMULATION=true \
GOOSE_PROVIDER=embedded \
GOOSE_MODEL=qwen2.5-7b-instruct \
goose session

# Watch logs - should see:
# "Tool emulation forced via EMBEDDED_FORCE_TOOL_EMULATION"
# "Using tool emulation mode"
```

### 4. Test Chat Mode (No Tools)
```bash
# Chat mode disables all tools
GOOSE_MODE=chat GOOSE_PROVIDER=embedded goose session

# Should NOT see any tool-related messages in logs
```

## Emulation Mode Behavior

When in emulation mode:

1. **Model receives this prompt as first user message:**
```
You are Goose, a general-purpose AI agent. Your goal is to analyze and solve problems by writing code.

# Tool Call Format

When you need to execute a tool, write ONLY the JSON tool call on a new line:

{"tool": "tool_name", "args": {"param": "value"}}

# Available Tools

- **shell**: Execute shell commands
  - Format: {"tool": "shell", "args": {"command": "your_command_here"}}
  - Example: {"tool": "shell", "args": {"command": "ls ~/Downloads"}}

- **final_output**: Signal task completion
  - Format: {"tool": "final_output", "args": {"summary": "what_was_accomplished"}}
```

2. **Model generates text response** (may contain JSON tool calls)

3. **ToolExecutor parses and executes** any `{"tool": "...", "args": {...}}` patterns

4. **Results appended inline:**
```
{"tool": "shell", "args": {"command": "ls"}}

Command executed successfully:
```
file1.txt
file2.txt
```
```

5. **Augmented text returned** to goose as the assistant's message

## Differences from Native Mode

| Aspect | Native Mode | Emulation Mode |
|--------|-------------|----------------|
| Tools parameter | Passed to model | Empty array |
| System prompt | Caller's system prompt | Empty string |
| Emulation prompt | Not used | First user message |
| Tool format | OpenAI tool_calls format | JSON in text: `{"tool":"name","args":{}}` |
| Tool execution | By goose framework | By ToolExecutor inline |
| Supported tools | All goose tools | Only: shell, final_output |
| Performance | Faster, structured | Slower, text parsing |
| Reliability | Higher | Lower (depends on model following format) |

## Configuration Options

```bash
# Force emulation (useful for testing)
EMBEDDED_FORCE_TOOL_EMULATION=true

# Other embedded provider configs (still apply)
EMBEDDED_MODEL_PATH=~/.models/my-model.gguf
EMBEDDED_GPU_LAYERS=40
EMBEDDED_CTX_SIZE=8192
EMBEDDED_THREADS=8

# Goose mode (chat = no tools at all)
GOOSE_MODE=chat    # or "manual" or "auto"
```

## Troubleshooting

### "Model supports native tool calling" but tools don't work
- The detection might have false-positive
- Force emulation: `EMBEDDED_FORCE_TOOL_EMULATION=true`

### Model not following JSON format in emulation mode
- Check model logs to see what it's generating
- Some models need more examples or different prompting
- Consider using a tool-capable model instead

### No logs showing
- Increase log level: `RUST_LOG=goose=debug`
- Check log file location: usually `~/.config/goose/goose.log`

### Commands not executing in emulation mode
- Look for: "Executing shell command: ..." in logs
- Check if model is actually outputting the JSON format
- Try explicit test: ask model to run `ls` command

## Examples

### Successful Emulation
```
DEBUG Detecting tool calling support...
INFO  Model does not support native tool calling, will use emulation mode
INFO  Using tool emulation mode
INFO  Executing shell command: ls ~/Downloads
```

### Successful Native
```
DEBUG Detecting tool calling support...
INFO  Model supports native tool calling
[no "Using tool emulation mode" message - using native path]
```

### Forced Emulation
```
INFO  Tool emulation forced via EMBEDDED_FORCE_TOOL_EMULATION
INFO  Using tool emulation mode
INFO  Executing shell command: pwd
```
