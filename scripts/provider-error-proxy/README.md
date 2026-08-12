# Provider Error Proxy

A network-level HTTP proxy for simulating provider errors when testing Goose's error handling and retry logic.

## Features

- **Interactive error injection**: Manually trigger different error types via stdin commands
- **Network-level interception**: No changes to Goose's Rust code required
- **Multi-provider support**: Works with OpenAI, Anthropic, Google, OpenRouter, Tetrate, and Databricks
- **Streaming support**: Handles both regular HTTP responses and streaming responses (SSE/chunked)
- **Provider-specific errors**: Returns appropriate error codes and formats for each provider
- **Transparent proxying**: Forwards all other requests unchanged to the real provider APIs

## Quickstart

```bash
# 1. Start the proxy (from scripts/provider-error-proxy directory)
uv run proxy.py

# 2. In another terminal, configure Goose to use the proxy
export OPENAI_HOST=http://localhost:8888
export ANTHROPIC_HOST=http://localhost:8888
export GOOGLE_HOST=http://localhost:8888
export OPENROUTER_HOST=http://localhost:8888
export TETRATE_HOST=http://localhost:8888
export DATABRICKS_HOST=http://localhost:8888

# For Databricks with OAuth, also set the real host:
export DATABRICKS_REAL_HOST=https://your-workspace.databricks.com

# 3. Run Goose normally
goose session start "tell me a joke"

# 4. In the proxy terminal, use interactive commands:
#    n - No error (pass through) - permanent
#    c - Context length exceeded error
#    r - Rate limit error
#    u - Unknown server error (500)
#    q - Quit
```

## Installation

This project uses `uv` for Python dependency management. From the `scripts/provider-error-proxy` directory:

```bash
# Install dependencies (uv will handle this automatically)
uv sync
```

## Usage

### Starting the Proxy

Start the proxy with default settings (port 8888):

```bash
uv run proxy.py
```

Use a custom port:

```bash
uv run proxy.py --port 9000
```

Start the proxy with an initial error mode (for automated testing):

```bash
# Start with context length error (3 times)
uv run proxy.py --mode "c 3"

# Start with rate limit error (30% of requests)
uv run proxy.py --mode "r 30%"

# Start with server error (all requests)
uv run proxy.py --mode "u *"
```

Command-line options:
- `--port PORT` - Port to listen on (default: 8888)
- `--mode COMMAND` - Initial error mode command (e.g., "c 3", "r 30%", "u *", "n")
  - Same syntax as interactive commands
- `--no-stdin` - Disable stdin reader (for background/automated mode)

For automated tests or background usage, combine `--no-stdin` with `--mode`:

```bash
# Run in background for automated testing
uv run proxy.py --mode "c 3" --no-stdin &
PROXY_PID=$!

# ... run your tests ...

# Stop the proxy
kill $PROXY_PID
```

### Interactive Commands

Once the proxy is running, you can control error injection interactively:

- **`n`** - No error (pass through all requests normally) - **permanent mode**
- **`c`** - Context length exceeded error (1 time by default)
  - `c 4` - Inject error 4 times in a row
  - `c 0.3` or `c 30%` - Inject error on 30% of requests
  - `c *` - Inject error on 100% of requests (all requests fail)
- **`r`** - Rate limit error (1 time by default, same modifiers as `c`)
- **`u`** - Unknown server error (500) (1 time by default, same modifiers as `c`)
- **`q`** - Quit the proxy

**Note:** Whitespace is flexible - `c 100%`, `c100%`, `c *`, and `c*` all work the same way.

The proxy will display the current mode and request count after each command.

### Configuring Goose

Set environment variables to redirect provider traffic through the proxy:

```bash
export OPENAI_HOST=http://localhost:8888
export ANTHROPIC_HOST=http://localhost:8888
export GOOGLE_HOST=http://localhost:8888
export OPENROUTER_HOST=http://localhost:8888
export TETRATE_HOST=http://localhost:8888
export DATABRICKS_HOST=http://localhost:8888
```

For providers that require authentication or metadata endpoints (like Databricks with OAuth), you also need to set the real host:

```bash
export DATABRICKS_REAL_HOST=https://your-workspace.databricks.com
```

Then run Goose normally. The proxy will intercept API requests and you can manually trigger errors as needed, while authentication and metadata requests are forwarded to the real provider.

## How It Works

1. **Request Interception**: The proxy listens on localhost and receives all provider API requests
2. **Provider Detection**: Identifies which provider the request is for based on headers and paths
3. **Smart Forwarding**: Authentication, OIDC, and metadata endpoints are always forwarded to the real provider without error injection
4. **Interactive Error Control**: Use stdin commands to control when and what type of errors to inject
5. **Error Injection**: When an error mode is active, API requests return provider-specific error responses
6. **Streaming Support**: Detects streaming responses (SSE/chunked) and streams them through transparently
7. **Transparent Forwarding**: All other requests are forwarded to the actual provider API unchanged

### Streaming Details

The proxy automatically detects and handles streaming responses by:
- Checking for `text/event-stream` content type (Server-Sent Events)
- Using `StreamResponse` to forward chunks in real-time without buffering

This means streaming completions from providers like OpenAI, Anthropic, and Databricks work seamlessly through the proxy.

## Error Types by Provider

The proxy returns realistic error responses for each provider:

### Context Length Exceeded (Command: `c`)
- **OpenAI**: 400 with `context_length_exceeded` error
- **Anthropic**: 400 with "prompt is too long" message
- **Google**: 400 with `INVALID_ARGUMENT` status
- **OpenRouter**: 400 with context length message
- **Tetrate**: 400 with context length error
- **Databricks**: 400 with `INVALID_PARAMETER_VALUE` error

### Rate Limit (Command: `r`)
- **OpenAI**: 429 with `rate_limit_exceeded` error
- **Anthropic**: 429 with `rate_limit_error` type
- **Google**: 429 with `RESOURCE_EXHAUSTED` status
- **OpenRouter**: 429 with rate limit message
- **Tetrate**: 429 with rate limit error
- **Databricks**: 429 with `RATE_LIMIT_EXCEEDED` error

### Server Error (Command: `u`)
- **OpenAI**: 500 with `internal_server_error` error
- **Anthropic**: 529 with `overloaded_error` type
- **Google**: 503 with `UNAVAILABLE` status
- **OpenRouter**: 500 with internal server error
- **Tetrate**: 503 with service unavailable error
- **Databricks**: 500 with `INTERNAL_ERROR` error

## Example Session

```
$ uv run proxy.py
============================================================
🔧 Provider Error Proxy
============================================================
Port: 8888

To use with Goose, set these environment variables:
  export OPENAI_HOST=http://localhost:8888
  export ANTHROPIC_HOST=http://localhost:8888
  ...
============================================================

============================================================
Current mode: ✅ No error (pass through)
Requests handled: 0
============================================================

Commands:
  n      - No error (pass through) - permanent
  c      - Context length exceeded (1 time)
  c 4    - Context length exceeded (4 times)
  c 0.3  - Context length exceeded (30% of requests)
  c 30%  - Context length exceeded (30% of requests)
  c *    - Context length exceeded (100% of requests)
  r      - Rate limit error (1 time)
  u      - Unknown server error (1 time)
  q      - Quit

Enter command: r
============================================================
Current mode: ⏱️  Rate limit exceeded (1 remaining)
Requests handled: 0
============================================================
...
2025-10-09 14:30:15 - __main__ - INFO - 📨 Request #1: POST /v1/chat/completions -> openai
2025-10-09 14:30:15 - __main__ - WARNING - 💥 Injecting RATE_LIMIT error (status 429) for openai

Enter command: n
============================================================
Current mode: ✅ No error (pass through)
Requests handled: 1
============================================================
...
2025-10-09 14:30:20 - __main__ - INFO - 📨 Request #2: POST /v1/chat/completions -> openai
2025-10-09 14:30:20 - __main__ - INFO - ✅ Proxied response: 200
```

## Development

The proxy is built with `aiohttp` for async HTTP handling. Key components:

- `ErrorProxy`: Main proxy class that handles request interception and error injection
- `ErrorMode`: Enum defining the available error injection modes
- `detect_provider()`: Identifies which provider based on headers/paths
- `handle_request()`: Main request handler that either proxies or returns errors
- `stdin_reader()`: Thread that reads interactive commands from stdin

## Testing

To test the proxy:

1. Start the proxy: `uv run proxy.py`
2. Configure Goose to use the proxy (set environment variables)
3. Run Goose in another terminal
4. Use interactive commands to trigger different error types
5. Observe how Goose handles each error type
6. Check proxy logs to see which requests were forwarded vs. errored
