'use strict';

/**
 * Single source of truth for Avocado Work desktop release asset names.
 * CI, the in-app GitHub updater, and the website download page must all
 * derive filenames from this module (or the same GOOSE_BUNDLE_NAME rules).
 */

function getBundleName() {
  return process.env.GOOSE_BUNDLE_NAME || 'Avocado Work';
}

function getReleaseAssets(bundleName = getBundleName()) {
  return {
    bundleName,
    macArm64: {
      website: `${bundleName}.dmg`,
      update: `${bundleName}.zip`,
      appDir: `${bundleName}-darwin-arm64`,
      appBundle: `${bundleName}.app`,
      updateManifestName: `${bundleName}-darwin-arm64.zip`,
    },
    macX64: {
      website: `${bundleName}_intel_mac.dmg`,
      update: `${bundleName}_intel_mac.zip`,
      appDir: `${bundleName}-darwin-x64`,
      appBundle: `${bundleName}.app`,
      updateManifestName: `${bundleName}-darwin-x64.zip`,
    },
    winX64: {
      website: `${bundleName}-Setup-x64.exe`,
      update: `${bundleName}-Setup-x64.exe`,
      portableZip: `${bundleName}-win32-x64.zip`,
      appDir: `${bundleName}-win32-x64`,
      exe: 'avocado-work.exe',
      squirrelName: 'avocado-work',
    },
  };
}

function allWebsiteFilenames(bundleName = getBundleName()) {
  const assets = getReleaseAssets(bundleName);
  return [assets.macArm64.website, assets.macX64.website, assets.winX64.website];
}

function allUpdateFilenames(bundleName = getBundleName()) {
  const assets = getReleaseAssets(bundleName);
  return [assets.macArm64.update, assets.macX64.update, assets.winX64.update];
}

function allReleaseFilenames(bundleName = getBundleName()) {
  const assets = getReleaseAssets(bundleName);
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

module.exports = {
  getBundleName,
  getReleaseAssets,
  allWebsiteFilenames,
  allUpdateFilenames,
  allReleaseFilenames,
  forbiddenGooseArtifactPattern,
};
