#!/usr/bin/env node
/**
 * Merge Templates
 *
 * Combines base template + built components + delta templates
 * to generate final templates for each skill.
 *
 * Usage:
 *   node scripts/merge-templates.js
 *   node scripts/merge-templates.js --force  # Rebuild even if templates exist
 */

import { readFileSync, writeFileSync, existsSync, mkdirSync } from "fs";
import { join, dirname } from "path";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Plugin root is one level up from scripts/
const PLUGIN_ROOT = join(__dirname, "..");
const CACHE_DIR = join(PLUGIN_ROOT, "cache");

// Paths
const BASE_TEMPLATE = join(PLUGIN_ROOT, "skills/_base/template.html");
const COMPONENTS_FILE = join(CACHE_DIR, "vibes-menu.js");

// Skills to generate templates for
// NOTE: riff/templates is a symlink to vibes/templates, so no separate generation needed
const SKILLS = [
  {
    name: "vibes",
    delta: join(PLUGIN_ROOT, "skills/vibes/template.delta.html"),
    output: join(PLUGIN_ROOT, "skills/vibes/templates/index.html"),
    title: "Made on Vibes DIY"
  },
  {
    name: "sell",
    delta: join(PLUGIN_ROOT, "skills/sell/template.delta.html"),
    output: join(PLUGIN_ROOT, "skills/sell/templates/unified.html"),
    title: "__APP_TITLE__"  // Sell uses dynamic title
  }
];

/**
 * Read a file, returning null if it doesn't exist
 */
function readFileSafe(path) {
  try {
    return readFileSync(path, "utf-8");
  } catch (e) {
    return null;
  }
}

/**
 * Merge templates for a single skill
 */
function mergeTemplate(skill, baseTemplate, components) {
  const delta = readFileSafe(skill.delta);
  if (!delta) {
    console.warn(`  Warning: Delta template not found: ${skill.delta}`);
    return null;
  }

  // Start with base template
  let merged = baseTemplate;

  // Replace title placeholder
  merged = merged.replace("__TITLE__", skill.title);

  // Inject components at placeholder
  merged = merged.replace(
    "// === COMPONENTS_PLACEHOLDER ===",
    components
  );

  // Inject delta at placeholder
  merged = merged.replace(
    "<!-- === DELTA_PLACEHOLDER === -->",
    delta
  );

  return merged;
}

/**
 * Main function
 */
function main() {
  const force = process.argv.includes("--force");

  console.log("Merge Templates");
  console.log("");

  // Read base template
  const baseTemplate = readFileSafe(BASE_TEMPLATE);
  if (!baseTemplate) {
    console.error(`Error: Base template not found: ${BASE_TEMPLATE}`);
    process.exit(1);
  }
  console.log(`  Base template: ${BASE_TEMPLATE}`);

  // Read built components
  const components = readFileSafe(COMPONENTS_FILE);
  if (!components) {
    console.error(`Error: Components not found: ${COMPONENTS_FILE}`);
    console.error("  Run: node scripts/build-components.js --force");
    process.exit(1);
  }
  console.log(`  Components: ${COMPONENTS_FILE} (${components.length} bytes)`);
  console.log("");

  // Process each skill
  const results = { success: [], failed: [], skipped: [] };

  for (const skill of SKILLS) {
    // Check if output exists and skip if not forcing
    if (!force && existsSync(skill.output)) {
      results.skipped.push(skill.name);
      continue;
    }

    // Ensure output directory exists
    const outputDir = dirname(skill.output);
    mkdirSync(outputDir, { recursive: true });

    // Merge template
    const merged = mergeTemplate(skill, baseTemplate, components);
    if (!merged) {
      results.failed.push(skill.name);
      continue;
    }

    // Write output
    try {
      writeFileSync(skill.output, merged, "utf-8");
      console.log(`  Generated: ${skill.output}`);
      results.success.push(skill.name);
    } catch (e) {
      console.error(`  Error writing ${skill.output}: ${e.message}`);
      results.failed.push(skill.name);
    }
  }

  // Summary
  console.log("");
  console.log("Summary:");
  if (results.success.length > 0) {
    console.log(`  Generated: ${results.success.join(", ")}`);
  }
  if (results.skipped.length > 0) {
    console.log(`  Skipped (use --force): ${results.skipped.join(", ")}`);
  }
  if (results.failed.length > 0) {
    console.log(`  Failed: ${results.failed.join(", ")}`);
    process.exit(1);
  }

  console.log("\nDone!");
}

main();
