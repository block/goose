# Provider Error Proxy

A Python HTTP proxy for testing Goose's provider error handling and retry logic.

## Quick Start

```bash
# Start the proxy
cd scripts/provider-error-proxy
uv run proxy.py --error-interval 5

# In another terminal, configure Goose
export OPENAI_HOST=http://localhost:8888
export ANTHROPIC_HOST=http://localhost:8888

# Run Goose - every 5th request will get an error
goose session start "tell me a joke"
```

## Files

- **`proxy.py`** - Main proxy server with streaming support
- **`README.md`** - Full documentation
- **`STREAMING.md`** - Details on streaming support
- **`example.sh`** - Example usage script
- **`test_proxy.py`** - Test script for the proxy
- **`pyproject.toml`** - Python project config (uses `uv`)

## Key Features

✅ **Streaming Support** - Handles SSE and chunked responses transparently  
✅ **Multi-Provider** - OpenAI, Anthropic, Google, OpenRouter, Tetrate, Databricks  
✅ **No Code Changes** - Works via environment variables  
✅ **Configurable** - Control error frequency and port  
✅ **Realistic Errors** - Provider-specific error responses  

## How It Works

1. Proxy listens on localhost:8888
2. Set `PROVIDER_HOST` env vars to point to proxy
3. Every Nth request returns a provider-specific error
4. All other requests forwarded to real provider
5. Streaming responses handled transparently

See `README.md` for complete documentation.
