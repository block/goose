#!/usr/bin/env node

// Resolves the path to the goose-acp-server binary from the platform-specific
// optional dependency. Writes the result to a JSON file that the CLI reads at
// startup so it can spawn the server automatically.

import { writeFileSync, mkdirSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);

const PLATFORMS = {
  "darwin-arm64": "@goose-ai/acp-server-darwin-arm64",
  "darwin-x64": "@goose-ai/acp-server-darwin-x64",
  "linux-arm64": "@goose-ai/acp-server-linux-arm64",
  "linux-x64": "@goose-ai/acp-server-linux-x64",
  "win32-x64": "@goose-ai/acp-server-win32-x64",
};

function warnLoud(message) {
  const indent = "    ";
  const lines = [
    "",
    `${indent}⚠️  @goose-ai/cli WARNING`,
    "",
    ...message.split("\n").map((l) => `${indent}${l}`),
    "",
  ];
  process.stderr.write(lines.join("\n") + "\n");
}

const key = `${process.platform}-${process.arch}`;
const pkg = PLATFORMS[key];

if (!pkg) {
  warnLoud(
    `No prebuilt goose-acp-server binary for ${key}.\n` +
      `You will need to provide a server URL manually with --server.`,
  );
  process.exit(0);
}

let binaryPath;
try {
  const pkgDir = dirname(require.resolve(`${pkg}/package.json`));
  const binName =
    process.platform === "win32" ? "goose-acp-server.exe" : "goose-acp-server";
  binaryPath = join(pkgDir, "bin", binName);
} catch {
  warnLoud(
    `Optional dependency ${pkg} not installed (wrong platform?).\n` +
      `You will need to provide a server URL manually with --server.`,
  );
  process.exit(0);
}

const outDir = join(__dirname, "..");
mkdirSync(outDir, { recursive: true });
writeFileSync(
  join(outDir, "server-binary.json"),
  JSON.stringify({ binaryPath }, null, 2) + "\n",
);

console.log(`@goose-ai/cli: found native server binary at ${binaryPath}`);
