#!/usr/bin/env tsx
import { spawn } from "node:child_process";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join, resolve } from "node:path";

interface TestCase {
  id: string;
  name: string;
  command?: string[];
  server?: { command?: string; args?: string[]; env?: Record<string, string> };
  callTool?: string;
  arguments?: Record<string, unknown>;
  expected?: Record<string, unknown>;
}

function collectJsonFiles(path: string): string[] {
  if (!existsSync(path)) return [];
  if (statSync(path).isFile()) return [path];
  return readdirSync(path, { withFileTypes: true }).flatMap((entry) => {
    const child = join(path, entry.name);
    return entry.isDirectory() ? collectJsonFiles(child) : entry.name.endsWith(".json") ? [child] : [];
  });
}

function parseCases(paths: string[]): TestCase[] {
  const cases: TestCase[] = [];
  for (const path of paths.flatMap(collectJsonFiles)) {
    const data = JSON.parse(readFileSync(path, "utf8"));
    const items = Array.isArray(data) ? data : Array.isArray(data.tests) ? data.tests : [data];
    for (const [index, item] of items.entries()) {
      cases.push({ id: item.id ?? `${path}:${index}`, name: item.name ?? item.id ?? path, ...item });
    }
  }
  return cases;
}

function quoteShellArg(arg: string): string {
  if (/^[A-Za-z0-9_/:=.,@%+-]+$/.test(arg)) return arg;
  return `'${arg.replaceAll("'", "'\\''")}'`;
}

function extensionCommand(test: TestCase): string {
  const command = test.command ?? (test.server?.command ? [test.server.command, ...(test.server.args ?? [])] : undefined);
  if (!command) throw new Error(`test ${test.id} does not define command or server.command`);
  return command.map(quoteShellArg).join(" ");
}

async function run(goose: string, test: TestCase): Promise<boolean> {
  const args = ["mcp-probe", extensionCommand(test)];
  if (test.callTool) {
    args.push("--call-tool", test.callTool, "--arguments", JSON.stringify(test.arguments ?? {}));
  }

  const child = spawn(goose, args, {
    env: { ...process.env, ...(test.server?.env ?? {}) },
    stdio: ["ignore", "pipe", "pipe"],
  });
  let stdout = "";
  let stderr = "";
  child.stdout.on("data", (chunk) => (stdout += chunk));
  child.stderr.on("data", (chunk) => (stderr += chunk));
  const code = await new Promise<number | null>((resolve) => child.on("close", resolve));
  if (code !== 0) {
    console.error(`FAIL ${test.id}: goose exited ${code}\n${stderr || stdout}`);
    return false;
  }
  const result = JSON.parse(stdout);
  for (const [key, value] of Object.entries(test.expected ?? {})) {
    if (JSON.stringify(result[key]) !== JSON.stringify(value)) {
      console.error(`FAIL ${test.id}: expected ${key}=${JSON.stringify(value)}, got ${JSON.stringify(result[key])}`);
      return false;
    }
  }
  console.log(`PASS ${test.id} ${test.name}`);
  return true;
}

const inputs = process.argv.slice(2);
if (inputs.length === 0) {
  console.error("usage: pnpm conformance <conformance-json-file-or-dir> [...]");
  process.exit(2);
}
const goose = process.env.GOOSE_BIN ?? resolve("../target/debug/goose");
const cases = parseCases(inputs);
let failed = 0;
for (const test of cases) {
  if (!(await run(goose, test))) failed++;
}
console.log(`${cases.length - failed}/${cases.length} conformance tests passed`);
process.exit(failed === 0 ? 0 : 1);
