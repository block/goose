/**
 * import-map.js - Import map replacement transform
 *
 * Deterministically replaces the import map in an HTML file
 * with an updated version from plugin cache.
 */

import { readFileSync } from 'fs';
import { join } from 'path';
import { PLUGIN_ROOT } from '../paths.js';
import { loadCachedImportMap } from '../compare.js';

/**
 * Regex to match import map script tag
 */
const IMPORT_MAP_REGEX = /<script\s+type=["']importmap["']>([\s\S]*?)<\/script>/i;

/**
 * Replace import map in HTML content
 * @param {string} html - The HTML content
 * @param {object} newImports - The new imports object
 * @returns {object} - Result with success, html, and diff
 */
function replaceImportMap(html, newImports) {
  const match = html.match(IMPORT_MAP_REGEX);

  if (!match) {
    return {
      success: false,
      error: 'No import map found in HTML'
    };
  }

  // Parse existing import map to preserve structure
  let existingMap;
  try {
    existingMap = JSON.parse(match[1]);
  } catch (e) {
    return {
      success: false,
      error: 'Could not parse existing import map'
    };
  }

  // Create new import map preserving scopes if any
  const newImportMap = {
    imports: newImports
  };

  // Preserve scopes if they exist
  if (existingMap.scopes) {
    newImportMap.scopes = existingMap.scopes;
  }

  // Format with consistent indentation
  const newImportMapJson = JSON.stringify(newImportMap, null, 2);

  // Construct the full replacement
  const replacement = `<script type="importmap">\n${newImportMapJson}\n    </script>`;

  // Perform replacement
  const newHtml = html.replace(IMPORT_MAP_REGEX, replacement);

  // Verify replacement occurred
  if (newHtml === html) {
    return {
      success: false,
      error: 'Import map replacement had no effect'
    };
  }

  return {
    success: true,
    html: newHtml,
    diff: {
      before: match[1].trim(),
      after: newImportMapJson
    }
  };
}

/**
 * Apply import map update from plugin cache
 * @param {string} html - The HTML content
 * @param {object} analysis - Analysis result from analyze()
 * @returns {object} - Transform result
 */
function applyImportMapUpdate(html, analysis) {
  const cachedImportMap = loadCachedImportMap(PLUGIN_ROOT);

  if (!cachedImportMap) {
    return {
      success: false,
      error: 'Could not load plugin cache. Run "vibes sync" first.'
    };
  }

  // For vibes-basic template, use the standard import map
  // For sell template, we need to preserve Clerk and adjust React versions
  let newImports;

  if (analysis.templateType === 'vibes-basic') {
    newImports = cachedImportMap.imports;
  } else {
    // Sell template: merge cached imports with Clerk
    newImports = { ...cachedImportMap.imports };

    // Preserve Clerk import if it exists
    if (analysis.importMap?.imports?.['@clerk/clerk-react']) {
      newImports['@clerk/clerk-react'] = analysis.importMap.imports['@clerk/clerk-react'];
    }

    // For sell, we might want to keep pinned React versions
    // This is a policy decision - for now, update React too
  }

  return replaceImportMap(html, newImports);
}

/**
 * Migrate ?deps= to ?external= parameters
 * @param {string} html - The HTML content
 * @returns {object} - Transform result
 */
function migrateDepsToExternal(html) {
  const match = html.match(IMPORT_MAP_REGEX);

  if (!match) {
    return {
      success: false,
      error: 'No import map found in HTML'
    };
  }

  let importMapContent = match[1];
  const originalContent = importMapContent;

  // Replace ?deps=react@X.Y.Z with ?external=react,react-dom
  // Pattern: ?deps=react@18.3.1 or similar
  importMapContent = importMapContent.replace(
    /\?deps=react@[\d.]+/g,
    '?external=react,react-dom'
  );

  if (importMapContent === originalContent) {
    return {
      success: false,
      error: 'No ?deps= patterns found to migrate'
    };
  }

  const replacement = `<script type="importmap">${importMapContent}</script>`;
  const newHtml = html.replace(IMPORT_MAP_REGEX, replacement);

  return {
    success: true,
    html: newHtml,
    diff: {
      before: originalContent.trim(),
      after: importMapContent.trim()
    }
  };
}

/**
 * Add ?external= parameter to imports missing it
 * @param {string} html - The HTML content
 * @returns {object} - Transform result
 */
function addExternalParams(html) {
  const match = html.match(IMPORT_MAP_REGEX);

  if (!match) {
    return {
      success: false,
      error: 'No import map found in HTML'
    };
  }

  let importMapContent = match[1];
  const originalContent = importMapContent;

  // Find use-vibes, use-fireproof URLs without ?external=
  // Add ?external=react,react-dom to them
  const packagesToFix = ['use-vibes', 'use-fireproof'];

  for (const pkg of packagesToFix) {
    // Match package URL without query params or with non-external params
    const regex = new RegExp(
      `("${pkg}"\\s*:\\s*"https://esm\\.sh/${pkg}@[^"?]+)("|\\?(?!external)[^"]*)`,
      'g'
    );

    importMapContent = importMapContent.replace(regex, (match, url, rest) => {
      if (rest === '"') {
        // No query params, add ?external=
        return `${url}?external=react,react-dom"`;
      } else if (rest.startsWith('?') && !rest.includes('external=')) {
        // Has query params but not external, add external
        return `${url}${rest.replace('?', '?external=react,react-dom&').replace('"', '"')}`;
      }
      return match;
    });
  }

  if (importMapContent === originalContent) {
    return {
      success: false,
      error: 'All packages already have ?external= parameter'
    };
  }

  const replacement = `<script type="importmap">${importMapContent}</script>`;
  const newHtml = html.replace(IMPORT_MAP_REGEX, replacement);

  return {
    success: true,
    html: newHtml,
    diff: {
      before: originalContent.trim(),
      after: importMapContent.trim()
    }
  };
}

export {
  replaceImportMap,
  applyImportMapUpdate,
  migrateDepsToExternal,
  addExternalParams,
  IMPORT_MAP_REGEX
};
