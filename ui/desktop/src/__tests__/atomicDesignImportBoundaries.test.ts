import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';

function read(path: string): string {
  return readFileSync(path, 'utf8');
}

function listFilesRecursive(dir: string): string[] {
  const out: string[] = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    const stat = statSync(full);
    if (stat.isDirectory()) {
      out.push(...listFilesRecursive(full));
    } else if (full.endsWith('.ts') || full.endsWith('.tsx')) {
      out.push(full);
    }
  }
  return out;
}

describe('atomic design import boundaries', () => {
  it('organisms must not import pages', () => {
    const files = listFilesRecursive('src/components/organisms');

    const violations: Array<{ file: string; lines: string[] }> = [];

    // Covers e.g.
    // - import X from '../pages/Foo'
    // - import X from '../../pages/Foo'
    // - import X from 'src/components/pages/Foo' (if we ever introduce aliases)
    const forbidden = /from\s+['"][^'\"]*(?:\/|\.)pages\//;

    for (const file of files) {
      const text = read(file);
      if (!forbidden.test(text)) continue;

      const lines = text
        .split('\n')
        .map((line, i) => ({ line, n: i + 1 }))
        .filter(({ line }) => forbidden.test(line))
        .map(({ line, n }) => `${n}: ${line.trim()}`);

      violations.push({ file, lines });
    }

    if (violations.length > 0) {
      const msg = violations
        .map((v) => `- ${v.file}\n${v.lines.map((l) => `  ${l}`).join('\n')}`)
        .join('\n');
      expect.fail(`Organisms importing Pages is forbidden:\n${msg}`);
    }
  });
});
