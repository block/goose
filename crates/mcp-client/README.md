## Testing stdio transport

```bash
cargo run -p mcp-client --example stdio
```

## Testing SSE transport

1. Start the MCP server in one terminal: `fastmcp run -t sse echo.py`
2. Run the client example in new terminal: `cargo run -p mcp-client --example sse`

