import { createRequire } from 'node:module';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';
import { resolveUpdateAssetName, updateAssetPattern } from '../githubUpdater';

const require = createRequire(import.meta.url);
const releaseAssetsPath = resolve(
  dirname(fileURLToPath(import.meta.url)),
  '../../../scripts/release-assets.js'
);

describe('resolveUpdateAssetName', () => {
  it('resolves darwin/win32 assets to the release-assets contract', () => {
    const { getReleaseAssets } = require(releaseAssetsPath) as {
      getReleaseAssets: (bundleName?: string) => {
        macArm64: { update: string };
        macX64: { update: string };
        winX64: { update: string };
      };
    };
    const assets = getReleaseAssets('Avocado Work');

    expect(resolveUpdateAssetName('darwin', 'arm64', 'Avocado Work')).toBe(assets.macArm64.update);
    expect(resolveUpdateAssetName('darwin', 'x64', 'Avocado Work')).toBe(assets.macX64.update);
    expect(resolveUpdateAssetName('win32', 'x64', 'Avocado Work')).toBe(assets.winX64.update);
  });

  it('does not resolve Goose-named assets for Avocado Work', () => {
    const names = [
      resolveUpdateAssetName('darwin', 'arm64', 'Avocado Work'),
      resolveUpdateAssetName('darwin', 'x64', 'Avocado Work'),
      resolveUpdateAssetName('win32', 'x64', 'Avocado Work'),
    ];
    for (const name of names) {
      expect(name.startsWith('Goose')).toBe(false);
      expect(name).not.toMatch(/^Goose(\.zip|_intel_mac\.zip|-win32-|-Setup)/);
    }
  });
});

describe('updateAssetPattern', () => {
  it('matches versioned update assets and GitHub period-sanitized names', () => {
    const arm = updateAssetPattern('darwin', 'arm64', 'Avocado Work');
    expect(arm.test('Avocado Work.zip')).toBe(true);
    expect(arm.test('Avocado.Work-1.45.0.zip')).toBe(true);
    // The arm64 zip pattern must not swallow the Intel zip.
    expect(arm.test('Avocado Work-1.45.0_intel_mac.zip')).toBe(false);

    const intel = updateAssetPattern('darwin', 'x64', 'Avocado Work');
    expect(intel.test('Avocado.Work-1.45.0_intel_mac.zip')).toBe(true);

    const win = updateAssetPattern('win32', 'x64', 'Avocado Work');
    expect(win.test('Avocado Work-Setup-x64.exe')).toBe(true);
    expect(win.test('Avocado.Work-Setup-1.45.0-x64.exe')).toBe(true);
  });
});
