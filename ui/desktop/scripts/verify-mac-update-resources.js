#!/usr/bin/env node

const fs = require('node:fs');
const path = require('node:path');

function fail(message) {
  console.error(message);
  process.exit(1);
}

const appPath = process.argv[2];
if (!appPath) {
  fail('Usage: node scripts/verify-mac-update-resources.js <path-to-app>');
}

const updateConfigPath = path.join(appPath, 'Contents', 'Resources', 'app-update.yml');
if (!fs.existsSync(updateConfigPath)) {
  fail(`Missing ${updateConfigPath}`);
}

const updateConfig = fs.readFileSync(updateConfigPath, 'utf8');
const requiredLines = [
  'provider: github',
  'owner: aaif-goose',
  'repo: goose',
  'updaterCacheDirName: goose-updater',
];

for (const line of requiredLines) {
  if (!updateConfig.split(/\r?\n/).includes(line)) {
    fail(`${updateConfigPath} is missing "${line}"`);
  }
}

console.log(`${updateConfigPath} is present and valid`);
