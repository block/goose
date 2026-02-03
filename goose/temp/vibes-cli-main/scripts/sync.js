#!/usr/bin/env node
/**
 * Vibes DIY Sync Script
 *
 * Fetches documentation, import maps, style prompts, and CSS variables from
 * upstream sources and caches locally for fast skill invocation.
 *
 * NOTE: Menu component building is now handled by build-components.js
 * which transpiles local components from the components/ directory.
 * Run: node scripts/build-components.js
 *
 * Usage:
 *   node scripts/sync.js          # Fetch only if cache is empty
 *   node scripts/sync.js --force  # Force refresh all cached files
 */

import { existsSync, mkdirSync, writeFileSync, readFileSync } from "fs";
import { join } from "path";
import {
  PLUGIN_ROOT,
  CACHE_DIR,
  CONFIG_FILES,
  TEMPLATES,
  CACHE_FILES,
  SKILL_FILES,
  relativeToPlugin
} from "./lib/paths.js";
import { parseImportMapTs, parseStylePromptsTs } from "./lib/parsers.js";

// Default upstream sources (can be overridden via config file or env vars)
// NOTE: Menu components are now built locally via build-components.js
const DEFAULT_SOURCES = {
  fireproof: "https://use-fireproof.com/llms-full.txt",
  stylePrompt: "https://raw.githubusercontent.com/VibesDIY/vibes.diy/main/prompts/pkg/style-prompts.ts",
  importMap: "https://raw.githubusercontent.com/VibesDIY/vibes.diy/main/vibes.diy/pkg/app/config/import-map.ts",
  cssVariables: "https://raw.githubusercontent.com/VibesDIY/vibes.diy/main/vibes.diy/pkg/app/styles/colors.css"
};

/**
 * Load source configuration from file or environment
 * Priority: env vars > config file > defaults
 */
function loadSourceConfig() {
  let fileConfig = {};

  // Try loading config file
  if (existsSync(CONFIG_FILES.sources)) {
    try {
      fileConfig = JSON.parse(readFileSync(CONFIG_FILES.sources, 'utf-8'));
      console.log('Loaded source config from', relativeToPlugin(CONFIG_FILES.sources));
    } catch (e) {
      console.warn(`Warning: Could not parse config file (${e.message}), using defaults`);
    }
  }

  // Merge with env vars taking priority
  // NOTE: vibesComponentsBase and useVibesBase removed - components built locally via build-components.js
  return {
    fireproof: process.env.VIBES_FIREPROOF_URL || fileConfig.fireproof || DEFAULT_SOURCES.fireproof,
    stylePrompt: process.env.VIBES_STYLE_PROMPT_URL || fileConfig.stylePrompt || DEFAULT_SOURCES.stylePrompt,
    importMap: process.env.VIBES_IMPORT_MAP_URL || fileConfig.importMap || DEFAULT_SOURCES.importMap,
    cssVariables: process.env.VIBES_CSS_VARIABLES_URL || fileConfig.cssVariables || DEFAULT_SOURCES.cssVariables
  };
}

// Load configuration
const SOURCE_CONFIG = loadSourceConfig();

// Documentation sources
// NOTE: fireproof.txt is now maintained independently in this plugin (not synced from upstream)
// This allows us to use unified port 8080 for local development per selem/docker-for-all branch
const DOC_SOURCES = {
  // fireproof: SOURCE_CONFIG.fireproof,  // Removed - now maintained in skills/vibes/cache/fireproof.txt
};

// Style prompt source
const STYLE_PROMPT_URL = SOURCE_CONFIG.stylePrompt;

// Import map source
const IMPORT_MAP_URL = SOURCE_CONFIG.importMap;

// CSS variables source (colors.css contains all button/card/theme variables)
const CSS_VARIABLES_URL = SOURCE_CONFIG.cssVariables;

// NOTE: Menu component sources removed - now built locally via build-components.js

// Default timeout for fetch requests (60 seconds)
const FETCH_TIMEOUT_MS = 60000;

/**
 * Fetch with timeout protection
 * @param {string} url - URL to fetch
 * @param {number} timeoutMs - Timeout in milliseconds (default: FETCH_TIMEOUT_MS)
 * @returns {Promise<Response>} - Fetch response
 */
async function fetchWithTimeout(url, timeoutMs = FETCH_TIMEOUT_MS) {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), timeoutMs);

  try {
    const response = await fetch(url, { signal: controller.signal });
    clearTimeout(timeoutId);
    return response;
  } catch (error) {
    clearTimeout(timeoutId);
    if (error.name === 'AbortError') {
      throw new Error(`Request timed out after ${timeoutMs / 1000}s`);
    }
    throw error;
  }
}

/**
 * Generic fetch-and-cache function
 * @param {object} config - Fetch configuration
 * @param {string} config.name - Cache name for logging
 * @param {string} config.url - URL to fetch from
 * @param {string} config.cachePath - Path to cache file
 * @param {boolean} force - Force refresh even if cached
 * @param {object} [options] - Optional processing options
 * @param {function} [options.transform] - Transform content before caching
 * @param {function} [options.onNotFound] - Fallback when URL returns 404
 * @param {function} [options.formatLog] - Custom log message for cached size
 * @returns {Promise<{name: string, success: boolean, cached: boolean, error?: string}>}
 */
async function fetchAndCache({ name, url, cachePath }, force, options = {}) {
  if (!force && existsSync(cachePath)) {
    return { name, success: true, cached: true };
  }

  try {
    console.log(`Fetching ${name} from ${url}...`);
    const response = await fetchWithTimeout(url);

    if (!response.ok) {
      if (options.onNotFound && response.status === 404) {
        const fallback = options.onNotFound();
        writeFileSync(cachePath, fallback, "utf-8");
        console.log(`  Generated ${name} (${fallback.length} bytes)`);
        return { name, success: true, cached: false };
      }
      return {
        name,
        success: false,
        cached: false,
        error: `HTTP ${response.status}: ${response.statusText}`
      };
    }

    let content = await response.text();

    if (options.transform) {
      const transformed = options.transform(content);
      if (transformed === null) {
        return {
          name,
          success: false,
          cached: false,
          error: `Failed to parse ${name} from source`
        };
      }
      content = transformed;
    }

    writeFileSync(cachePath, content, "utf-8");
    const logMsg = options.formatLog ? options.formatLog(content) : `${content.length} bytes`;
    console.log(`  Cached ${name} (${logMsg})`);

    return { name, success: true, cached: false };
  } catch (error) {
    return {
      name,
      success: false,
      cached: false,
      error: error instanceof Error ? error.message : String(error)
    };
  }
}

// Convenience wrappers using fetchAndCache

async function fetchDoc(name, url, force) {
  return fetchAndCache(
    { name, url, cachePath: join(CACHE_DIR, `${name}.txt`) },
    force
  );
}

// parseImportMapTs and parseStylePromptsTs are imported from ./lib/parsers.js

async function fetchStylePrompt(force) {
  return fetchAndCache(
    { name: "style-prompt", url: STYLE_PROMPT_URL, cachePath: join(CACHE_DIR, "style-prompt.txt") },
    force,
    { transform: parseStylePromptsTs }
  );
}

async function fetchImportMap(force) {
  return fetchAndCache(
    { name: "import-map", url: IMPORT_MAP_URL, cachePath: join(CACHE_DIR, "import-map.json") },
    force,
    {
      transform: (content) => {
        const imports = parseImportMapTs(content);
        return JSON.stringify({
          lastUpdated: new Date().toISOString(),
          source: IMPORT_MAP_URL,
          imports
        }, null, 2);
      },
      formatLog: (content) => {
        const parsed = JSON.parse(content);
        return `${Object.keys(parsed.imports).length} entries`;
      }
    }
  );
}

/**
 * Fetch CSS variables from vibes.diy
 */
async function fetchCssVariables(force) {
  const minimalCssFallback = () => `:root {
  /* Vibes color variables */
  --vibes-black: #0f172a;
  --vibes-white: #ffffff;
  --vibes-near-black: #1e293b;
  --vibes-gray-ultralight: #f8fafc;
  --vibes-gray-lightest: #f1f5f9;
  --vibes-gray-light: #e2e8f0;
  --vibes-gray: #94a3b8;
  --vibes-gray-dark: #64748b;

  /* Button variants */
  --vibes-variant-blue: #3b82f6;
  --vibes-variant-red: #ef4444;
  --vibes-variant-yellow: #eab308;
  --vibes-variant-gray: #6b7280;
  --vibes-variant-green: #22c55e;
}

/* Menu animations */
@keyframes vibes-bounce {
  0%, 100% { transform: translateY(0); }
  50% { transform: translateY(-4px); }
}
`;

  return fetchAndCache(
    { name: "vibes-variables", url: CSS_VARIABLES_URL, cachePath: join(CACHE_DIR, "vibes-variables.css") },
    force,
    { onNotFound: minimalCssFallback }
  );
}

// NOTE: fetchMenuComponents removed - use build-components.js instead

/**
 * Generate the import map JSON string for templates
 * Uses unpinned React (esm.sh resolves latest compatible) and ?external= for singleton
 *
 * NOTE: Version 0.24.3-dev includes toCloud() for Fireproof Cloud sync.
 * See CLAUDE.md for details.
 */
const STABLE_VIBES_VERSION = "0.18.9";

// @fireproof/clerk package for authenticated Clerk sync
const FIREPROOF_CLERK_VERSION = "0.0.3";
const FIREPROOF_CLERK_PACKAGE = "@necrodome/fireproof-clerk";

function generateImportMapJson(imports) {
  // Use unpinned React URLs - esm.sh will resolve compatible versions
  const templateImports = {
    "react": "https://esm.sh/react",
    "react-dom": "https://esm.sh/react-dom",
    "react-dom/client": "https://esm.sh/react-dom/client",
    "react/jsx-runtime": "https://esm.sh/react/jsx-runtime",
  };

  // Use stable version with ?external=react,react-dom for single React instance
  // Override upstream version if it's a dev version (has known bugs)
  templateImports["use-fireproof"] = `https://esm.sh/use-vibes@${STABLE_VIBES_VERSION}?external=react,react-dom`;
  templateImports["use-vibes"] = `https://esm.sh/use-vibes@${STABLE_VIBES_VERSION}?external=react,react-dom`;

  // @fireproof/clerk for authenticated Clerk sync
  templateImports["@fireproof/clerk"] = `https://esm.sh/${FIREPROOF_CLERK_PACKAGE}@${FIREPROOF_CLERK_VERSION}?external=react,react-dom`;

  return JSON.stringify({ imports: templateImports }, null, 6).replace(/^/gm, '  ').trim();
}

// NOTE: Component extraction and template update functions removed.
// Template components are now managed by:
//   - build-components.js (builds from local components/)
//   - merge-templates.js (combines base + delta templates)

/**
 * Update import maps in skill/agent files
 */
function updateSkillImportMaps(imports) {
  const updated = [];
  const failed = [];

  // Use centralized skill file paths
  const filesToUpdate = [SKILL_FILES.vibesSkill];

  const importMapRegex = /<script type="importmap">\s*\{[\s\S]*?"imports":\s*\{[\s\S]*?\}\s*\}\s*<\/script>/g;

  const newImportMap = `<script type="importmap">
  ${generateImportMapJson(imports)}
  </script>`;

  for (const filePath of filesToUpdate) {
    if (!existsSync(filePath)) {
      continue;
    }

    try {
      const content = readFileSync(filePath, "utf-8");
      const newContent = content.replace(importMapRegex, newImportMap);

      if (newContent !== content) {
        writeFileSync(filePath, newContent, "utf-8");
        updated.push(relativeToPlugin(filePath));
      }
    } catch (error) {
      failed.push(relativeToPlugin(filePath));
    }
  }

  return { updated, failed };
}

async function main() {
  const force = process.argv.includes("--force");
  const verbose = process.argv.includes("--verbose") || process.argv.includes("-v");

  mkdirSync(CACHE_DIR, { recursive: true });

  console.log("Vibes DIY Sync");
  console.log(`Cache directory: ${CACHE_DIR}`);
  console.log(`Force refresh: ${force}`);
  console.log("");

  const results = [];

  // Fetch documentation
  for (const [name, url] of Object.entries(DOC_SOURCES)) {
    const result = await fetchDoc(name, url, force);
    results.push(result);
  }

  // Fetch style prompt
  const stylePromptResult = await fetchStylePrompt(force);
  results.push(stylePromptResult);

  // Fetch import map
  const importMapResult = await fetchImportMap(force);
  results.push(importMapResult);

  // Fetch CSS variables
  const cssResult = await fetchCssVariables(force);
  results.push(cssResult);

  // NOTE: Menu components are now built locally via build-components.js
  // Run: node scripts/build-components.js && node scripts/merge-templates.js

  // Update skill files with new import map
  if (importMapResult.success && !importMapResult.cached) {
    const cache = JSON.parse(readFileSync(CACHE_FILES.importMap, "utf-8"));
    const { updated, failed } = updateSkillImportMaps(cache.imports);

    if (updated.length > 0) {
      console.log("\nUpdated import maps in:");
      for (const file of updated) {
        console.log(`  - ${file}`);
      }
    }
    if (failed.length > 0) {
      console.log("\nFailed to update:");
      for (const file of failed) {
        console.log(`  - ${file}`);
      }
    }
  }

  // Summary
  console.log("\nSummary:");
  const successful = results.filter(r => r.success);
  const failedResults = results.filter(r => !r.success);
  const fromCache = results.filter(r => r.cached);

  console.log(`  Total: ${results.length}`);
  console.log(`  Fetched: ${successful.length - fromCache.length}`);
  console.log(`  From cache: ${fromCache.length}`);

  if (failedResults.length > 0) {
    console.log(`  Failed: ${failedResults.length}`);
    for (const f of failedResults) {
      console.log(`    - ${f.name}: ${f.error}`);
    }
    process.exit(1);
  }

  if (verbose) {
    console.log("\nCached files:");
    for (const result of successful) {
      if (result.name === "import-map") {
        if (existsSync(CACHE_FILES.importMap)) {
          const content = JSON.parse(readFileSync(CACHE_FILES.importMap, "utf-8"));
          console.log(`  import-map: ${Object.keys(content.imports).length} entries, updated ${content.lastUpdated}`);
        }
      } else {
        const cachePath = join(CACHE_DIR, `${result.name}.txt`);
        if (existsSync(cachePath)) {
          const content = readFileSync(cachePath, "utf-8");
          console.log(`  ${result.name}: ${content.length} bytes`);
        }
      }
    }
  }

  // Check cache staleness
  if (existsSync(CACHE_FILES.importMap)) {
    const cache = JSON.parse(readFileSync(CACHE_FILES.importMap, "utf-8"));
    const lastUpdated = new Date(cache.lastUpdated);
    const daysSinceUpdate = Math.floor((Date.now() - lastUpdated.getTime()) / (1000 * 60 * 60 * 24));

    if (daysSinceUpdate > 30) {
      console.warn(`\nWarning: Cache is ${daysSinceUpdate} days old. Consider running with --force to update.`);
    }
  }

  console.log("\nDone!");
}

main().catch(console.error);
