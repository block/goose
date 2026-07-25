#!/usr/bin/env tsx
import { spawn } from "node:child_process";

interface ProbeScript {
  steps: Array<Record<string, unknown>>;
  elicitation?: Record<string, unknown>;
}

function scriptForScenario(scenario: string | undefined): ProbeScript {
  switch (scenario) {
    case "tools_call":
      return { steps: [{ action: "callTool", name: "add_numbers", arguments: { a: 2, b: 3 } }] };
    case "elicitation-sep1034-client-defaults":
      return {
        steps: [{ action: "callTool", name: "test_client_elicitation_defaults", arguments: {} }],
        elicitation: { action: "acceptSchemaDefaults" },
      };
    case "auth/scope-step-up":
      return { steps: [{ action: "callTool", name: "test-tool", arguments: {} }] };
    default:
      return { steps: [{ action: "listTools" }, { action: "listPrompts" }, { action: "listResources" }] };
  }
}

const args = process.argv.slice(2);
if (args.length !== 1) {
  console.error("usage: driver.ts <server-url-or-stdio-command>");
  process.exit(2);
}

const goose = process.env.GOOSE_BIN ?? "../target/debug/goose";
const child = spawn(goose, ["mcp-probe", args[0], "--script", "-"], {
  env: process.env,
  stdio: ["pipe", "inherit", "inherit"],
});
child.stdin.end(JSON.stringify(scriptForScenario(process.env.MCP_CONFORMANCE_SCENARIO)));
child.on("close", (code) => process.exit(code ?? 1));
