import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const workspaceYaml = readFileSync(
  resolve(dirname(fileURLToPath(import.meta.url)), '../../../pnpm-workspace.yaml'),
  'utf8'
);

describe('pnpm workspace native builds', () => {
  it('does not auto-compile macOS-only natives that break Windows CI', () => {
    expect(workspaceYaml).toMatch(/ignoredBuiltDependencies:[\s\S]*\bfs-xattr\b/);
    expect(workspaceYaml).toMatch(/ignoredBuiltDependencies:[\s\S]*\bmacos-alias\b/);
    expect(workspaceYaml).not.toMatch(/^\s*fs-xattr:\s*true\s*$/m);
    expect(workspaceYaml).not.toMatch(/^\s*macos-alias:\s*true\s*$/m);
  });
});
