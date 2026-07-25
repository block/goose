# Goose MCP conformance harness

This directory contains a TypeScript driver for running upstream MCP conformance data against Goose's normal MCP extension/session path.

The driver reads JSON conformance fixtures and invokes:

```bash
goose mcp-probe '<stdio MCP server command>'
```

That command starts a lightweight Goose session with no provider/LLM, loads the supplied stdio MCP server as an extension, and reports discovered tools/prompts/resources as JSON. Optional fixture fields can also request a direct tool call.

## Usage

```bash
source ../bin/activate-hermit
cargo build -p goose-cli
cd mcp-conformance
pnpm install
pnpm conformance /path/to/modelcontextprotocol/conformance/tests
```

Set `GOOSE_BIN` to test a specific Goose binary:

```bash
GOOSE_BIN=../target/debug/goose pnpm conformance ./fixtures
```

## Fixture shape

The driver accepts a JSON file, an array of cases, or an object with a `tests` array. Each case must define either `command` or `server.command`/`server.args`:

```json
{
  "id": "initialize",
  "name": "initializes against the conformance server",
  "server": {
    "command": "node",
    "args": ["/path/to/conformance/server.js", "--case", "initialize"]
  },
  "expected": {
    "toolResult": null
  }
}
```

To call a tool after initialization:

```json
{
  "id": "call-echo",
  "name": "calls echo",
  "command": ["node", "./server.js"],
  "callTool": "conformance__echo",
  "arguments": { "message": "hello" }
}
```
