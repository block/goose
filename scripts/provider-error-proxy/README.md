# Provider Error Proxy

A network-level HTTP proxy for simulating provider errors when testing Goose's error handling and retry logic.

## Features

- **Network-level interception**: No changes to Goose's Rust code required
- **Multi-provider support**: Works with OpenAI, Anthropic, Google, OpenRouter, Tetrate, and Databricks
- **Streaming support**: Handles both regular HTTP responses and streaming responses (SSE/chunked)
- **Configurable error injection**: Inject errors at specified intervals (e.g., every 5th request)
- **Provider-specific errors**: Returns appropriate error codes and formats for each provider
- **Transparent proxying**: Forwards all other requests unchanged to the real provider APIs

## Installation

This project uses `uv` for Python dependency management. From the `scripts/provider-error-proxy` directory:

```bash
# Install dependencies (uv will handle this automatically)
uv sync
```

## Usage

### Basic Usage

Start the proxy with default settings (port 8888, error every 5 requests):

```bash
uv run proxy.py
```

### Custom Configuration

```bash
# Use a different port
uv run proxy.py --port 9000

# Change error injection frequency (every 3 requests)
uv run proxy.py --error-interval 3

# Combine options
uv run proxy.py --port 9000 --error-interval 10
```

### Configure Goose to Use the Proxy

Set environment variables to redirect provider traffic through the proxy:

```bash
export OPENAI_HOST=http://localhost:8888
export ANTHROPIC_HOST=http://localhost:8888
export GOOGLE_HOST=http://localhost:8888
export OPENROUTER_HOST=http://localhost:8888
export TETRATE_HOST=http://localhost:8888
export DATABRICKS_HOST=http://localhost:8888
```

Then run Goose normally. The proxy will intercept requests and inject errors at the configured interval.

## How It Works

1. **Request Interception**: The proxy listens on localhost and receives all provider API requests
2. **Provider Detection**: Identifies which provider the request is for based on headers and paths
3. **Error Injection**: Every Nth request (configurable) returns a provider-specific error response
4. **Streaming Support**: Detects streaming responses (SSE/chunked) and streams them through transparently
5. **Transparent Forwarding**: All other requests are forwarded to the actual provider API unchanged

### Streaming Details

The proxy automatically detects and handles streaming responses by:
- Checking for `text/event-stream` content type (Server-Sent Events)
- Checking for `stream` in the content type
- Checking for `chunked` transfer encoding
- Using `StreamResponse` to forward chunks in real-time without buffering

This means streaming completions from providers like OpenAI, Anthropic, and Databricks work seamlessly through the proxy.

## Error Types by Provider

The proxy returns realistic error responses for each provider:

- **OpenAI**: 429 Rate Limit Error
- **Anthropic**: 529 Overloaded Error  
- **Google**: 503 Service Unavailable
- **OpenRouter**: 429 Rate Limit Error
- **Tetrate**: 503 Service Unavailable
- **Databricks**: 429 Rate Limit Exceeded

## Example Output

```
2025-10-08 18:00:00 - __main__ - INFO - ============================================================
2025-10-08 18:00:00 - __main__ - INFO - 🔧 Provider Error Proxy
2025-10-08 18:00:00 - __main__ - INFO - ============================================================
2025-10-08 18:00:00 - __main__ - INFO - Port: 8888
2025-10-08 18:00:00 - __main__ - INFO - Error interval: every 5 requests
2025-10-08 18:00:00 - __main__ - INFO - 
2025-10-08 18:00:00 - __main__ - INFO - To use with Goose, set these environment variables:
2025-10-08 18:00:00 - __main__ - INFO -   export OPENAI_HOST=http://localhost:8888
...
2025-10-08 18:00:05 - __main__ - INFO - 📨 POST /v1/chat/completions -> openai
2025-10-08 18:00:05 - __main__ - INFO - ✅ Forwarding request #1
2025-10-08 18:00:05 - __main__ - INFO - ✅ Proxied response: 200
...
2025-10-08 18:00:25 - __main__ - INFO - 📨 POST /v1/chat/completions -> openai
2025-10-08 18:00:25 - __main__ - WARNING - 🔴 Injecting error on request #5
2025-10-08 18:00:25 - __main__ - WARNING - 💥 Returning 429 error for openai
```

## Development

The proxy is built with `aiohttp` for async HTTP handling. Key components:

- `ErrorProxy`: Main proxy class that handles request interception and error injection
- `detect_provider()`: Identifies which provider based on headers/paths
- `should_inject_error()`: Determines when to inject errors based on request count
- `handle_request()`: Main request handler that either proxies or returns errors

## Testing

To test the proxy:

1. Start the proxy: `uv run proxy.py --error-interval 2`
2. Configure Goose to use the proxy (set environment variables)
3. Run Goose and observe error handling behavior
4. Check proxy logs to see which requests were forwarded vs. errored

## Future Enhancements

Potential improvements for future versions:

- **Interactive mode**: stdin interface to manually trigger errors
- **Error type selection**: Choose which error types to inject
- **Request filtering**: Only inject errors for specific endpoints
- **Statistics**: Track success/error rates and response times
- **Configuration file**: YAML config for complex error scenarios
