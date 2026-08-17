'use strict';

/**
 * Single source of truth for Avocado Work desktop release asset names.
 * CI, the in-app GitHub updater, and the website download page must all
 * derive filenames from this module (or the same GOOSE_BUNDLE_NAME rules).
 *
 * Release assets are versioned: passing a version produces filenames like
 * `Avocado Work-1.45.0.dmg`. The on-disk `.app` bundle and its output
 * directory are ALSO versioned (so /Applications shows the version), while
 * the executable name and userData identity stay stable for auto-update
 * continuity (see main.ts `STABLE_APP_IDENTITY`).
 */

function getBundleName() {
  return process.env.GOOSE_BUNDLE_NAME || 'Avocado Work';
}

function normalizeVersion(version) {
  if (!version) {
    return '';
  }
  return String(version).replace(/^v/, '').trim();
}

/** `-1.45.0` when a version is supplied, otherwise `` (unversioned/local build). */
function versionSuffix(version) {
  const v = normalizeVersion(version);
  return v ? `-${v}` : '';
}

function getReleaseAssets(bundleName = getBundleName(), version = '') {
  const v = normalizeVersion(version);
  const s = versionSuffix(v);
  return {
    bundleName,
    version: v,
    macArm64: {
      website: `${bundleName}${s}.dmg`,
      update: `${bundleName}${s}.zip`,
      appDir: `${bundleName}${s}-darwin-arm64`,
      appBundle: `${bundleName}${s}.app`,
      updateManifestName: `${bundleName}${s}-darwin-arm64.zip`,
    },
    macX64: {
      website: `${bundleName}${s}_intel_mac.dmg`,
      update: `${bundleName}${s}_intel_mac.zip`,
      appDir: `${bundleName}${s}-darwin-x64`,
      appBundle: `${bundleName}${s}.app`,
      updateManifestName: `${bundleName}${s}-darwin-x64.zip`,
    },
    winX64: {
      website: `${bundleName}-Setup${s}-x64.exe`,
      update: `${bundleName}-Setup${s}-x64.exe`,
      portableZip: `${bundleName}${s}-win32-x64.zip`,
      appDir: `${bundleName}${s}-win32-x64`,
      exe: 'avocado-work.exe',
      squirrelName: 'avocado-work',
    },
  };
}

function allWebsiteFilenames(bundleName = getBundleName(), version = '') {
  const assets = getReleaseAssets(bundleName, version);
  return [assets.macArm64.website, assets.macX64.website, assets.winX64.website];
}

function allUpdateFilenames(bundleName = getBundleName(), version = '') {
  const assets = getReleaseAssets(bundleName, version);
  return [assets.macArm64.update, assets.macX64.update, assets.winX64.update];
}

function allReleaseFilenames(bundleName = getBundleName(), version = '') {
  const assets = getReleaseAssets(bundleName, version);
  return [
    assets.macArm64.website,
    assets.macArm64.update,
    assets.macX64.website,
    assets.macX64.update,
    assets.winX64.website,
    assets.winX64.portableZip,
  ];
}

function forbiddenGooseArtifactPattern() {
  // Matches Goose-named desktop release artifacts (not CLI goose-*.tar.* archives).
  return /\bGoose(\.zip|_intel_mac\.zip|-darwin-|-win32-|\.app|\.dmg|-Setup)/;
}

/**
 * Version-agnostic matchers for a downloaded/release asset name. Because assets
 * are versioned, the in-app updater and website must match by shape rather than
 * an exact filename. GitHub replaces spaces in asset names with periods, so the
 * separator between bundle-name tokens is treated as `[ .]`.
 */
function assetMatchers(bundleName = getBundleName()) {
  // Escape regex metachars, then allow space OR period between words (GitHub sanitizes spaces to dots).
  const escaped = bundleName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&').replace(/ /g, '[ .]');
  // The version suffix is "-<semver>"; GitHub also sanitizes spaces to periods.
  const ver = '(?:[-_. ]?[0-9][0-9A-Za-z.+-]*)?';
  return {
    macArm64Website: new RegExp(`^${escaped}${ver}\\.dmg$`, 'i'),
    macX64Website: new RegExp(`^${escaped}${ver}_intel_mac\\.dmg$`, 'i'),
    macArm64Update: new RegExp(`^${escaped}${ver}\\.zip$`, 'i'),
    macX64Update: new RegExp(`^${escaped}${ver}_intel_mac\\.zip$`, 'i'),
    winWebsite: new RegExp(`^${escaped}-Setup${ver}-x64\\.exe$`, 'i'),
  };
}

module.exports = {
  getBundleName,
  normalizeVersion,
  versionSuffix,
  getReleaseAssets,
  allWebsiteFilenames,
  allUpdateFilenames,
  allReleaseFilenames,
  forbiddenGooseArtifactPattern,
  assetMatchers,
};

// CLI helper so shell/CI can read versioned names without duplicating rules:
//   node scripts/release-assets.js <version> [--json | --field macArm64.website]
if (require.main === module) {
  const argv = process.argv.slice(2);
  let version = '';
  let field = '';
  let asJson = false;
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === '--json') {
      asJson = true;
    } else if (arg === '--field') {
      field = argv[++i] || '';
    } else if (!version) {
      version = arg;
    }
  }
  const assets = getReleaseAssets(getBundleName(), version || process.env.RELEASE_VERSION || '');
  if (field) {
    const value = field.split('.').reduce((acc, key) => (acc == null ? acc : acc[key]), assets);
    process.stdout.write(value == null ? '' : String(value));
  } else if (asJson) {
    process.stdout.write(JSON.stringify(assets, null, 2));
  } else {
    process.stdout.write(JSON.stringify(assets));
  }
}
