# Tool Calling Fallback for Embedded Provider

## Overview

The embedded provider uses llama-server to run local GGUF models. Some models support native tool calling via the OpenAI-compatible API, but many smaller/older models do not. This document describes the fallback mechanism for models without native tool calling support.

## Architecture Decision

**Approach: Text-Parsing Fallback (from PR #4813)**
- No second model needed (lightweight!)
- Same model instructed via system prompt to output JSON
- Parse text response for `{"tool": "name", "args": {...}}` patterns
- Execute tools (shell commands) inline
- Append results back to text

**Rejected: ToolShim Approach**
- Would require loading a second model (e.g., mistral-nemo)
- Defeats the purpose of small, performant embedded models
- Too heavy for the use case

## How It Works

```
┌─────────────────────────────────────────┐
│  complete_with_model() called           │
└──────────────┬──────────────────────────┘
               │
               ▼
    ┌──────────────────────┐
    │ Check conditions:    │
    │ 1. Not chat mode?    │
    │ 2. Tools requested?  │
    │ 3. Native support?   │
    └──────────┬───────────┘
               │
      ┌────────┴────────┐
      │                 │
      ▼                 ▼
┌──────────┐      ┌──────────────┐
│ NATIVE   │      │ EMULATION    │
│ (current)│      │ (PR #4813)   │
└──────────┘      └──────────────┘
      │                 │
      │                 ▼
      │      Use special system prompt:
      │      "Output JSON like:
      │       {"tool":"shell","args":{...}}"
      │                 │
      │                 ▼
      │      llama-server generates text
      │      (NO tools parameter)
      │                 │
      │                 ▼
      │      Parse text for JSON patterns
      │      Execute shell commands
      │      Append results inline
      │                 │
      └────────┬────────┘
               │
               ▼
         Return Message
```

## Comparison: ToolShim vs PR #4813 Approach

| Aspect | ToolShim (rejected) | PR #4813 Approach (accepted) |
|--------|---------------------|------------------------------|
| Second model? | ✅ Yes (mistral-nemo) | ❌ No - same model |
| How it works | Separate API call to interpret | Same model instructed via prompt |
| Tools supported | All goose tools | Hardcoded: shell, final_output |
| Weight | Heavy (2+ models) | Lightweight (1 model) |
| Architecture | Uses ToolInterpreter trait | Simple text parsing |
| Overhead | Extra inference pass | None - just text parsing |

## Emulation Mode Details

### When Emulation Mode Activates

```rust
let use_emulation = 
    !self.tool_calling_support.unwrap_or(true)  // Model doesn't support native tools
    && goose_mode != "chat"                      // Not in chat-only mode
    && !tools.is_empty();                        // Tools were requested
```

### System Prompt for Emulation

```
You are Goose, a general-purpose AI agent. Your goal is to analyze and solve problems by writing code.

# Tool Call Format

When you need to execute a tool, write ONLY the JSON tool call on a new line:

{"tool": "tool_name", "args": {"param": "value"}}

The tool will execute immediately and you'll receive the result (success or error) to continue with.

# Available Tools

- **shell**: Execute shell commands
  - Format: {"tool": "shell", "args": {"command": "your_command_here"}}
  - Example: {"tool": "shell", "args": {"command": "ls ~/Downloads"}}

- **final_output**: Signal task completion with a detailed summary of work done
  - Format: {"tool": "final_output", "args": {"summary": "what_was_accomplished"}}

# Instructions

1. Analyze the request and break down into smaller tasks if appropriate
2. Execute ONE tool at a time
3. STOP when the original request was satisfied
4. Call the final_output tool when done

# Response Guidelines

- Use Markdown formatting for all responses except tool calls.
- Whenever taking actions, use the pronoun 'I'
```

### Text Parsing

The `ToolExecutor` scans the model's text response for JSON patterns:

1. Find `{"tool":"` in the text
2. Parse the complete JSON object using `find_json_end()`
3. Extract tool name and arguments
4. Execute the tool (shell command)
5. Append the result inline: `\n\n<tool output>\n`
6. Continue scanning for more tool calls

### Tool Execution

**Supported Tools (Hardcoded):**
- `shell`: Execute shell commands via `sh -c`
  - Returns stdout/stderr
  - Truncates large outputs (>4KB)
- `final_output`: No-op, signals completion

**Output Truncation:**
- Tool outputs >4KB are truncated
- Prevents context overflow with small models

## Tool Capability Detection

### On Initialization

```rust
async fn detect_tool_support(&self) -> bool {
    // Make test request with a simple tool definition
    let test_tool = Tool {
        name: "test".into(),
        description: Some("test tool".to_string()),
        input_schema: json!({"type": "object", "properties": {}}),
    };
    
    let test_payload = create_request(
        &self.model,
        "test",
        &[Message::user().with_text("test")],
        &[test_tool],
        &ImageFormat::OpenAi,
    );
    
    match self.post(&test_payload).await {
        Ok(response) => {
            // Check if response has tool_calls field (native support)
            response.get("choices")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("message"))
                .and_then(|m| m.get("tool_calls"))
                .is_some()
        }
        Err(_) => false, // Error likely means no tool support
    }
}
```

### Caching

The detection result is cached in `tool_calling_support: Option<bool>`:
- `None`: Not yet detected
- `Some(true)`: Native tool calling works
- `Some(false)`: Needs emulation

## Configuration

### Environment Variables

- `GOOSE_MODE`: 
  - `"chat"`: No tools at all
  - `"manual"` or `"auto"`: Enable tools (default)
  
- `EMBEDDED_FORCE_TOOL_EMULATION`:
  - `"true"`: Always use emulation (skip detection)
  - `"false"`: Use detection (default)

### Config Keys in Metadata

```rust
ConfigKey::new("EMBEDDED_FORCE_TOOL_EMULATION", false, false, Some("false"))
```

## Implementation

### Modified Files

- `crates/goose/src/providers/embedded.rs`
  - Add `tool_calling_support: Option<bool>` field
  - Add `detect_tool_support()` method
  - Add `ToolExecutor` struct (ported from PR #4813)
  - Add `EMULATION_SYSTEM_PROMPT` constant
  - Modify `complete_with_model()` with conditional logic

### ToolExecutor Structure

```rust
struct ToolExecutor;

impl ToolExecutor {
    /// Parse and execute JSON tool calls from the response
    async fn execute_tool_calls(text: &str) -> String {
        // Find {"tool":"...", "args":{...}} patterns
        // Parse JSON
        // Execute tools
        // Append results
    }
    
    /// Find the end of a JSON object in text
    fn find_json_end(text: &str) -> Option<usize> {
        // Track braces and strings
        // Return index of closing brace
    }
    
    /// Execute a single tool call
    async fn execute_tool_call(json: &Value) -> Option<String> {
        match json.get("tool")?.as_str()? {
            "shell" => /* run command */,
            "final_output" => /* no-op */,
            _ => None,
        }
    }
}
```

## Testing Strategy

### Test Cases

1. **Tool-capable model (e.g., qwen2.5-7b-instruct with tool support)**
   - Should detect native support
   - Should use native tool calling (current path)
   - Should NOT use emulation

2. **Non-tool model**
   - Should detect lack of support
   - Should fall back to emulation
   - Should execute shell commands via text parsing
   - Shell commands should work and return output

3. **Chat mode**
   - Should not use tools at all (current behavior)
   - Should work regardless of tool support

4. **Forced emulation**
   - Set `EMBEDDED_FORCE_TOOL_EMULATION=true`
   - Should always use emulation even if native works

### Test Commands

```bash
# Test with tool-capable model
GOOSE_PROVIDER=embedded GOOSE_MODEL=qwen2.5-7b-instruct goose session start

# Test with forced emulation
EMBEDDED_FORCE_TOOL_EMULATION=true GOOSE_PROVIDER=embedded goose session start

# Test chat mode (no tools)
GOOSE_MODE=chat GOOSE_PROVIDER=embedded goose session start
```

## Design Decisions

### Why Hardcoded Tools?

This is a **fallback mode**, not the primary path. For models that support native tool calling, goose's full tool system works. For models that don't, having basic shell execution via text parsing is better than nothing.

The hardcoded tools (shell, final_output) provide core functionality:
- `shell`: Execute arbitrary commands (most important)
- `final_output`: Signal completion (nice to have)

### Why Not Support All Goose Tools?

Supporting all goose tools in emulation mode would require:
1. Dynamic system prompt generation based on available tools
2. Routing tool calls back through goose's tool system
3. Converting between formats

This adds complexity and defeats the lightweight goal. The PR #4813 approach keeps it simple.

### Why Not Always Use Emulation?

Native tool calling is more reliable when available:
- Better structured output
- Model was trained for it
- Less parsing brittleness
- Better error handling

Emulation is a fallback, not a replacement.

## Future Enhancements

### Possible Improvements

1. **Better Detection**: Test multiple requests to confirm tool support
2. **More Tools**: Add file operations (read/write) to emulation mode
3. **Streaming**: Support streaming in emulation mode
4. **Error Handling**: Better recovery from malformed JSON
5. **Metrics**: Track emulation vs native usage

### Known Limitations

1. **Hardcoded tools**: Only shell and final_output
2. **No streaming**: Emulation mode waits for complete response
3. **Parsing brittleness**: Malformed JSON might not be caught
4. **Output truncation**: Large outputs are truncated (necessary for small models)

## References

- PR #4813: Original implementation with llama_cpp library
- `toolshim.rs`: Alternative approach using second model (rejected)
- `ollama.rs`: Similar pattern with GOOSE_MODE checking
