#!/usr/bin/env node
/**
 * Build Vibes Components
 *
 * Transpiles local TypeScript components from components/ directory
 * and outputs bundled JavaScript to cache/vibes-menu.js
 *
 * Usage:
 *   node scripts/build-components.js
 *   node scripts/build-components.js --force  # Rebuild even if cache exists
 */

import { readFileSync, writeFileSync, existsSync, mkdirSync } from "fs";
import { join, dirname } from "path";
import { fileURLToPath } from "url";
import * as esbuild from "esbuild";
import { transformComponent } from "./lib/component-transforms.js";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Plugin root is one level up from scripts/
const PLUGIN_ROOT = join(__dirname, "..");
const CACHE_DIR = join(PLUGIN_ROOT, "cache");
const COMPONENTS_DIR = join(PLUGIN_ROOT, "components");
const OUTPUT_FILE = join(CACHE_DIR, "vibes-menu.js");

// Component sources in dependency order
// These are relative to COMPONENTS_DIR
const COMPONENT_SOURCES = {
  // Hooks first (no dependencies)
  "useMobile": "mocks/use-vibes-base.ts",
  "useIsMobile": "hooks/useIsMobile.ts",

  // All icons (no dependencies)
  "BackIcon": "icons/BackIcon.tsx",
  "InviteIcon": "icons/InviteIcon.tsx",
  "LoginIcon": "icons/LoginIcon.tsx",
  "RemixIcon": "icons/RemixIcon.tsx",
  "SettingsIcon": "icons/SettingsIcon.tsx",
  "GoogleIcon": "icons/GoogleIcon.tsx",
  "GitHubIcon": "icons/GitHubIcon.tsx",
  "MoonIcon": "icons/MoonIcon.tsx",
  "SunIcon": "icons/SunIcon.tsx",

  // New components (in dependency order)
  "BrutalistCard.styles": "BrutalistCard/BrutalistCard.styles.ts",
  "BrutalistCard": "BrutalistCard/BrutalistCard.tsx",
  "LabelContainer.styles": "LabelContainer/LabelContainer.styles.ts",
  "LabelContainer": "LabelContainer/LabelContainer.tsx",

  // Core components
  "VibesSwitch.styles": "VibesSwitch/VibesSwitch.styles.ts",
  "VibesSwitch": "VibesSwitch/VibesSwitch.tsx",
  "HiddenMenuWrapper.styles": "HiddenMenuWrapper/HiddenMenuWrapper.styles.ts",
  "HiddenMenuWrapper": "HiddenMenuWrapper/HiddenMenuWrapper.tsx",
  "VibesButton.styles": "VibesButton/VibesButton.styles.ts",
  "VibesButton": "VibesButton/VibesButton.tsx",
  "VibesPanel.styles": "VibesPanel/VibesPanel.styles.ts",
  "VibesPanel": "VibesPanel/VibesPanel.tsx",
};

/**
 * Transpile a single component file
 * @param {string} name - Component name for logging
 * @param {string} source - Source code content
 * @param {boolean} isTS - Whether this is a .ts file (vs .tsx)
 * @returns {Promise<{name: string, code: string|null, success: boolean}>}
 */
async function transpileComponent(name, source, isTS) {
  const loader = isTS ? "ts" : "tsx";

  try {
    const result = await esbuild.transform(source, {
      loader,
      jsx: "transform",
      jsxFactory: "React.createElement",
      jsxFragment: "React.Fragment",
      target: "es2020",
    });

    // Apply all component transformations (imports, hooks, namespacing)
    const code = transformComponent(result.code, name);

    return { name, code, success: true };
  } catch (err) {
    console.warn(`  Warning: Failed to transpile ${name}: ${err.message}`);
    return { name, code: null, success: false };
  }
}

/**
 * Build all components
 */
async function buildComponents(force) {
  // Check if rebuild is needed
  if (!force && existsSync(OUTPUT_FILE)) {
    console.log("Cache exists. Use --force to rebuild.");
    return { success: true, cached: true };
  }

  // Ensure cache directory exists
  mkdirSync(CACHE_DIR, { recursive: true });

  console.log("Building components from local source...");
  console.log(`  Source: ${COMPONENTS_DIR}`);
  console.log(`  Output: ${OUTPUT_FILE}`);
  console.log("");

  const results = [];

  // Read and transpile each component
  for (const [name, relativePath] of Object.entries(COMPONENT_SOURCES)) {
    const fullPath = join(COMPONENTS_DIR, relativePath);

    if (!existsSync(fullPath)) {
      console.warn(`  Warning: Component file not found: ${relativePath}`);
      results.push({ name, code: null, success: false });
      continue;
    }

    try {
      const source = readFileSync(fullPath, "utf-8");
      const isTS = relativePath.endsWith(".ts") && !relativePath.endsWith(".tsx");
      const result = await transpileComponent(name, source, isTS);
      results.push(result);
      if (result.success) {
        console.log(`  Transpiled ${name}`);
      }
    } catch (err) {
      console.warn(`  Warning: Failed to read ${name}: ${err.message}`);
      results.push({ name, code: null, success: false });
    }
  }

  // Count successes and failures
  const successful = results.filter(r => r.success);
  const failed = results.filter(r => !r.success);

  if (failed.length > 0) {
    console.warn(`\n  Failed to build ${failed.length}/${results.length} components: ${failed.map(f => f.name).join(", ")}`);
  }

  if (successful.length === 0) {
    console.error("Error: No components built successfully");
    return { success: false, cached: false, error: "No components built" };
  }

  // Combine all transpiled code
  let combinedOutput = `// Auto-generated vibes menu components
// Run: node scripts/build-components.js --force to regenerate
// Source: ${COMPONENTS_DIR}
// Generated: ${new Date().toISOString()}
// Components: ${successful.length}/${results.length}

`;

  // Add each component section
  for (const { name, code, success } of results) {
    if (success && code) {
      combinedOutput += `// === ${name} ===\n${code}\n\n`;
    }
  }

  // Add window exports for components that need to be accessed globally
  combinedOutput += `// === Window Exports (for standalone apps) ===
// Expose key components to window for use in inline scripts
if (typeof window !== 'undefined') {
  // Hooks
  window.useMobile = useMobile;
  window.useIsMobile = useIsMobile;

  // Core components
  window.HiddenMenuWrapper = HiddenMenuWrapper;
  window.VibesSwitch = VibesSwitch;
  window.VibesButton = VibesButton;
  window.VibesPanel = VibesPanel;
  window.BrutalistCard = BrutalistCard;
  window.LabelContainer = LabelContainer;

  // Button variant constants
  window.BLUE = BLUE;
  window.RED = RED;
  window.YELLOW = YELLOW;
  window.GRAY = GRAY;

  // Icons
  window.BackIcon = BackIcon;
  window.InviteIcon = InviteIcon;
  window.LoginIcon = LoginIcon;
  window.RemixIcon = RemixIcon;
  window.SettingsIcon = SettingsIcon;
  window.GoogleIcon = GoogleIcon;
  window.GitHubIcon = GitHubIcon;
  window.MoonIcon = MoonIcon;
  window.SunIcon = SunIcon;
}
`;

  // Write output
  writeFileSync(OUTPUT_FILE, combinedOutput, "utf-8");

  console.log(`\nBuilt ${successful.length} components (${combinedOutput.length} bytes)`);
  console.log(`Output: ${OUTPUT_FILE}`);

  return { success: true, cached: false, componentCount: successful.length };
}

// Main execution
async function main() {
  const force = process.argv.includes("--force");

  try {
    const result = await buildComponents(force);
    if (!result.success) {
      process.exit(1);
    }
    if (result.cached) {
      console.log("Components already built. Use --force to rebuild.");
    } else {
      console.log("\nDone!");
    }
  } catch (error) {
    console.error("Build failed:", error.message);
    process.exit(1);
  }
}

main();
