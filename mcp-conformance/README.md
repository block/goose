# Goose MCP conformance harness

The upstream MCP conformance tester invokes `src/driver.ts`, appending its server URL and setting `MCP_CONFORMANCE_SCENARIO`. The driver translates that scenario into a JSON probe script and runs Goose:

```bash
goose mcp-probe <target> --script <path|->
```

`target` may be an HTTP(S) MCP endpoint or a stdio command. Script tool names are raw MCP names; the probe applies Goose's internal extension scope.

## Script format

```json
{
  "steps": [
    { "action": "listTools" },
    { "action": "listPrompts" },
    { "action": "listResources" },
    { "action": "callTool", "name": "echo", "arguments": { "message": "hello" } }
  ],
  "elicitation": { "action": "accept", "content": { "answer": "yes" } }
}
```

Elicitation actions are `accept` (with explicit `content`), `acceptSchemaDefaults`, `decline`, and `cancel`.

Without `--script`, the command remains useful as a manual probe and lists tools, prompts, and resources.

## Conformance

```bash
source ../bin/activate-hermit
just mcp-conformance 2025-11-25 all
```

Set `GOOSE_BIN` when invoking the driver directly to select another Goose binary.
