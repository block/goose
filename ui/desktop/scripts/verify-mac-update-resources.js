#!/usr/bin/env node

'use strict';

const fs = require('node:fs');
const path = require('node:path');

function fail(message) {
  console.error(message);
  process.exit(1);
}

function normalizeYaml(text) {
  return text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0 && !line.startsWith('#'));
}

function expectedConfigLines() {
  const owner = process.env.GITHUB_OWNER;
  const repo = process.env.GITHUB_REPO;
  if (owner && repo) {
    return [
      'provider: github',
      `owner: ${owner}`,
      `repo: ${repo}`,
      'updaterCacheDirName: avocado-work-updater',
    ];
  }

  const sourcePath = path.join(__dirname, '..', 'src', 'app-update.yml');
  if (!fs.existsSync(sourcePath)) {
    fail(`Missing source update config: ${sourcePath}`);
  }
  return normalizeYaml(fs.readFileSync(sourcePath, 'utf8'));
}

const appPath = process.argv[2];
if (!appPath) {
  fail('Usage: node scripts/verify-mac-update-resources.js <path-to-app>');
}

const updateConfigPath = path.join(appPath, 'Contents', 'Resources', 'app-update.yml');
if (!fs.existsSync(updateConfigPath)) {
  fail(`Missing ${updateConfigPath}`);
}

const updateConfig = normalizeYaml(fs.readFileSync(updateConfigPath, 'utf8'));
const requiredLines = expectedConfigLines();

for (const line of requiredLines) {
  if (!updateConfig.includes(line)) {
    fail(`${updateConfigPath} is missing "${line}"`);
  }
}

if (updateConfig.includes('owner: aaif-goose') || updateConfig.includes('repo: goose')) {
  fail(
    `${updateConfigPath} still targets upstream aaif-goose/goose; expected the fork update identity`
  );
}

console.log(`${updateConfigPath} is present and valid`);
