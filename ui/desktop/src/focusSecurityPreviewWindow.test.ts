import { createRequire } from 'node:module';
import { describe, expect, it } from 'vitest';

const require = createRequire(import.meta.url);
const { buildAppleScriptForPid, selectPreviewElectronPid } = require('../../../scripts/focus-security-preview-window.js');

describe('focus security preview window helper', () => {
  it('selects the repo preview electron process with the repo --dir marker over bare Electron', () => {
    const electronBinary =
      '/repo/ui/node_modules/electron/dist/Electron.app/Contents/MacOS/Electron';
    const repoRoot = '/repo';
    const processTable = [
      `101 ${electronBinary}`,
      `102 ${electronBinary} --type=renderer --foo`,
      `103 ${electronBinary} . --dir ${repoRoot}`,
    ].join('\n');

    expect(selectPreviewElectronPid(processTable, electronBinary, repoRoot)).toBe(103);
  });

  it('builds a System Events pid activation script instead of activating Electron.app directly', () => {
    const script = buildAppleScriptForPid(1234);

    expect(script).toContain('unix id is 1234');
    expect(script).toContain('System Events');
    expect(script).not.toContain('tell application "/');
  });
});
