import path from 'node:path';
import { createRequire } from 'node:module';
import { describe, expect, it } from 'vitest';

const require = createRequire(import.meta.url);
const launcher = require('../scripts/electron-forge-start-dev.js');

describe('electron-forge dev launcher', () => {
  it('resolves the desktop root from the script location instead of the caller cwd', () => {
    expect(launcher.getDesktopRoot('/repo/ui/desktop/scripts')).toBe(
      path.resolve('/repo/ui/desktop')
    );
  });

  it('prepends the electron-forge start verb and preserves forwarded args', () => {
    expect(launcher.getElectronForgeStartArgs(['--dir', '/repo'])).toEqual([
      'start',
      '--',
      '--dir',
      '/repo',
    ]);
  });

  it('omits the passthrough separator when no app args are forwarded', () => {
    expect(launcher.getElectronForgeStartArgs([])).toEqual(['start']);
  });
});
