import path from 'node:path';
import { createRequire } from 'node:module';
import { describe, expect, it } from 'vitest';

const require = createRequire(import.meta.url);
const launcher = require('../scripts/electron-forge-start-dev.js');

describe('electron-forge dev launcher', () => {
  const desktopRoot = '/repo/ui/desktop';

  it('resolves the desktop root from the script location instead of the caller cwd', () => {
    expect(launcher.getDesktopRoot('/repo/ui/desktop/scripts')).toBe(path.resolve(desktopRoot));
  });

  it('normalizes pnpm forwarded args that include a leading separator token', () => {
    expect(launcher.normalizeForwardedArgs(['--', '--dir', '/repo'])).toEqual([
      '--dir',
      '/repo',
    ]);
  });

  it('prepends the electron-forge start verb and preserves forwarded args', () => {
    expect(launcher.getElectronForgeStartArgs(desktopRoot, ['--dir', '/repo'])).toEqual([
      'start',
      desktopRoot,
      '--',
      '--dir',
      '/repo',
    ]);
  });

  it('avoids duplicating the passthrough separator for pnpm forwarded args', () => {
    expect(launcher.getElectronForgeStartArgs(desktopRoot, ['--', '--dir', '/repo'])).toEqual([
      'start',
      desktopRoot,
      '--',
      '--dir',
      '/repo',
    ]);
  });

  it('omits the passthrough separator when no app args are forwarded', () => {
    expect(launcher.getElectronForgeStartArgs(desktopRoot, [])).toEqual(['start', desktopRoot]);
  });
});
