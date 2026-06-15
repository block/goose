const fs = require('node:fs');
const path = require('node:path');
const { execFileSync } = require('node:child_process');
const { resolveMacosBundleMode } = require('./lib/macosBundleMode.cjs');

const repoRoot = path.resolve(__dirname, '..', '..', '..');
const desktopRoot = path.resolve(__dirname, '..');
const entitlementsPath = path.join(desktopRoot, 'entitlements.plist');
const productMetadata = require(path.join(
  repoRoot,
  'distro',
  'security-cn',
  'branding',
  'product-metadata.json'
));

function readArg(flag) {
  const index = process.argv.indexOf(flag);
  if (index === -1 || index + 1 >= process.argv.length) {
    return undefined;
  }
  return process.argv[index + 1];
}

function resolveBundlePath() {
  const explicitBundlePath = readArg('--app');
  if (explicitBundlePath) {
    return path.resolve(explicitBundlePath);
  }

  const arch = readArg('--arch') || process.env.ELECTRON_ARCH || 'arm64';
  const bundleName = process.env.GOOSE_BUNDLE_NAME || productMetadata.productName || 'Goose';
  return path.join(desktopRoot, 'out', `${bundleName}-darwin-${arch}`, `${bundleName}.app`);
}

function ensureBundleExists(bundlePath) {
  if (!fs.existsSync(bundlePath)) {
    console.error(`[finalize-macos-bundle] Bundle not found: ${bundlePath}`);
    process.exit(1);
  }
}

function run(command, args) {
  execFileSync(command, args, { stdio: 'inherit' });
}

function main() {
  const bundlePath = resolveBundlePath();
  const mode = resolveMacosBundleMode(process.env);

  ensureBundleExists(bundlePath);
  run('xattr', ['-cr', bundlePath]);

  if (mode.shouldAdhocResign) {
    run('codesign', [
      '--force',
      '--deep',
      '--sign',
      '-',
      '--entitlements',
      entitlementsPath,
      bundlePath,
    ]);
  }

  run('codesign', ['--verify', '--deep', '--strict', '--verbose=4', bundlePath]);

  console.log(`[finalize-macos-bundle] bundle=${bundlePath}`);
  console.log(`[finalize-macos-bundle] signing_mode=${mode.signingMode}`);
  console.log(`[finalize-macos-bundle] adhoc_resign=${mode.shouldAdhocResign ? 'yes' : 'no'}`);
}

main();
