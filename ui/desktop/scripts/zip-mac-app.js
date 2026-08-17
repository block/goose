#!/usr/bin/env node

'use strict';

// Zips the packaged macOS .app into the update zip electron-updater consumes,
// deriving the (possibly versioned) directory / bundle / zip names from the
// single source of truth. Replaces the inline BUNDLE_NAME logic that used to
// live in package.json so versioned names stay in one place.
//
//   node scripts/zip-mac-app.js --arch arm64
//   node scripts/zip-mac-app.js --arch x64

const { execFileSync } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');
const { getReleaseAssets, getBundleName } = require('./release-assets.js');

function parseArch(argv) {
  const idx = argv.indexOf('--arch');
  const arch = idx >= 0 ? argv[idx + 1] : 'arm64';
  if (arch !== 'arm64' && arch !== 'x64') {
    console.error(`Unsupported --arch "${arch}" (expected arm64 or x64)`);
    process.exit(1);
  }
  return arch;
}

const arch = parseArch(process.argv.slice(2));
const version = process.env.RELEASE_VERSION || '';
const assets = getReleaseAssets(getBundleName(), version);
const target = arch === 'x64' ? assets.macX64 : assets.macArm64;

const appDir = path.join('out', target.appDir);
if (!fs.existsSync(path.join(appDir, target.appBundle))) {
  console.error(`Missing packaged app: ${path.join(appDir, target.appBundle)}`);
  process.exit(1);
}

console.log(`Zipping ${target.appBundle} -> ${target.update}`);
execFileSync(
  'ditto',
  ['-c', '-k', '--sequesterRsrc', '--keepParent', target.appBundle, target.update],
  { cwd: appDir, stdio: 'inherit' }
);
