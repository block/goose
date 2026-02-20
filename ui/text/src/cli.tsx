#!/usr/bin/env node
import { render } from "ink";
import meow from "meow";
import { spawn } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import App from "./app.js";

const cli = meow(
  `
  Usage
    $ goose

  Options
    --server, -s  Server URL (default: auto-launch bundled server)
    --text, -t    Send a single prompt and exit
`,
  {
    importMeta: import.meta,
    flags: {
      server: { type: "string", shortFlag: "s" },
      text: { type: "string", shortFlag: "t" },
    },
  },
);

const DEFAULT_PORT = 3284;
const DEFAULT_URL = `http://127.0.0.1:${DEFAULT_PORT}`;

function findServerBinary(): string | null {
  const __dirname = dirname(fileURLToPath(import.meta.url));

  // When installed as a package, server-binary.json is written by postinstall
  const candidates = [
    join(__dirname, "..", "server-binary.json"),
    join(__dirname, "server-binary.json"),
  ];

  for (const candidate of candidates) {
    try {
      const data = JSON.parse(readFileSync(candidate, "utf-8"));
      return data.binaryPath ?? null;
    } catch {
      // not found here, try next
    }
  }

  return null;
}

async function waitForServer(url: string, timeoutMs = 10_000): Promise<void> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    try {
      const res = await fetch(`${url}/status`);
      if (res.ok) return;
    } catch {
      // server not ready yet
    }
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(
    `Server did not become ready at ${url} within ${timeoutMs}ms`,
  );
}

let serverProcess: ReturnType<typeof spawn> | null = null;

async function main() {
  let serverUrl = cli.flags.server;

  if (!serverUrl) {
    const binary = findServerBinary();
    if (binary) {
      serverProcess = spawn(binary, ["--port", String(DEFAULT_PORT)], {
        stdio: "ignore",
        detached: false,
      });

      serverProcess.on("error", (err) => {
        console.error(`Failed to start goose-acp-server: ${err.message}`);
        process.exit(1);
      });

      try {
        await waitForServer(DEFAULT_URL);
      } catch (err) {
        console.error((err as Error).message);
        serverProcess.kill();
        process.exit(1);
      }

      serverUrl = DEFAULT_URL;
    } else {
      // No binary found — fall back to default URL and hope something is running
      serverUrl = DEFAULT_URL;
    }
  }

  const { waitUntilExit } = render(
    <App serverUrl={serverUrl} initialPrompt={cli.flags.text} />,
  );

  await waitUntilExit();
}

// Clean up the spawned server on exit
function cleanup() {
  if (serverProcess && !serverProcess.killed) {
    serverProcess.kill();
  }
}

process.on("exit", cleanup);
process.on("SIGINT", () => {
  cleanup();
  process.exit(0);
});
process.on("SIGTERM", () => {
  cleanup();
  process.exit(0);
});

main().catch((err) => {
  console.error(err);
  cleanup();
  process.exit(1);
});
