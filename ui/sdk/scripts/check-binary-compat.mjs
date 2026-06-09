#!/usr/bin/env node
// Compatibility smoke test: boot the freshly-built goose binary via `goose acp`
// and call every read-only ACP method through the freshly-built SDK. The
// generated client validates every response with Zod, so any schema drift
// between the binary and the SDK client fails this check and blocks the
// publish.
//
// Run with:
//   GOOSE_BINARY=/path/to/goose node ui/sdk/scripts/check-binary-compat.mjs
//
// Or via package script:
//   GOOSE_BINARY=/path/to/goose pnpm --filter @aaif/goose-sdk run check:compat

import { spawn } from "node:child_process";
import { mkdtempSync, rmSync, existsSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { Readable, Writable } from "node:stream";

const SDK_ROOT = resolve(new URL("..", import.meta.url).pathname);
const SDK_DIST = join(SDK_ROOT, "dist");

if (!existsSync(SDK_DIST)) {
  console.error(
    `[compat] expected built SDK at ${SDK_DIST} — run pnpm build first`,
  );
  process.exit(1);
}

const GOOSE_BINARY = process.env.GOOSE_BINARY;
if (!GOOSE_BINARY || !existsSync(GOOSE_BINARY)) {
  console.error(
    `[compat] GOOSE_BINARY must point to a built goose binary (got: ${GOOSE_BINARY ?? "<unset>"})`,
  );
  process.exit(1);
}

const { GooseClient } = await import(join(SDK_DIST, "goose-client.js"));
const { PROTOCOL_VERSION, ndJsonStream } = await import(
  "@agentclientprotocol/sdk"
);

// Each entry is a read-only ACP method we expect to succeed against a fresh,
// unconfigured goose install. Platform-specific skips keep hardware-sensitive
// checks from turning local environment quirks into publish blockers.
const READ_ONLY_CHECKS = [
  {
    name: "providersList_unstable",
    call: (c) => c.goose.providersList_unstable({ providerIds: [] }),
  },
  {
    name: "providersCatalogList_unstable",
    call: (c) => c.goose.providersCatalogList_unstable({}),
  },
  {
    name: "providersSetupCatalogList_unstable",
    call: (c) => c.goose.providersSetupCatalogList_unstable({}),
  },
  {
    name: "defaultsRead_unstable",
    call: (c) => c.goose.defaultsRead_unstable({}),
  },
  {
    name: "preferencesRead_unstable",
    call: (c) => c.goose.preferencesRead_unstable({}),
  },
  {
    name: "sourcesList_unstable",
    call: (c) => c.goose.sourcesList_unstable({}),
  },
  {
    name: "dictationConfig_unstable",
    skipIf: () => process.platform === "darwin",
    skipReason:
      "skipped on macOS because local-inference Metal probing can panic before returning a schema response",
    call: (c) => c.goose.dictationConfig_unstable({}),
  },
  {
    name: "dictationModelsList_unstable",
    call: (c) => c.goose.dictationModelsList_unstable({}),
  },
  {
    name: "configExtensionsList_unstable",
    call: (c) => c.goose.configExtensionsList_unstable({}),
  },
];

const sandbox = mkdtempSync(join(tmpdir(), "goose-compat-"));
const env = {
  ...process.env,
  HOME: sandbox,
  XDG_CONFIG_HOME: join(sandbox, ".config"),
  XDG_DATA_HOME: join(sandbox, ".local/share"),
  XDG_STATE_HOME: join(sandbox, ".local/state"),
  XDG_CACHE_HOME: join(sandbox, ".cache"),
  GOOSE_CONFIG_DIR: join(sandbox, ".config/goose"),
};

console.log(`[compat] using binary: ${GOOSE_BINARY}`);
console.log(`[compat] sandbox HOME: ${sandbox}`);
console.log(`[compat] binary size: ${statSync(GOOSE_BINARY).size} bytes`);

const child = spawn(GOOSE_BINARY, ["acp"], {
  stdio: ["pipe", "pipe", "inherit"],
  env,
});

let exitedEarly = false;
child.on("exit", (code, signal) => {
  if (!exitedEarly) {
    console.error(
      `[compat] goose acp exited unexpectedly (code=${code} signal=${signal})`,
    );
  }
});
child.on("error", (err) => {
  console.error(`[compat] failed to spawn goose acp: ${err.message}`);
  process.exit(1);
});

const stream = ndJsonStream(
  Writable.toWeb(child.stdin),
  Readable.toWeb(child.stdout),
);

const client = new GooseClient(
  () => ({
    requestPermission: async () => ({
      outcome: { outcome: "cancelled" },
    }),
    sessionUpdate: async () => {},
  }),
  stream,
);

let failed = 0;
let passed = 0;

const timeout = (ms, label) =>
  new Promise((_, reject) =>
    setTimeout(() => reject(new Error(`${label} timed out after ${ms}ms`)), ms),
  );

try {
  await Promise.race([
    client.initialize({
      protocolVersion: PROTOCOL_VERSION,
      clientInfo: { name: "publish-npm-compat", version: "0.0.0" },
      clientCapabilities: {},
    }),
    timeout(15_000, "initialize"),
  ]);
  console.log("[compat] ✅ initialize");

  for (const check of READ_ONLY_CHECKS) {
    if (check.skipIf?.()) {
      console.log(`[compat] ⏭️ ${check.name} (${check.skipReason})`);
      continue;
    }

    try {
      await Promise.race([check.call(client), timeout(15_000, check.name)]);
      console.log(`[compat] ✅ ${check.name}`);
      passed += 1;
    } catch (err) {
      failed += 1;
      const msg = err instanceof Error ? (err.stack ?? err.message) : String(err);
      console.error(`[compat] ❌ ${check.name}`);
      console.error(indent(msg, "  "));
    }
  }
} finally {
  exitedEarly = true;
  child.kill("SIGTERM");
  try {
    rmSync(sandbox, { recursive: true, force: true });
  } catch {
    // best-effort cleanup
  }
}

if (failed > 0) {
  console.error(
    `\n[compat] ${failed} check(s) failed, ${passed} passed — refusing to publish.`,
  );
  console.error(
    "[compat] This means the SDK's generated client schema doesn't match what",
  );
  console.error(
    "[compat] the goose binary returns. Regenerate the SDK or fix the server DTO.",
  );
  process.exit(1);
}

console.log(`\n[compat] all ${passed} checks passed.`);
process.exit(0);

function indent(s, prefix) {
  return s
    .split("\n")
    .map((line) => prefix + line)
    .join("\n");
}
