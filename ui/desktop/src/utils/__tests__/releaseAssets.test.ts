import { createRequire } from 'node:module';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, describe, expect, it } from 'vitest';

const require = createRequire(import.meta.url);
const releaseAssetsPath = resolve(
  dirname(fileURLToPath(import.meta.url)),
  '../../../scripts/release-assets.js'
);

type ReleaseAssetsModule = {
  getBundleName: () => string;
  getReleaseAssets: (bundleName?: string) => {
    bundleName: string;
    macArm64: { website: string; update: string; appDir: string };
    macX64: { website: string; update: string; appDir: string };
    winX64: {
      website: string;
      update: string;
      portableZip: string;
      squirrelName: string;
      exe: string;
    };
  };
  allReleaseFilenames: (bundleName?: string) => string[];
  forbiddenGooseArtifactPattern: () => RegExp;
};

function clearReleaseAssetsCache() {
  try {
    delete require.cache[require.resolve(releaseAssetsPath)];
  } catch {
    // Module not loaded yet.
  }
}

function loadReleaseAssets(): ReleaseAssetsModule {
  clearReleaseAssetsCache();
  return require(releaseAssetsPath) as ReleaseAssetsModule;
}

describe('release-assets contract', () => {
  const originalBundleName = process.env.GOOSE_BUNDLE_NAME;

  afterEach(() => {
    if (originalBundleName === undefined) {
      delete process.env.GOOSE_BUNDLE_NAME;
    } else {
      process.env.GOOSE_BUNDLE_NAME = originalBundleName;
    }
    clearReleaseAssetsCache();
  });

  it('derives all desktop release names from GOOSE_BUNDLE_NAME', () => {
    delete process.env.GOOSE_BUNDLE_NAME;
    const {
      getBundleName,
      getReleaseAssets,
      allReleaseFilenames,
    } = loadReleaseAssets();

    expect(getBundleName()).toBe('Avocado Work');
    const assets = getReleaseAssets();
    expect(assets.macArm64.website).toBe('Avocado Work.dmg');
    expect(assets.macArm64.update).toBe('Avocado Work.zip');
    expect(assets.macX64.website).toBe('Avocado Work_intel_mac.dmg');
    expect(assets.macX64.update).toBe('Avocado Work_intel_mac.zip');
    expect(assets.winX64.website).toBe('Avocado Work-Setup-x64.exe');
    expect(assets.winX64.update).toBe('Avocado Work-Setup-x64.exe');
    expect(assets.winX64.portableZip).toBe('Avocado Work-win32-x64.zip');
    expect(assets.winX64.squirrelName).toBe('avocado-work');
    expect(assets.winX64.exe).toBe('avocado-work.exe');
    expect(allReleaseFilenames()).toEqual(
      expect.arrayContaining([
        'Avocado Work.dmg',
        'Avocado Work.zip',
        'Avocado Work_intel_mac.dmg',
        'Avocado Work_intel_mac.zip',
        'Avocado Work-Setup-x64.exe',
        'Avocado Work-win32-x64.zip',
      ])
    );
  });

  it('uses a custom bundle name without hard-coding Avocado Work', () => {
    process.env.GOOSE_BUNDLE_NAME = 'Test Brand';
    const { getBundleName, getReleaseAssets, allReleaseFilenames } = loadReleaseAssets();

    expect(getBundleName()).toBe('Test Brand');
    const assets = getReleaseAssets();
    expect(assets.macArm64.website).toBe('Test Brand.dmg');
    expect(assets.macArm64.update).toBe('Test Brand.zip');
    expect(assets.macX64.update).toBe('Test Brand_intel_mac.zip');
    expect(assets.winX64.website).toBe('Test Brand-Setup-x64.exe');
    expect(allReleaseFilenames()).not.toEqual(
      expect.arrayContaining([expect.stringContaining('Avocado Work')])
    );
  });

  it('matches forbidden Goose desktop artifact names', () => {
    const { forbiddenGooseArtifactPattern } = loadReleaseAssets();
    const pattern = forbiddenGooseArtifactPattern();
    expect(pattern.test('Goose.zip')).toBe(true);
    expect(pattern.test('Goose_intel_mac.zip')).toBe(true);
    expect(pattern.test('out/Goose-darwin-arm64/Goose.app')).toBe(true);
    expect(pattern.test('Goose-win32-x64.zip')).toBe(true);
    expect(pattern.test('goose-darwin-arm64.tar.bz2')).toBe(false);
    expect(pattern.test('Avocado Work.zip')).toBe(false);
  });

  it('mac update manifest names come from the asset map', () => {
    delete process.env.GOOSE_BUNDLE_NAME;
    const manifestPath = resolve(
      dirname(fileURLToPath(import.meta.url)),
      '../../../scripts/generate-mac-update-manifest.js'
    );
    delete require.cache[require.resolve(manifestPath)];
    const { manifestFilePairs } = require(manifestPath) as {
      manifestFilePairs: () => Array<{ sourceName: string; updateName: string }>;
    };

    expect(manifestFilePairs()).toEqual([
      {
        sourceName: 'Avocado Work.zip',
        updateName: 'Avocado Work-darwin-arm64.zip',
      },
      {
        sourceName: 'Avocado Work_intel_mac.zip',
        updateName: 'Avocado Work-darwin-x64.zip',
      },
    ]);
  });
});
