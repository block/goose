/**
 * Provider smoke tests — code execution mode (JS batching).
 *
 * Ported from scripts/test_providers_code_exec.sh.  Each available
 * (non-agentic) provider/model pair gets its own test that spawns `goose run`
 * with the memory + code_execution builtins and validates that the
 * code_execution tool was invoked.
 */

import { test, expect, beforeAll } from 'vitest';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { buildGoose, discoverTestCases, runGoose, type TestCase } from './test_providers_lib';

const BUILTINS = 'memory,code_execution';

let gooseBin: string;

beforeAll(() => {
  gooseBin = buildGoose();
});

const allCases = discoverTestCases({ skipAgentic: true });
const available = allCases.filter((tc) => tc.available && !tc.flaky);
const flaky = allCases.filter((tc) => tc.available && tc.flaky);
const skipped = allCases.filter((tc) => !tc.available);

async function runCodeExecTest(tc: TestCase): Promise<void> {
  const testdir = fs.mkdtempSync(path.join(os.tmpdir(), 'goose-codeexec-'));

  try {
    const prompt =
      "Store a memory with category 'test' and data 'hello world', then retrieve all memories from category 'test'.";

    const output = await runGoose(gooseBin, testdir, prompt, BUILTINS, {
      GOOSE_PROVIDER: tc.provider,
      GOOSE_MODEL: tc.model,
    });

    // Matches: "execute_typescript | code_execution", "get_function_details | code_execution",
    //           "tool call | execute", "tool calls | execute" (old format)
    //           "▸ execute N tool call" (new format with tool_graph)
    //           "▸ execute_typescript" (plain tool name in output)
    const codeExecPattern =
      /(execute_typescript \| code_execution)|(get_function_details \| code_execution)|(tool calls? \| execute)|(▸.*execute.*tool call)|(▸ execute_typescript)/;

    expect(
      codeExecPattern.test(output),
      `Expected code_execution tool to be called\n\nFull output:\n${output}`
    ).toBe(true);
  } finally {
    fs.rmSync(testdir, { recursive: true, force: true });
  }
}

if (available.length > 0) {
  test.each(available)('$provider / $model', async (tc) => {
    await runCodeExecTest(tc);
  });
}

if (flaky.length > 0) {
  test.each(flaky)('$provider / $model (flaky — allowed to fail)', async (tc) => {
    try {
      await runCodeExecTest(tc);
    } catch (err) {
      console.warn(`Flaky test ${tc.provider}/${tc.model} failed (allowed): ${err}`);
    }
  }, 120_000);
}

if (skipped.length > 0) {
  test.skip.each(skipped)('$provider / $model — $skippedReason', () => {});
}
