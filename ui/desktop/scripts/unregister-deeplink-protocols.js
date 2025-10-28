#!/usr/bin/env node

/*
 * This script unregisters the deeplink protocols for the app.
 * It is intended to be used on macOS only.
 *
 * It is useful when you want to switch between the dev and prod versions of the app,
 * and you want to make sure that the deeplinks are handled by the correct version.
 *
 * Usage:
 * node unregister-deeplink-protocols.js
 */

const { execFile } = require('child_process');
const { platform } = require('os');
const { bundleId } = require('../electron-builder-config.js');

function unregister(bundleId) {
  if (platform() !== 'darwin') {
    console.log('This script is only for macOS');
    return;
  }

  console.log(`Unregistering deeplink protocols for ${bundleId}...`);

  const lsregisterPath =
    '/System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister';

  execFile(lsregisterPath, ['-u', bundleId], (err, stdout, stderr) => {
    if (err) {
      console.error(err);
      return;
    }
    console.log(stdout);
  });
}

unregister(bundleId);
