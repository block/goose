/**
 * compare.js - Compare analyzed app against current plugin state
 *
 * Phase 2 of the update pipeline:
 * - Load current plugin versions from cache
 * - Compare against analyzed app versions
 * - Identify available updates
 */

import { readFileSync, existsSync } from 'fs';
import { join } from 'path';
import { extractVersion } from './parsers.js';
import { PLUGIN_ROOT } from './paths.js';

/**
 * Validate import map cache structure
 * @param {object} cache - Parsed cache object
 * @returns {boolean} - True if valid
 */
function validateCacheSchema(cache) {
  if (!cache || typeof cache !== 'object') {
    return false;
  }
  if (!cache.imports || typeof cache.imports !== 'object') {
    return false;
  }
  // Check that imports is not empty and has valid URL values
  const importKeys = Object.keys(cache.imports);
  if (importKeys.length === 0) {
    return false;
  }
  // Verify at least one required key exists
  const requiredKeys = ['react', 'use-vibes'];
  const hasRequiredKey = requiredKeys.some(key => key in cache.imports);
  if (!hasRequiredKey) {
    return false;
  }
  return true;
}

/**
 * Load import map from cache
 * @param {string} pluginRoot - Plugin root directory
 * @returns {object|null} - Cached import map or null
 */
function loadCachedImportMap(pluginRoot) {
  // Try working cache first
  const workingCachePath = join(pluginRoot, 'cache', 'import-map.json');
  if (existsSync(workingCachePath)) {
    try {
      const cache = JSON.parse(readFileSync(workingCachePath, 'utf-8'));
      if (!validateCacheSchema(cache)) {
        console.warn('Warning: Working cache import map has invalid structure');
      } else {
        return {
          imports: cache.imports,
          source: 'working-cache',
          lastUpdated: cache.lastUpdated
        };
      }
    } catch (e) {
      console.warn('Warning: Could not parse working cache import map');
    }
  }

  // Fall back to shipped cache
  const shippedCachePath = join(pluginRoot, 'skills', 'vibes', 'cache', 'import-map.json');
  if (existsSync(shippedCachePath)) {
    try {
      const cache = JSON.parse(readFileSync(shippedCachePath, 'utf-8'));
      if (!validateCacheSchema(cache)) {
        console.warn('Warning: Shipped cache import map has invalid structure');
      } else {
        return {
          imports: cache.imports,
          source: 'shipped-cache',
          lastUpdated: cache.lastUpdated
        };
      }
    } catch (e) {
      console.warn('Warning: Could not parse shipped cache import map');
    }
  }

  return null;
}

/**
 * Compare two semantic versions for ordering.
 *
 * Handles standard semver (1.2.3) and dev/preview versions (0.19.0-dev-preview-50).
 * Dev versions are considered "newer" than their base version for upgrade detection,
 * meaning we want to upgrade FROM dev TO stable.
 *
 * @param {string} v1 - First version string (e.g., "0.24.3-dev" or "0.19.0-dev-preview-50")
 * @param {string} v2 - Second version string
 * @returns {number} Comparison result:
 *   - -1 if v1 < v2 (v1 is older, should upgrade to v2)
 *   - 0 if versions are equivalent
 *   - 1 if v1 > v2 (v1 is newer)
 * @example
 * compareVersions("0.20.0", "0.24.3-dev")  // -1 (0.20.0 is older)
 * compareVersions("0.19.0", "0.19.0")  // 0 (equal)
 * compareVersions("0.19.0-dev", "0.19.0")  // 1 (dev > stable for downgrade detection)
 */
function compareVersions(v1, v2) {
  if (!v1 || !v2) return 0;

  // Handle dev versions - they sort after release versions
  const v1IsDev = v1.includes('-dev') || v1.includes('-preview');
  const v2IsDev = v2.includes('-dev') || v2.includes('-preview');

  // Strip dev/preview suffixes for base comparison
  const v1Base = v1.replace(/-dev.*|-preview.*/, '');
  const v2Base = v2.replace(/-dev.*|-preview.*/, '');

  const parts1 = v1Base.split('.').map(n => parseInt(n, 10) || 0);
  const parts2 = v2Base.split('.').map(n => parseInt(n, 10) || 0);

  // Compare base versions
  for (let i = 0; i < Math.max(parts1.length, parts2.length); i++) {
    const p1 = parts1[i] || 0;
    const p2 = parts2[i] || 0;
    if (p1 < p2) return -1;
    if (p1 > p2) return 1;
  }

  // Base versions are equal - dev versions are "newer" for upgrade purposes
  // but we want to upgrade FROM dev TO stable
  if (v1IsDev && !v2IsDev) return 1; // v1 (dev) > v2 (stable) in terms of needing downgrade
  if (!v1IsDev && v2IsDev) return -1; // v1 (stable) < v2 (dev)

  return 0;
}

/**
 * Compare analysis against current plugin state
 * @param {object} analysis - Result from analyze()
 * @returns {object} - Comparison result
 */
function compare(analysis) {
  if (!analysis.success) {
    return {
      success: false,
      error: analysis.error
    };
  }

  const cachedImportMap = loadCachedImportMap(PLUGIN_ROOT);

  if (!cachedImportMap) {
    return {
      success: false,
      error: 'Could not load plugin cache. Run "vibes sync" first.'
    };
  }

  // Extract target versions from cache
  const targetVersions = {
    react: extractVersion(cachedImportMap.imports['react']),
    reactDom: extractVersion(cachedImportMap.imports['react-dom']),
    useVibes: extractVersion(cachedImportMap.imports['use-vibes']),
    useFireproof: extractVersion(cachedImportMap.imports['use-fireproof'])
  };

  // Compare each library
  const versionDiffs = {};
  for (const [lib, currentVersion] of Object.entries(analysis.versions)) {
    const targetVersion = targetVersions[lib];
    if (currentVersion && targetVersion) {
      const comparison = compareVersions(currentVersion, targetVersion);
      versionDiffs[lib] = {
        current: currentVersion,
        target: targetVersion,
        status: comparison < 0 ? 'outdated' : comparison > 0 ? 'newer' : 'current',
        needsUpdate: comparison < 0
      };
    } else if (!currentVersion && targetVersion) {
      versionDiffs[lib] = {
        current: null,
        target: targetVersion,
        status: 'missing',
        needsUpdate: true
      };
    }
  }

  // Check for pattern issues
  const patternIssues = [];

  // Check ?external= vs ?deps= (vibes-basic should use ?external=)
  if (analysis.templateType === 'vibes-basic') {
    if (analysis.patterns.usesDeps && !analysis.patterns.usesExternal) {
      patternIssues.push({
        id: 'deps-to-external',
        description: 'Using ?deps= instead of ?external= for React singleton',
        severity: 'recommended'
      });
    }
    if (!analysis.patterns.usesExternal && !analysis.patterns.usesDeps) {
      patternIssues.push({
        id: 'missing-external',
        description: 'Missing ?external= parameter on use-vibes imports',
        severity: 'important'
      });
    }
  }

  // Check for dev versions in production
  const currentUseVibes = analysis.versions.useVibes || '';
  if (currentUseVibes.includes('-dev') || currentUseVibes.includes('-preview')) {
    patternIssues.push({
      id: 'dev-version',
      description: `Using development version (${currentUseVibes})`,
      severity: 'recommended'
    });
  }

  // Identify available updates
  const availableUpdates = [];

  // Import map update if any versions are outdated
  const hasOutdatedVersions = Object.values(versionDiffs).some(d => d.needsUpdate);
  if (hasOutdatedVersions) {
    availableUpdates.push({
      id: 'import-map',
      type: 'import-map-replace',
      name: 'Update import map',
      description: 'Update library versions to latest stable',
      priority: 'recommended',
      breaking: false,
      affectedLibs: Object.entries(versionDiffs)
        .filter(([_, d]) => d.needsUpdate)
        .map(([lib, d]) => `${lib}: ${d.current} → ${d.target}`)
    });
  }

  // Pattern fixes
  for (const issue of patternIssues) {
    if (issue.id === 'deps-to-external') {
      availableUpdates.push({
        id: 'deps-to-external',
        type: 'deps-to-external',
        name: 'Fix React singleton pattern',
        description: 'Migrate ?deps= to ?external= for proper React singleton',
        priority: 'recommended',
        breaking: false
      });
    }
    if (issue.id === 'missing-external') {
      availableUpdates.push({
        id: 'add-external',
        type: 'import-map-replace',
        name: 'Add ?external= parameters',
        description: 'Add ?external=react,react-dom to prevent duplicate React instances',
        priority: 'important',
        breaking: false
      });
    }
  }

  // Component updates
  if (analysis.components.vibesSwitch === 'v1') {
    availableUpdates.push({
      id: 'vibes-switch',
      type: 'component-replace',
      name: 'Update VibesSwitch component',
      description: 'Update to latest VibesSwitch with improved animations',
      priority: 'optional',
      breaking: false
    });
  }

  return {
    success: true,
    analysis,
    cache: {
      source: cachedImportMap.source,
      lastUpdated: cachedImportMap.lastUpdated
    },
    target: {
      versions: targetVersions,
      importMap: cachedImportMap.imports
    },
    versionDiffs,
    patternIssues,
    availableUpdates,
    summary: {
      outdatedLibs: Object.values(versionDiffs).filter(d => d.needsUpdate).length,
      patternIssueCount: patternIssues.length,
      availableUpdateCount: availableUpdates.length,
      hasRecommendedUpdates: availableUpdates.some(u => u.priority === 'recommended'),
      hasImportantUpdates: availableUpdates.some(u => u.priority === 'important')
    }
  };
}

export {
  compare,
  loadCachedImportMap,
  compareVersions,
  validateCacheSchema,
  PLUGIN_ROOT
};
