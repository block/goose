/**
 * registry.js - Update definitions and execution
 *
 * Central registry of all available updates with:
 * - Conditions for when updates apply
 * - Transform functions to execute
 * - Metadata for display
 */

import { applyImportMapUpdate, migrateDepsToExternal, addExternalParams } from '../lib/transforms/import-map.js';
import { replaceComponent, isComponentModified } from '../lib/transforms/component-replace.js';
import { addMissingConfigFields, findMissingFields, parseConfig } from '../lib/transforms/config-merge.js';
import { readFileSync } from 'fs';

/**
 * Update registry - defines all available updates
 *
 * Each update has:
 * - id: Unique identifier
 * - name: Human-readable name
 * - description: Detailed description
 * - condition: Function to check if update applies (receives analysis and comparison)
 * - priority: 'important' | 'recommended' | 'optional'
 * - breaking: Boolean indicating if update may break things
 * - templates: Array of template types this applies to
 * - execute: Function to apply the update (receives html, analysis, comparison)
 */
const UPDATE_REGISTRY = [
  // Import map updates
  {
    id: 'import-map',
    name: 'Update import map',
    description: 'Update library versions to latest stable',
    condition: (analysis, comparison) => {
      if (!comparison.versionDiffs) return false;
      return Object.values(comparison.versionDiffs).some(d => d.needsUpdate);
    },
    priority: 'recommended',
    breaking: false,
    templates: ['vibes-basic', 'sell'],
    execute: (html, analysis, comparison) => {
      return applyImportMapUpdate(html, analysis);
    }
  },

  // Pattern fixes
  {
    id: 'deps-to-external',
    name: 'Fix React singleton pattern',
    description: 'Migrate ?deps= to ?external= for proper React singleton',
    condition: (analysis, comparison) => {
      return analysis.templateType === 'vibes-basic' &&
             analysis.patterns.usesDeps &&
             !analysis.patterns.usesExternal;
    },
    priority: 'recommended',
    breaking: false,
    templates: ['vibes-basic'],
    execute: (html, analysis, comparison) => {
      return migrateDepsToExternal(html);
    }
  },

  {
    id: 'add-external',
    name: 'Add ?external= parameters',
    description: 'Add ?external=react,react-dom to prevent duplicate React instances',
    condition: (analysis, comparison) => {
      return analysis.templateType === 'vibes-basic' &&
             !analysis.patterns.usesExternal &&
             !analysis.patterns.usesDeps;
    },
    priority: 'important',
    breaking: false,
    templates: ['vibes-basic'],
    execute: (html, analysis, comparison) => {
      return addExternalParams(html);
    }
  },

  // Component updates
  {
    id: 'vibes-switch',
    name: 'Update VibesSwitch component',
    description: 'Update to latest VibesSwitch with improved animations and accessibility',
    condition: (analysis, comparison) => {
      return analysis.components.vibesSwitch === 'v1' &&
             !isComponentModified(analysis.filePath, 'vibesSwitch');
    },
    priority: 'optional',
    breaking: false,
    templates: ['vibes-basic'],
    execute: (html, analysis, comparison) => {
      return replaceComponent(html, 'vibesSwitch');
    }
  },

  {
    id: 'menu-wrapper',
    name: 'Update HiddenMenuWrapper component',
    description: 'Update to latest HiddenMenuWrapper with improved slide animation',
    condition: (analysis, comparison) => {
      return analysis.components.menuWrapper === 'v1' &&
             !isComponentModified(analysis.filePath, 'hiddenMenuWrapper');
    },
    priority: 'optional',
    breaking: false,
    templates: ['vibes-basic'],
    execute: (html, analysis, comparison) => {
      return replaceComponent(html, 'hiddenMenuWrapper');
    }
  },

  // Sell-specific updates
  {
    id: 'config-fields',
    name: 'Add missing CONFIG fields',
    description: 'Add new CONFIG fields with placeholder values',
    condition: (analysis, comparison) => {
      if (analysis.templateType !== 'sell') return false;

      // Check if any fields are missing
      try {
        const html = readFileSync(analysis.filePath, 'utf-8');
        const config = parseConfig(html);
        if (!config) return false;

        const missing = findMissingFields(config);
        return missing.length > 0;
      } catch (e) {
        return false;
      }
    },
    priority: 'optional',
    breaking: false,
    templates: ['sell'],
    execute: (html, analysis, comparison) => {
      return addMissingConfigFields(html);
    }
  }
];

/**
 * Get updates applicable to a specific analysis/comparison
 * @param {object} analysis - Analysis result
 * @param {object} comparison - Comparison result
 * @returns {object[]} - Applicable updates
 */
function getApplicableUpdates(analysis, comparison) {
  return UPDATE_REGISTRY.filter(update => {
    // Check template type
    if (!update.templates.includes(analysis.templateType)) {
      return false;
    }

    // Check condition
    try {
      return update.condition(analysis, comparison);
    } catch (e) {
      console.warn(`Warning: Error checking condition for ${update.id}: ${e.message}`);
      return false;
    }
  });
}

/**
 * Execute an update
 * @param {object} update - Update definition
 * @param {string} html - HTML content
 * @param {object} analysis - Analysis result
 * @param {object} comparison - Comparison result
 * @returns {object} - Transform result
 */
function executeUpdate(update, html, analysis, comparison) {
  try {
    return update.execute(html, analysis, comparison);
  } catch (e) {
    return {
      success: false,
      error: `Error executing ${update.id}: ${e.message}`
    };
  }
}

/**
 * Get update by ID
 * @param {string} id - Update ID
 * @returns {object|null} - Update definition or null
 */
function getUpdateById(id) {
  return UPDATE_REGISTRY.find(u => u.id === id) || null;
}

export {
  UPDATE_REGISTRY,
  getApplicableUpdates,
  executeUpdate,
  getUpdateById
};
