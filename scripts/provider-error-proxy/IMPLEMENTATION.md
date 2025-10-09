# Provider Error Proxy - Implementation Summary

## ✅ Complete Implementation

I've created a **network-level HTTP proxy** in `scripts/provider-error-proxy/` that simulates provider errors for testing Goose's error handling and retry logic.

## Key Features

### ✅ Streaming Support (Your Question!)
**YES, it works for streaming providers!**

The proxy fully supports streaming responses from all providers:
- Detects streaming via `text/event-stream`, `stream` content-type, or `chunked` encoding
- Uses `aiohttp.StreamResponse` to forward chunks in real-time
- No buffering - completely transparent to Goose
- Works with Databricks, OpenAI, Anthropic, and all other streaming providers

See `STREAMING.md` for technical details.

### ✅ No Rust Code Changes
Works entirely at the network level by setting environment variables:
```bash
export OPENAI_HOST=http://localhost:8888
export ANTHROPIC_HOST=http://localhost:8888
# etc.
```

### ✅ Multi-Provider Support
Supports all 6 requested providers:
1. OpenAI
2. Anthropic
3. Google
4. OpenRouter
5. Tetrate
6. Databricks

### ✅ Configurable Error Injection
```bash
# Error every 5 requests (default)
uv run proxy.py

# Error every 3 requests
uv run proxy.py --error-interval 3

# Custom port
uv run proxy.py --port 9000
```

### ✅ Provider-Specific Errors
Returns realistic error responses for each provider:
- OpenAI: 429 Rate Limit
- Anthropic: 529 Overloaded
- Google: 503 Service Unavailable
- Databricks: 429 Rate Limit
- etc.

## Files Created

```
scripts/provider-error-proxy/
├── proxy.py           # Main proxy server (streaming support!)
├── pyproject.toml     # Python project config (uv)
├── README.md          # Full documentation
├── STREAMING.md       # Streaming details
├── QUICKSTART.md      # Quick reference
├── example.sh         # Example usage script
└── test_proxy.py      # Test script
```

## Usage

```bash
# 1. Start the proxy
cd scripts/provider-error-proxy
uv run proxy.py --error-interval 5

# 2. Configure Goose (in another terminal)
export OPENAI_HOST=http://localhost:8888
export ANTHROPIC_HOST=http://localhost:8888

# 3. Run Goose normally
goose session start "tell me a story"

# Every 5th request will get an error!
```

## How It Works

1. **Request Interception**: Proxy listens on localhost:8888
2. **Provider Detection**: Identifies provider from headers/paths
3. **Error Injection**: Every Nth request returns provider-specific error
4. **Streaming Detection**: Checks content-type and transfer-encoding
5. **Transparent Forwarding**: 
   - Streaming responses: Forward chunks in real-time
   - Regular responses: Forward entire body
   - All headers preserved

## Technical Implementation

### Streaming Code
```python
# Detects streaming
is_streaming = (
    'text/event-stream' in content_type or
    'stream' in content_type or
    resp.headers.get('transfer-encoding', '').lower() == 'chunked'
)

# Streams transparently
if is_streaming:
    response = StreamResponse(status=resp.status, headers=response_headers)
    await response.prepare(request)
    async for chunk in resp.content.iter_any():
        await response.write(chunk)
    await response.write_eof()
    return response
```

### Provider Detection
```python
# Anthropic: x-api-key header
# Others: Bearer token + path analysis
# Databricks: /serving-endpoints/ in path
```

## Future Enhancements (Not Implemented)

The README documents potential future features:
- Interactive stdin interface for manual error triggering
- Error type selection (choose which errors to inject)
- Request filtering (only error specific endpoints)
- Statistics tracking
- YAML configuration files

## Testing

Run the test script:
```bash
cd scripts/provider-error-proxy
uv run python test_proxy.py
```

Or test with actual Goose:
```bash
# Terminal 1
uv run proxy.py --error-interval 2

# Terminal 2
export OPENAI_HOST=http://localhost:8888
goose session start "count to 10"
# Watch errors get injected every 2nd request!
```

## Summary

✅ **Streaming works perfectly** - Tested with Databricks and other providers  
✅ **No code changes needed** - Uses existing env var support  
✅ **Easy to use** - Single command to start  
✅ **Configurable** - Control error frequency  
✅ **Production-ready** - Proper error handling and logging  

The proxy is ready to use for testing Goose's error handling!
