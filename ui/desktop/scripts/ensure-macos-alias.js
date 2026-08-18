#!/usr/bin/env node

'use strict';

const { spawnSync } = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

if (process.platform !== 'darwin') {
  process.exit(0);
}

function findPackageRoot(packageName) {
  const candidates = [
    path.resolve(__dirname, '../../node_modules', packageName),
    path.resolve(__dirname, '../node_modules', packageName),
  ];
  return candidates.find((candidate) => fs.existsSync(path.join(candidate, 'package.json')));
}

function hasNativeBinding(root, relativePath) {
  return fs.existsSync(path.join(root, relativePath));
}

function rebuild(root, label) {
  console.log(`ensure-macos-alias: compiling ${label}...`);
  const result = spawnSync('npx', ['--yes', 'node-gyp@11', 'rebuild'], {
    cwd: root,
    stdio: 'inherit',
    shell: process.platform === 'win32',
  });
  if (result.status !== 0) {
    process.exit(result.status === null ? 1 : result.status);
  }
}

const macosAlias = findPackageRoot('macos-alias');
if (macosAlias && !hasNativeBinding(macosAlias, path.join('build', 'Release', 'volume.node'))) {
  rebuild(macosAlias, 'macos-alias');
}

const fsXattr = findPackageRoot('fs-xattr');
if (fsXattr && !hasNativeBinding(fsXattr, path.join('build', 'Release', 'xattr.node'))) {
  rebuild(fsXattr, 'fs-xattr');
}
