#!/usr/bin/env node
/**
 * Vibes Parallel Assembler
 *
 * Assembles multiple riff apps in parallel.
 *
 * Usage:
 *   node scripts/assemble-all.js riff-1 riff-2 riff-3 ...
 *
 * Each directory should contain app.jsx, output goes to index.html in same dir.
 */

import { readFileSync, writeFileSync, existsSync } from 'fs';
import { resolve } from 'path';
import { TEMPLATES } from './lib/paths.js';

const PLACEHOLDER = '// __VIBES_APP_CODE__';
const templatePath = TEMPLATES.vibesBasic;

// Read template once
if (!existsSync(templatePath)) {
  console.error(`Template not found: ${templatePath}`);
  process.exit(1);
}
const template = readFileSync(templatePath, 'utf8');

if (!template.includes(PLACEHOLDER)) {
  console.error(`Template missing placeholder: ${PLACEHOLDER}`);
  process.exit(1);
}

// Get riff directories from args
const riffDirs = process.argv.slice(2);

if (riffDirs.length === 0) {
  console.error('Usage: node scripts/assemble-all.js riff-1 riff-2 ...');
  process.exit(1);
}

// Assemble all in parallel
const results = await Promise.all(
  riffDirs.map(async (dir) => {
    const appPath = resolve(dir, 'app.jsx');
    const outputPath = resolve(dir, 'index.html');

    if (!existsSync(appPath)) {
      return { dir, success: false, error: `App not found: ${appPath}` };
    }

    try {
      const appCode = readFileSync(appPath, 'utf8').trim();
      const output = template.replace(PLACEHOLDER, appCode);
      writeFileSync(outputPath, output);
      return { dir, success: true };
    } catch (e) {
      return { dir, success: false, error: e.message };
    }
  })
);

// Report results
let hasErrors = false;
for (const r of results) {
  if (r.success) {
    console.log(`Assembled: ${r.dir}/index.html`);
  } else {
    console.error(`Failed: ${r.dir} - ${r.error}`);
    hasErrors = true;
  }
}

console.log(`\nAssembled ${results.filter(r => r.success).length}/${results.length} riffs.`);
process.exit(hasErrors ? 1 : 0);
