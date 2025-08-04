# CI Port Configuration

## Overview

The Goose server uses a simplified port binding strategy that relies on Axum's standard port binding behavior. This approach is more reliable and less complex than manual port finding.

## Default Configuration

- **Host**: `0.0.0.0` (binds to all interfaces)
- **Port**: `49152` (starts in dynamic/private port range)

## Environment Variables

### Server Configuration
- `GOOSE_SERVER__HOST`: Server host (default: `0.0.0.0`)
- `GOOSE_SERVER__PORT`: Starting port (default: `49152`)
- `GOOSE_SERVER__SECRET_KEY`: Secret key for API authentication

## Port Binding Strategy

The server uses Axum's standard `tokio::net::TcpListener::bind()` method, which:

1. **Direct Binding**: Attempts to bind directly to the configured port
2. **Standard Error Handling**: Uses Axum's built-in error handling
3. **Simple and Reliable**: No complex port finding logic
4. **Fast Startup**: Minimal overhead during server initialization

## CI Environment Setup

### GitHub Actions Example

```yaml
env:
  GOOSE_SERVER__HOST: "0.0.0.0"
  GOOSE_SERVER__PORT: "49152"
  GOOSE_SERVER__SECRET_KEY: "your-secret-key"
```

### Docker Example

```dockerfile
ENV GOOSE_SERVER__HOST=0.0.0.0
ENV GOOSE_SERVER__PORT=49152
ENV GOOSE_SERVER__SECRET_KEY=your-secret-key
```

## Port Ranges

- **Well-known ports**: 0-1023 (requires root privileges)
- **Registered ports**: 1024-49151 (may be used by other services)
- **Dynamic/Private ports**: 49152-65535 (recommended for CI)

## Troubleshooting

### Common Issues

1. **Port already in use**: The server will fail to start with a clear error message
2. **Permission denied**: Use ports above 1024
3. **Address not available**: Check host configuration

### Debug Information

The server logs basic information about:
- Pricing cache initialization
- Scheduler creation
- Listening address

### Example Log Output

```
INFO: Initializing pricing cache...
INFO: No scheduler type specified, defaulting to legacy scheduler
INFO: Creating legacy scheduler
INFO: listening on 0.0.0.0:49152
```

## Best Practices

1. **Use dynamic port range**: Start with ports above 49152
2. **Keep it simple**: Let Axum handle port binding
3. **Monitor logs**: Check server logs for binding information
4. **Test locally**: Verify configuration works in your local environment

## Migration from Previous Versions

If you were using port `3000` or `8081`, update your configuration:

```bash
# Old configuration
GOOSE_SERVER__PORT=3000

# New configuration
GOOSE_SERVER__PORT=49152
```

## Advantages of Simplified Approach

1. **Reliability**: Uses proven Axum/Tokio networking code
2. **Simplicity**: No complex port finding logic
3. **Performance**: Faster startup with minimal overhead
4. **Maintainability**: Less custom code to maintain
5. **Standard Behavior**: Follows Rust networking best practices

## Error Handling

If the configured port is unavailable, the server will:
1. Fail to start with a clear error message
2. Log the specific binding error
3. Exit with an appropriate error code

This allows CI systems to detect and handle port conflicts appropriately. 