# Error Injection Proxy Provider for Goose

The Error Injection Proxy Provider allows you to inject various error conditions into a running Goose instance without modifying code or recompiling. This is perfect for testing error handling, debugging issues, and demonstrating resilience.

## How It Works

The `error_proxy` provider acts as a transparent proxy that:
1. Wraps any real provider (OpenAI, Anthropic, etc.)
2. Checks a control file before each API call
3. Either passes through to the real provider OR returns a simulated error
4. Can be controlled dynamically at runtime via the control file

## Quick Start

### 1. Start Goose with Error Proxy

```bash
# Set up the proxy to wrap your provider
export GOOSE_PROVIDER=error_proxy
export ERROR_PROXY_TARGET_PROVIDER=openai  # or anthropic, google, etc.
export ERROR_PROXY_CONTROL_FILE=/tmp/goose-error-control.json

# Set your actual provider credentials
export OPENAI_API_KEY=your-actual-api-key

# Start goose
goose session start
```

### 2. Control Errors from Another Terminal

```bash
# Enable rate limit errors every 3rd call
./examples/goose-error-control.py enable rate_limit --pattern every_nth --nth 3

# Enable random context errors (30% chance)
./examples/goose-error-control.py enable context_exceeded --pattern random --probability 0.3

# Disable all errors
./examples/goose-error-control.py disable

# Check current status
./examples/goose-error-control.py status
```

## Error Types

- **rate_limit**: Simulates API rate limiting with configurable retry delays
- **context_exceeded**: Simulates context length exceeded errors
- **server_error**: Simulates 500 server errors
- **auth_error**: Simulates authentication failures
- **timeout**: Simulates request timeouts

## Error Patterns

- **every_nth**: Inject error every N calls (e.g., every 3rd call)
- **random**: Random errors with specified probability (0.0 to 1.0)
- **burst**: Burst of N consecutive errors, then normal
- **continuous**: Always error when enabled
- **once**: Error once then automatically disable

## Control File Format

The control file is a JSON file that configures error injection:

```json
{
  "enabled": true,
  "error_type": "rate_limit",
  "pattern": "every_nth",
  "nth": 3,
  "retry_after_seconds": 60,
  "target_models": ["gpt-4", "gpt-4-turbo"],
  "custom_message": "Custom error message for testing"
}
```

## Advanced Usage

### Presets for Common Scenarios

```bash
# Simulate a flaky service (20% random failures)
./examples/goose-error-control.py preset flaky

# Simulate an overloaded API (rate limits)
./examples/goose-error-control.py preset overloaded

# Simulate a broken service (continuous errors)
./examples/goose-error-control.py preset broken

# Simulate slow/timeout responses
./examples/goose-error-control.py preset slow
```

### Target Specific Models

```bash
# Only inject errors for GPT-4 models
./examples/goose-error-control.py enable rate_limit \
  --target-models gpt-4 gpt-4-turbo \
  --message "GPT-4 rate limit simulation"
```

### Watch Mode

Monitor the control file for changes in real-time:

```bash
./examples/goose-error-control.py watch
```

### Programmatic Control

Control from Python:
```python
import json

# Enable errors
with open('/tmp/goose-error-control.json', 'w') as f:
    json.dump({
        "enabled": True,
        "error_type": "rate_limit",
        "pattern": "every_nth",
        "nth": 5
    }, f)
```

Control from shell:
```bash
echo '{"enabled": true, "error_type": "server_error", "pattern": "random", "probability": 0.1}' > /tmp/goose-error-control.json
```

## Use Cases

### Testing Error Handling
```bash
# Start your test
./examples/goose-error-control.py enable server_error --pattern burst --burst-count 3

# Run your application
# Verify it handles the errors gracefully

# Disable errors
./examples/goose-error-control.py disable
```

### Load Testing
```bash
# Simulate rate limits under load
./examples/goose-error-control.py enable rate_limit \
  --pattern every_nth --nth 10 \
  --retry-after 30
```

### Debugging Context Issues
```bash
# Test how your app handles context length errors
./examples/goose-error-control.py enable context_exceeded \
  --pattern random --probability 0.1 \
  --message "Testing context handling"
```

### Demo Resilience
```bash
# Show how your app recovers from failures
./examples/goose-error-control.py preset flaky
```

## Implementation Details

The error proxy provider:
- Is registered as `error_proxy` in the provider factory
- Reads the control file before each API call
- Maintains counters for pattern-based injection
- Supports both streaming and non-streaming APIs
- Passes through all provider capabilities when not erroring

## Files

- `crates/goose/src/providers/error_proxy.rs` - The proxy provider implementation
- `examples/goose-error-control.py` - CLI tool for controlling errors
- `examples/error-proxy-demo.sh` - Interactive demo script
- Control file (default: `/tmp/goose-error-control.json`) - Runtime configuration

## Environment Variables

- `ERROR_PROXY_TARGET_PROVIDER` - The actual provider to wrap (e.g., "openai")
- `ERROR_PROXY_CONTROL_FILE` - Path to the control file (default: /tmp/goose-error-control.json)
- All environment variables required by the target provider (e.g., OPENAI_API_KEY)

## Tips

1. **Start with errors disabled**: The proxy creates a default disabled configuration if the control file doesn't exist
2. **Use watch mode for debugging**: See exactly when errors are triggered
3. **Reset counters**: Delete and recreate the control file to reset pattern counters
4. **Combine with logging**: Use `RUST_LOG=debug` to see when errors are injected
5. **Test in isolation**: Use target_models to test specific model behaviors

## Troubleshooting

**Goose doesn't see the proxy provider:**
- Make sure `GOOSE_PROVIDER=error_proxy` is set
- Check that goose is built with the error_proxy module

**Errors aren't being injected:**
- Verify the control file path matches between goose and the control script
- Check that `"enabled": true` in the control file
- Use `status` command to verify configuration
- Check pattern settings (e.g., nth value, probability)

**Wrong provider is being used:**
- Verify `ERROR_PROXY_TARGET_PROVIDER` is set correctly
- Ensure target provider credentials are configured

## Example Session

```bash
# Terminal 1: Start goose with error proxy
export GOOSE_PROVIDER=error_proxy
export ERROR_PROXY_TARGET_PROVIDER=openai
export OPENAI_API_KEY=sk-...
goose session start

# Terminal 2: Control errors
./examples/goose-error-control.py status  # Check it's disabled

# Enable errors
./examples/goose-error-control.py enable rate_limit --pattern every_nth --nth 2

# Now in Terminal 1, send messages to goose
# Every 2nd message will fail with a rate limit error

# Terminal 2: Change error type
./examples/goose-error-control.py enable context_exceeded --pattern random --probability 0.5

# Terminal 2: Disable when done
./examples/goose-error-control.py disable
```

This provides a powerful way to test error conditions without modifying your goose installation or provider implementations!
