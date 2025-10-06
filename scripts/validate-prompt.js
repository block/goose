#!/usr/bin/env node
// validate-prompt.js
// Simple Node.js script to validate the minimal prompt JSON schema used by the
// documentation prompt library. It's intended as a developer convenience and
// is not part of Goose runtime.
// Usage: node scripts/validate-prompt.js path/to/prompt.json
const fs = require('fs');
const path = require('path');

function validatePrompt(filePath) {
  const raw = fs.readFileSync(filePath, 'utf8');
  let data;
  try {
    data = JSON.parse(raw);
  } catch (err) {
    console.error(`JSON parse error for ${filePath}:`, err.message);
    process.exitCode = 2;
    return false;
  }

  const required = ['id','title','description','category','example_prompt','example_result','extensions'];
  const missing = required.filter(k => !(k in data));
  if (missing.length) {
    console.error(`Missing required fields in ${filePath}:`, missing.join(', '));
    process.exitCode = 3;
    return false;
  }

  if (!Array.isArray(data.extensions) || data.extensions.length < 3) {
    console.error(`'extensions' must be an array with 3 or more items (found ${Array.isArray(data.extensions)?data.extensions.length:'not an array'})`);
    process.exitCode = 4;
    return false;
  }

  for (let i=0;i<data.extensions.length;i++){
    const ext = data.extensions[i];
    if (!ext.name || !ext.description || typeof ext.is_builtin === 'undefined' || !Array.isArray(ext.environmentVariables)){
      console.error(`Extension at index ${i} is missing required fields (name, description, is_builtin, environmentVariables)`);
      process.exitCode = 5;
      return false;
    }
  }

  // check title matches filename-ish
  const fname = path.basename(filePath, '.json');
  const normalizedTitle = data.title.toLowerCase().replace(/[^a-z0-9]+/g,'-').replace(/(^-|-$)/g,'');
  if (normalizedTitle !== fname) {
    console.warn(`Warning: normalized title '${normalizedTitle}' does not match filename '${fname}'. This is recommended but not required.`);
  }

  console.log(`VALID: ${filePath} — ${data.title}`);
  return true;
}

const args = process.argv.slice(2);
if (!args.length) {
  console.error('Usage: validate-prompt.js <path-to-prompt.json>');
  process.exitCode = 1;
  process.exit(1);
}

const ok = validatePrompt(args[0]);
process.exit(ok?0:1);
