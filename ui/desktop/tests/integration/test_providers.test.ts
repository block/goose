/**
 * Provider smoke tests — normal mode (direct tool calls).
 *
 * Ported from scripts/test_providers.sh.  Each available provider/model pair
 * gets its own test that spawns `goose run` with the developer builtin, asks
 * the model to read files via the shell tool, and validates the output.
 */

import { test, expect, beforeAll } from 'vitest';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { buildGoose, discoverTestCases, runGoose, type TestCase } from './test_providers_lib';

const BUILTINS = 'developer';
const TEST_CONTENT = 'test-content-abc123';

let gooseBin: string;
let testFile: string;

beforeAll(() => {
  gooseBin = buildGoose();

  const targetDir = path.resolve(process.cwd(), '..', '..', 'target');
  fs.mkdirSync(targetDir, { recursive: true });
  testFile = path.join(targetDir, 'test-content.txt');
  fs.writeFileSync(testFile, TEST_CONTENT + '\n');
});

const allCases = discoverTestCases();
const available = allCases.filter((tc) => tc.available && !tc.flaky);
const flaky = allCases.filter((tc) => tc.available && tc.flaky);
const skipped = allCases.filter((tc) => !tc.available);

async function runNormalTest(tc: TestCase): Promise<void> {
  const testdir = fs.mkdtempSync(path.join(os.tmpdir(), 'goose-test-'));

  try {
    let prompt: string;
    let tokenA: string | undefined;
    let tokenB: string | undefined;

    if (tc.agentic) {
      fs.copyFileSync(testFile, path.join(testdir, 'test-content.txt'));
      prompt = 'read ./test-content.txt and output its contents exactly';
    } else {
      tokenA = `smoke-alpha-${Math.floor(Math.random() * 32768)}`;
      tokenB = `smoke-bravo-${Math.floor(Math.random() * 32768)}`;
      fs.writeFileSync(path.join(testdir, 'part-a.txt'), tokenA + '\n');
      fs.writeFileSync(path.join(testdir, 'part-b.txt'), tokenB + '\n');
      prompt =
        'Use the shell tool to cat ./part-a.txt and ./part-b.txt, then reply with ONLY the contents of both files, one per line, nothing else.';
    }

    const output = await runGoose(gooseBin, testdir, prompt, BUILTINS, {
      GOOSE_PROVIDER: tc.provider,
      GOOSE_MODEL: tc.model,
    });

    if (tc.agentic) {
      expect(
        output.toLowerCase(),
        `Expected model output to contain "${TEST_CONTENT}"\n\nFull output:\n${output}`
      ).toContain(TEST_CONTENT.toLowerCase());
    } else {
      const shellToolPattern = /(shell \| developer)|(▸.*shell)/;
      expect(
        shellToolPattern.test(output),
        `Expected model to use shell tool\n\nFull output:\n${output}`
      ).toBe(true);
      expect(
        output,
        `Expected output to contain token from part-a.txt (${tokenA})\n\nFull output:\n${output}`
      ).toContain(tokenA);
      expect(
        output,
        `Expected output to contain token from part-b.txt (${tokenB})\n\nFull output:\n${output}`
      ).toContain(tokenB);
    }
  } finally {
    fs.rmSync(testdir, { recursive: true, force: true });
  }
}

if (available.length > 0) {
  test.each(available)('$provider / $model', async (tc) => {
    await runNormalTest(tc);
  });
}

if (flaky.length > 0) {
  test.each(flaky)('$provider / $model (flaky — allowed to fail)', async (tc) => {
    try {
      await runNormalTest(tc);
    } catch (err) {
      console.warn(`Flaky test ${tc.provider}/${tc.model} failed (allowed): ${err}`);
    }
  });
}

if (skipped.length > 0) {
  test.skip.each(skipped)('$provider / $model — $skippedReason', () => {});
}
