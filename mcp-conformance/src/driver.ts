#!/usr/bin/env tsx
import { spawn } from "node:child_process";

interface ProbeScript {
  steps: Array<Record<string, unknown>>;
  elicitation?: Record<string, unknown>;
  oauth?: { clientId?: string; clientSecret?: string };
  protocolVersion?: string;
}

function scriptForScenario(scenario: string | undefined): ProbeScript {
  const protocolVersion = process.env.MCP_CONFORMANCE_PROTOCOL_VERSION;
  const withProtocol = (script: ProbeScript): ProbeScript => ({ ...script, protocolVersion });
  const context = process.env.MCP_CONFORMANCE_CONTEXT
    ? JSON.parse(process.env.MCP_CONFORMANCE_CONTEXT) as Record<string, string>
    : {};
  switch (scenario) {
    case "tools_call":
      return withProtocol({ steps: [{ action: "callTool", name: "add_numbers", arguments: { a: 2, b: 3 } }] });
    case "elicitation-sep1034-client-defaults":
      return withProtocol({
        steps: [{ action: "callTool", name: "test_client_elicitation_defaults", arguments: {} }],
        elicitation: { action: "acceptSchemaDefaults" },
      });
    case "auth/scope-step-up":
      return withProtocol({ steps: [{ action: "callTool", name: "test-tool", arguments: {} }] });
    case "sse-retry":
      return withProtocol({ steps: [{ action: "callTool", name: "test_reconnection", arguments: {} }] });
    case "auth/pre-registration":
      return withProtocol({
        steps: [{ action: "listTools" }],
        oauth: { clientId: context.client_id, clientSecret: context.client_secret },
      });
    case "sep-2322-client-request-state":
      return withProtocol({
        steps: [
          { action: "callTool", name: "test_mrtr_echo_state", arguments: {} },
          { action: "callTool", name: "test_mrtr_no_state", arguments: {} },
          { action: "callTool", name: "test_mrtr_unrelated", arguments: {} },
          { action: "callTool", name: "test_mrtr_no_result_type", arguments: {} },
        ],
        elicitation: { action: "accept", content: { confirmed: true } },
      });
    case "http-custom-headers":
      return withProtocol({
        steps: (context.toolCalls as unknown as Array<Record<string, unknown>>).map((call) => ({
          action: "callTool",
          ...call,
        })),
      });
    case "http-invalid-tool-headers":
      return withProtocol({ steps: [{ action: "callTool", name: "valid_tool", arguments: {} }] });
    default:
      return withProtocol({ steps: [{ action: "listTools" }, { action: "listPrompts" }, { action: "listResources" }] });
  }
}

const args = process.argv.slice(2);
if (args.length !== 1) {
  console.error("usage: driver.ts <server-url-or-stdio-command>");
  process.exit(2);
}

const goose = process.env.GOOSE_BIN ?? "../target/debug/goose";
const child = spawn(goose, ["mcp-probe", args[0], "--script", "-"], {
  env: { ...process.env, GOOSE_OAUTH_AUTOMATIC_CALLBACK: "1" },
  stdio: ["pipe", "inherit", "inherit"],
});
child.stdin.end(JSON.stringify(scriptForScenario(process.env.MCP_CONFORMANCE_SCENARIO)));
child.on("close", (code) => process.exit(code ?? 1));
