---
title: Tool Shim Guide
sidebar_label: Tool Shim
sidebar_position: 61
---

Most LLM providers support native "function calling" or "tool calling", where the model returns a structured request (tool name and arguments) that goose can execute directly. Some models - especially smaller local models, reasoning models, and models served through OpenAI-compatible routers - don't return tool calls in that structured format. Instead they describe the tool call as plain text in their response.

When that happens, goose can't act on the model's intent and the session stalls. The **tool shim** is goose's mechanism for handling these models.

## When to enable the tool shim

Enable the tool shim if you're using a model that:

- Doesn't natively support tool/function calling (common with many local models run via Ollama)
- Intermittently emits tool calls as plain text instead of using the provider's structured `tool_calls` field - for example, output like `functions.shell:0 <|tool_call_argument_begin|> {"command": "ls"}` or `Using tool: shell\n{"name": "shell", "arguments": {...}}` showing up in the assistant's response instead of an actual tool invocation
- Mixes reasoning/`<think>` tags with tool call output (some DeepSeek and Kimi models do this), which can confuse strict tool-call parsing

A good signal that you need the tool shim: goose's reply contains raw tool-call-looking text and the corresponding tool never actually runs.

## How it works

When `GOOSE_TOOLSHIM` is enabled, goose changes how it handles tool calls for that model:

1. **System prompt augmentation** - goose appends the available tools (names, schemas, descriptions) to the system prompt and instructs the model to describe tool usage as a JSON object (`{"name": "...", "arguments": {...}}`) in its text response, rather than relying on the provider's native tool-calling API.
2. **Tool/conversation history as text** - prior tool requests and tool results in the conversation are rewritten as plain text instead of structured tool-call content, since some providers reject tool-call content blocks when no tools are declared in the request.
3. **Parsing the response** - after the model responds, goose scans the text for tool call intent in roughly this order:
   - Tokenized formats some models emit natively, such as `<|tool_calls_section_begin|> <|tool_call_begin|> functions.shell:0 <|tool_call_argument_begin|> {...} <|tool_call_end|> <|tool_calls_section_end|>`
   - Inline JSON tool directives like `Using tool: shell\n{"name": "shell", "arguments": {...}}`
   - If neither of those is found, goose falls back to a separate **interpreter model** that converts the free-form text into structured tool calls
4. **Cleanup** - any raw tool-call markup or JSON directives are stripped from the text shown to the user, and the parsed tool call is added to the message as a normal tool request.

The tokenized and inline-JSON parsing (step 3a/3b) happens regardless of which interpreter backend is configured, so even without a working interpreter model, goose can often recover tool calls from models that emit one of those known formats.

## Configuration

### `GOOSE_TOOLSHIM`

Set to `true` or `1` (case-insensitive) to enable the tool shim. Default is `false`.

```bash
export GOOSE_TOOLSHIM=true
```

### Choosing an interpreter backend

If the direct parsers (tokenized markers, inline JSON) don't find a tool call, goose falls back to an interpreter model. The backend used for this is controlled by `GOOSE_TOOLSHIM_BACKEND`:

- `ollama` (default) - uses a local Ollama server's [structured outputs](https://ollama.com/blog/structured-outputs) API to interpret the model's text into tool calls
- `local` / `llama.cpp` - uses goose's local inference (llama.cpp) backend instead of Ollama

```bash
export GOOSE_TOOLSHIM_BACKEND=ollama   # or "local" / "llama.cpp"
```

### Ollama interpreter: `GOOSE_TOOLSHIM_OLLAMA_MODEL`

When using the Ollama backend (the default), this sets which Ollama model is used as the interpreter. If unset, goose defaults to `mistral-nemo`.

```bash
export GOOSE_TOOLSHIM_OLLAMA_MODEL=llama3.2
```

Make sure the interpreter model is pulled and available on your Ollama server:

```bash
ollama pull mistral-nemo
# or, if you set GOOSE_TOOLSHIM_OLLAMA_MODEL
ollama pull llama3.2
```

For best results interpreting longer tool calls, run the Ollama server with an increased context length:

```bash
OLLAMA_CONTEXT_LENGTH=32768 ollama serve
```

goose connects to Ollama using the `OLLAMA_HOST` config value (or `localhost:11434` by default).

### Local (llama.cpp) interpreter: `GOOSE_TOOLSHIM_MODEL`

When `GOOSE_TOOLSHIM_BACKEND` is set to `local` or `llama.cpp`, set `GOOSE_TOOLSHIM_MODEL` to the model goose's local inference should use as the interpreter. This is required for the local backend - there's no built-in default.

```bash
export GOOSE_TOOLSHIM_BACKEND=local
export GOOSE_TOOLSHIM_MODEL=<your-local-model>
```

## Non-Ollama / custom providers

The tool shim is a goose-side feature - it operates on the conversation before it's sent to the provider and on the response after it comes back, so it isn't tied to any single provider. If you're using a custom OpenAI-compatible provider (for example, a router proxying to models like Kimi K2.5 or DeepSeek on Bedrock) and that provider's responses include tool calls as plain text rather than structured `tool_calls`, enabling `GOOSE_TOOLSHIM=true` applies the same parsing and system-prompt changes described above to that provider's traffic.

The Ollama- and llama.cpp-based interpreters (`GOOSE_TOOLSHIM_BACKEND`) are only used as a last resort when the direct text parsers don't recognize the model's output format. If your model consistently emits one of the recognized formats (tokenized `<|tool_call_begin|>...<|tool_call_end|>` markers, or inline `{"name": ..., "arguments": ...}` JSON), the tool shim can recover those calls without needing an interpreter model or an Ollama server running at all.

## Troubleshooting

**Tools stop working / goose's response includes text that looks like a tool call:**

1. Check whether the model you're using natively supports tool calling. If not, set `GOOSE_TOOLSHIM=true`.
2. If you're already using the tool shim and the interpreter backend is `ollama`, confirm the interpreter model (default `mistral-nemo`, or your `GOOSE_TOOLSHIM_OLLAMA_MODEL`) is pulled and your Ollama server is reachable.
3. If you're on the `local`/`llama.cpp` backend, make sure `GOOSE_TOOLSHIM_MODEL` is set - this backend has no default model.

## See also

- [Environment Variables](/docs/guides/environment-variables) for the full reference of `GOOSE_TOOLSHIM` and related variables
- [Ollama Tool Shim](/docs/experimental/ollama) for Ollama-specific setup details
