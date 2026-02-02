#!/usr/bin/env node

/**
 * Helper script to add provider metadata entries
 * Usage:
 *   node scripts/add_provider_metadata.js <provider_id>
 *   node scripts/add_provider_metadata.js groq deepinfra together
 *
 * This script:
 * 1. Fetches models.dev API data
 * 2. Extracts metadata for the specified provider(s)
 * 3. Generates JSON entries for provider_metadata.json
 */

const https = require('https');
const fs = require('fs');
const path = require('path');

const MODELS_DEV_URL = 'https://models.dev/api.json';
const METADATA_FILE = path.join(__dirname, '../crates/goose/src/providers/canonical/data/provider_metadata.json');

// Map npm packages to our format enum
function detectFormat(npmPackage) {
  if (!npmPackage) return 'openai';
  if (npmPackage.includes('anthropic')) return 'anthropic';
  if (npmPackage.includes('ollama')) return 'ollama';
  return 'openai'; // Default
}

// Fetch models.dev data
function fetchModelsdev() {
  return new Promise((resolve, reject) => {
    https.get(MODELS_DEV_URL, (res) => {
      let data = '';
      res.on('data', chunk => data += chunk);
      res.on('end', () => {
        try {
          resolve(JSON.parse(data));
        } catch (e) {
          reject(new Error(`Failed to parse models.dev response: ${e.message}`));
        }
      });
    }).on('error', reject);
  });
}

// Extract provider metadata
function extractProviderMetadata(providerData, providerId) {
  const format = detectFormat(providerData.npm);

  return {
    id: providerId,
    display_name: providerData.name || providerId,
    format: format,
    api_url: providerData.api || '',
    doc_url: providerData.doc || '',
    env_var: (providerData.env && providerData.env[0]) || `${providerId.toUpperCase().replace(/-/g, '_')}_API_KEY`,
    supports_streaming: true,
    requires_auth: true
  };
}

// Main
async function main() {
  const args = process.argv.slice(2);

  if (args.length === 0) {
    console.error('Usage: node add_provider_metadata.js <provider_id> [provider_id...]');
    console.error('Example: node add_provider_metadata.js groq deepinfra');
    process.exit(1);
  }

  console.log('Fetching models.dev data...');
  const modelsdev = await fetchModelsdev();

  // Load existing metadata
  let existingMetadata = [];
  if (fs.existsSync(METADATA_FILE)) {
    existingMetadata = JSON.parse(fs.readFileSync(METADATA_FILE, 'utf8'));
  }

  const existingIds = new Set(existingMetadata.map(p => p.id));

  // Extract metadata for each provider
  const newEntries = [];
  for (const providerId of args) {
    if (existingIds.has(providerId)) {
      console.warn(`⚠ Provider ${providerId} already exists in metadata, skipping`);
      continue;
    }

    if (!modelsdev[providerId]) {
      console.error(`✗ Provider ${providerId} not found in models.dev`);
      continue;
    }

    const metadata = extractProviderMetadata(modelsdev[providerId], providerId);
    newEntries.push(metadata);
    console.log(`✓ Extracted metadata for ${providerId}`);
  }

  if (newEntries.length === 0) {
    console.log('No new providers to add.');
    return;
  }

  // Display new entries
  console.log('\nNew metadata entries:');
  console.log(JSON.stringify(newEntries, null, 2));

  console.log('\n⚠ NOTE: This script only generates the JSON.');
  console.log('To add these providers:');
  console.log('1. Copy the JSON above');
  console.log('2. Add it to: crates/goose/src/providers/canonical/data/provider_metadata.json');
  console.log('3. Ensure canonical models exist for these providers (run build_canonical_models if needed)');
  console.log('4. Rebuild the project: cargo build');
}

main().catch(err => {
  console.error('Error:', err.message);
  process.exit(1);
});
