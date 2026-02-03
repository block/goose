/**
 * analyze.js - Parse and detect version information from Vibes apps
 *
 * Phase 1 of the update pipeline:
 * - Detect template type (vibes-basic vs sell)
 * - Extract library versions from import map
 * - Identify era markers and patterns
 * - Locate custom app code boundaries
 */

import { readFileSync, existsSync } from 'fs';
import { resolve, basename } from 'path';
import { extractVersion } from './parsers.js';

/**
 * Era definitions with detection logic
 */
const ERAS = {
  'pre-0.18': {
    name: 'Pre-0.18 (Legacy)',
    markers: ['missing ?external=', 'old component patterns'],
    notes: 'May need manual migration'
  },
  '0.18.x': {
    name: '0.18.x Stable',
    markers: ['?external=react,react-dom', 'stable APIs'],
    notes: 'Current stable release'
  },
  '0.19.x-dev': {
    name: '0.19.x Development',
    markers: ['dev versions', 'potential bugs'],
    notes: 'Should upgrade to stable'
  },
  'sell-v1': {
    name: 'Sell v1',
    markers: ['?deps=react@18.3.1', 'Clerk integration'],
    notes: 'Different update path'
  }
};

/**
 * Extract import map from HTML content
 * @param {string} html - The HTML content
 * @returns {object|null} - Parsed import map or null if not found
 */
function extractImportMap(html) {
  const match = html.match(/<script\s+type=["']importmap["']>([\s\S]*?)<\/script>/i);
  if (!match) return null;

  try {
    return JSON.parse(match[1]);
  } catch (e) {
    console.warn('Warning: Could not parse import map JSON');
    return null;
  }
}


/**
 * Detect template type from HTML content
 * @param {string} html - The HTML content
 * @param {object} importMap - Parsed import map
 * @returns {string} - 'vibes-basic' or 'sell'
 */
function detectTemplateType(html, importMap) {
  // Check for Clerk imports (definitive sell indicator)
  const imports = importMap?.imports || {};
  if (imports['@clerk/clerk-react']) {
    return 'sell';
  }

  // Check for Clerk components in code
  if (html.includes('ClerkProvider') || html.includes('useAuth()') || html.includes('useUser()')) {
    return 'sell';
  }

  // Check for route detection (hostname-based routing)
  if (html.includes('window.location.hostname') && html.includes('subdomain')) {
    return 'sell';
  }

  // Check for CONFIG object with sell-specific fields
  if (html.includes('__CLERK_PUBLISHABLE_KEY__') || html.includes('CLERK_PUBLISHABLE_KEY')) {
    return 'sell';
  }

  return 'vibes-basic';
}

/**
 * Detect era based on import map patterns
 * @param {object} imports - Import map imports object
 * @param {string} templateType - 'vibes-basic' or 'sell'
 * @returns {string} - Era identifier
 */
function detectEra(imports, templateType) {
  if (!imports) return 'unknown';

  const useVibesUrl = imports['use-vibes'] || '';
  const reactUrl = imports['react'] || '';

  // Sell template has its own era
  if (templateType === 'sell') {
    return 'sell-v1';
  }

  // Check for dev versions
  if (useVibesUrl.includes('-dev') || useVibesUrl.includes('-preview')) {
    return '0.19.x-dev';
  }

  // Check for ?external= parameter (0.18.x pattern)
  if (useVibesUrl.includes('?external=')) {
    return '0.18.x';
  }

  // Check for ?deps= parameter (older pattern or sell)
  if (useVibesUrl.includes('?deps=')) {
    return 'sell-v1';
  }

  // If use-vibes exists but no query params, likely pre-0.18
  if (useVibesUrl && !useVibesUrl.includes('?')) {
    return 'pre-0.18';
  }

  return 'unknown';
}

/**
 * Extract library versions from import map
 * @param {object} imports - Import map imports object
 * @returns {object} - Library versions
 */
function extractLibraryVersions(imports) {
  if (!imports) return {};

  return {
    react: extractVersion(imports['react']),
    reactDom: extractVersion(imports['react-dom']),
    useVibes: extractVersion(imports['use-vibes']),
    useFireproof: extractVersion(imports['use-fireproof']),
    clerk: extractVersion(imports['@clerk/clerk-react'])
  };
}

/**
 * Detect import map query parameter patterns
 * @param {object} imports - Import map imports object
 * @returns {object} - Pattern flags
 */
function detectQueryPatterns(imports) {
  if (!imports) return { usesExternal: false, usesDeps: false };

  const useVibesUrl = imports['use-vibes'] || '';

  return {
    usesExternal: useVibesUrl.includes('?external='),
    usesDeps: useVibesUrl.includes('?deps='),
    reactPinned: (imports['react'] || '').includes('@18')
  };
}

/**
 * Find app code boundaries in HTML
 * @param {string} html - The HTML content
 * @returns {object} - Start and end positions, or null
 */
function findAppCodeBoundaries(html) {
  // Standard placeholder
  const placeholderIndex = html.indexOf('// __VIBES_APP_CODE__');
  if (placeholderIndex !== -1) {
    return {
      type: 'placeholder',
      position: placeholderIndex,
      marker: '// __VIBES_APP_CODE__'
    };
  }

  // Look for Babel script block with app code
  const babelMatch = html.match(/<script\s+type=["']text\/babel["'][^>]*data-type=["']module["'][^>]*>([\s\S]*?)<\/script>/i);
  if (babelMatch) {
    const start = html.indexOf(babelMatch[0]) + babelMatch[0].indexOf('>') + 1;
    const end = start + babelMatch[1].length;
    return {
      type: 'babel-module',
      start,
      end,
      content: babelMatch[1]
    };
  }

  return null;
}

/**
 * Detect VibesSwitch component version
 * @param {string} html - The HTML content
 * @returns {string|null} - Version identifier or null
 */
function detectVibesSwitchVersion(html) {
  // Look for VibesSwitch component markers
  if (!html.includes('VibesSwitch')) {
    return null;
  }

  // v1: Original with basic SVG
  // v2: With animation improvements
  // Check for specific patterns to identify version

  if (html.includes('vibes-switch-container') && html.includes('@keyframes')) {
    return 'v2';
  }

  if (html.includes('VibesSwitch')) {
    return 'v1';
  }

  return null;
}

/**
 * Detect HiddenMenuWrapper component presence and version
 * @param {string} html - The HTML content
 * @returns {string|null} - Version identifier or null
 */
function detectMenuWrapperVersion(html) {
  if (!html.includes('HiddenMenuWrapper')) {
    return null;
  }

  // Check for specific version markers
  if (html.includes('menu-slide-up')) {
    return 'v2';
  }

  return 'v1';
}

/**
 * Analyze an HTML file and return comprehensive analysis
 * @param {string} filePath - Path to the HTML file
 * @returns {object} - Analysis result
 */
function analyze(filePath) {
  const resolvedPath = resolve(filePath);

  if (!existsSync(resolvedPath)) {
    return {
      success: false,
      error: `File not found: ${resolvedPath}`,
      filePath: resolvedPath
    };
  }

  let html;
  try {
    html = readFileSync(resolvedPath, 'utf-8');
  } catch (e) {
    return {
      success: false,
      error: `Could not read file: ${e.message}`,
      filePath: resolvedPath
    };
  }

  // Extract import map
  const importMap = extractImportMap(html);
  const imports = importMap?.imports || {};

  // Detect template type
  const templateType = detectTemplateType(html, importMap);

  // Detect era
  const era = detectEra(imports, templateType);

  // Extract versions
  const versions = extractLibraryVersions(imports);

  // Detect patterns
  const patterns = detectQueryPatterns(imports);

  // Find app code
  const appCodeBoundaries = findAppCodeBoundaries(html);

  // Component versions
  const components = {
    vibesSwitch: detectVibesSwitchVersion(html),
    menuWrapper: detectMenuWrapperVersion(html)
  };

  return {
    success: true,
    filePath: resolvedPath,
    fileName: basename(resolvedPath),

    // Template info
    templateType,
    era,
    eraInfo: ERAS[era] || { name: 'Unknown', markers: [], notes: 'Could not determine era' },

    // Versions
    versions,

    // Patterns
    patterns,

    // Import map (raw)
    importMap,

    // App code location
    appCodeBoundaries,

    // Component versions
    components,

    // File stats
    fileSize: html.length,
    hasImportMap: !!importMap
  };
}

/**
 * Analyze multiple files (batch mode)
 * @param {string[]} filePaths - Array of file paths
 * @returns {object[]} - Array of analysis results
 */
function analyzeMultiple(filePaths) {
  return filePaths.map(filePath => analyze(filePath));
}

export {
  analyze,
  analyzeMultiple,
  extractImportMap,
  detectTemplateType,
  detectEra,
  extractLibraryVersions,
  detectQueryPatterns,
  findAppCodeBoundaries,
  ERAS
};
