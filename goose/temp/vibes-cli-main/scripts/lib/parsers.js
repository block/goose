/**
 * Parsing utilities for vibes.diy upstream files
 *
 * These are extracted into a separate module for testability.
 */

// Required keys that must be present in a valid import map
export const REQUIRED_IMPORT_MAP_KEYS = ['react', 'react-dom'];

/**
 * Extract version from a URL string
 * e.g., "https://esm.sh/use-vibes@0.24.3-dev?external=react" -> "0.24.3-dev"
 * @param {string} url - The URL to parse
 * @returns {string|null} - Version string or null
 */
export function extractVersion(url) {
  if (!url) return null;
  const match = url.match(/@([\d.]+[^?/]*)/);
  return match ? match[1] : null;
}

/**
 * Parse the import-map.ts file from vibes.diy and extract the import map.
 *
 * Handles multiple syntax patterns:
 * - Quoted keys: "react-dom": "https://esm.sh/react-dom@19.2.1"
 * - Unquoted keys: react: "https://esm.sh/react@19.2.1"
 * - Template literals with VIBES_VERSION: "use-vibes": `https://esm.sh/use-vibes@${VIBES_VERSION}`
 *
 * @param {string} content - Raw TypeScript content from import-map.ts
 * @param {object} options - Options
 * @param {boolean} options.silent - Suppress validation warnings (for testing)
 * @returns {Object.<string, string>} - Object mapping package names to CDN URLs
 * @example
 * const imports = parseImportMapTs(tsContent);
 * // { "react": "https://esm.sh/react@19.2.1", "use-vibes": "https://esm.sh/use-vibes@0.19" }
 */
export function parseImportMapTs(content, options = {}) {
  const imports = {};

  // Extract VIBES_VERSION for template literal resolution
  const versionMatch = content.match(/const VIBES_VERSION\s*=\s*["']([^"']+)["']/);
  const vibesVersion = versionMatch ? versionMatch[1] : "0.19";

  // Unified pattern: matches both quoted and unquoted keys with static or template values
  // Pattern breakdown:
  //   ["']?(\w[\w-]*)["']?  - key (optionally quoted, allows hyphens)
  //   \s*:\s*               - colon with optional whitespace
  //   (["'`])               - opening quote/backtick (captured for backreference)
  //   (https://[^"'`]+?)    - URL (non-greedy to stop at quote)
  //   (?:\$\{VIBES_VERSION\})? - optional template variable
  //   \3                    - closing quote (backreference)
  const entryPattern = /["']?([\w][\w-]*)["']?\s*:\s*(["'`])(https:\/\/[^"'`]+?)(?:\$\{VIBES_VERSION\})?\2/g;

  for (const match of content.matchAll(entryPattern)) {
    const [fullMatch, key, , urlPart] = match;
    // Check if this was a template literal with VIBES_VERSION
    const hasVersion = fullMatch.includes('${VIBES_VERSION}');
    imports[key] = hasVersion ? urlPart + vibesVersion : urlPart;
  }

  // Validate we got expected entries
  if (!options.silent) {
    const missingKeys = REQUIRED_IMPORT_MAP_KEYS.filter(k => !imports[k]);
    if (missingKeys.length > 0) {
      console.warn(`Warning: Import map missing expected keys: ${missingKeys.join(', ')}`);
    }
  }

  return imports;
}

/**
 * Parse the style-prompts.ts file and extract the default style prompt.
 *
 * Looks for DEFAULT_STYLE_NAME and then extracts the corresponding prompt
 * from the stylePrompts array. Falls back to searching for a brutalist-style
 * prompt if the default cannot be found.
 *
 * @param {string} content - Raw TypeScript content from style-prompts.ts
 * @returns {string} The extracted style prompt text, or empty string if not found
 */
export function parseStylePromptsTs(content) {
  // Find the DEFAULT_STYLE_NAME
  const defaultNameMatch = content.match(/DEFAULT_STYLE_NAME\s*=\s*["']([^"']+)["']/);
  const defaultName = defaultNameMatch ? defaultNameMatch[1] : "brutalist web";

  // Find the stylePrompts array and extract the prompt for the default style
  const styleRegex = new RegExp(
    `\\{\\s*name:\\s*["']${defaultName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}["']\\s*,\\s*prompt:\\s*(['"\`])((?:(?!\\1)[^\\\\]|\\\\.)*)\\1`,
    's'
  );

  const match = content.match(styleRegex);
  if (match) {
    return match[2]
      .replace(/\\n/g, '\n')
      .replace(/\\"/g, '"')
      .replace(/\\'/g, "'")
      .replace(/\\\\/g, '\\');
  }

  // Fallback
  const fallbackMatch = content.match(/prompt:\s*['"`](Create a UI theme in a neo-brutalist style[^'"`]*)['"]/s);
  if (fallbackMatch) {
    return fallbackMatch[1];
  }

  return "";
}
