/**
 * plan.js - Generate and format update plans
 *
 * Phase 3 of the update pipeline:
 * - Format comparison results into readable plan
 * - Filter and sort available updates
 * - Generate actionable output for user review
 */

import { ERAS } from './analyze.js';

/**
 * ANSI color codes for terminal output
 */
const colors = {
  reset: '\x1b[0m',
  bold: '\x1b[1m',
  dim: '\x1b[2m',
  green: '\x1b[32m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  red: '\x1b[31m',
  cyan: '\x1b[36m'
};

/**
 * Priority icons for display
 */
const priorityIcons = {
  important: '!',
  recommended: '*',
  optional: '-'
};

/**
 * Priority colors for display
 */
const priorityColors = {
  important: colors.red,
  recommended: colors.yellow,
  optional: colors.dim
};

/**
 * Generate a formatted plan from comparison results
 * @param {object} comparison - Result from compare()
 * @param {object} options - Display options
 * @returns {object} - Formatted plan
 */
function generatePlan(comparison, options = {}) {
  if (!comparison.success) {
    return {
      success: false,
      error: comparison.error,
      output: `Error: ${comparison.error}`
    };
  }

  const { analysis, versionDiffs, availableUpdates, summary } = comparison;

  // Sort updates by priority
  const sortedUpdates = [...availableUpdates].sort((a, b) => {
    const priorityOrder = { important: 0, recommended: 1, optional: 2 };
    return (priorityOrder[a.priority] || 3) - (priorityOrder[b.priority] || 3);
  });

  // Assign numeric IDs for CLI selection
  const numberedUpdates = sortedUpdates.map((update, index) => ({
    ...update,
    number: index + 1
  }));

  return {
    success: true,
    analysis,
    updates: numberedUpdates,
    summary,
    hasUpdates: numberedUpdates.length > 0
  };
}

/**
 * Format plan as terminal output
 * @param {object} plan - Generated plan
 * @param {object} options - Display options
 * @returns {string} - Formatted terminal output
 */
function formatPlanOutput(plan, options = {}) {
  if (!plan.success) {
    return `${colors.red}Error:${colors.reset} ${plan.error}`;
  }

  const { analysis, updates, summary } = plan;
  const lines = [];

  // Header
  lines.push('');
  lines.push(`${colors.bold}vibes update${colors.reset} analysis for: ${colors.cyan}${analysis.fileName}${colors.reset}`);
  lines.push('');

  // Current state
  lines.push(`${colors.bold}Current state:${colors.reset}`);
  lines.push(`  Template: ${analysis.templateType}`);

  if (analysis.versions.useVibes) {
    lines.push(`  use-vibes: ${analysis.versions.useVibes}`);
  }

  lines.push(`  Era: ${analysis.eraInfo.name}`);

  if (analysis.eraInfo.notes) {
    lines.push(`  ${colors.dim}${analysis.eraInfo.notes}${colors.reset}`);
  }

  lines.push('');

  // Available updates
  if (updates.length === 0) {
    lines.push(`${colors.green}✓ App is up to date!${colors.reset}`);
    lines.push('');
  } else {
    lines.push(`${colors.bold}Available updates:${colors.reset}`);
    lines.push('');

    for (const update of updates) {
      const icon = priorityIcons[update.priority] || '-';
      const color = priorityColors[update.priority] || colors.reset;

      lines.push(`  ${colors.bold}[${update.number}]${colors.reset} ${update.name}`);
      lines.push(`      ${colors.dim}${update.description}${colors.reset}`);

      if (update.affectedLibs && update.affectedLibs.length > 0) {
        for (const lib of update.affectedLibs) {
          lines.push(`      ${colors.dim}• ${lib}${colors.reset}`);
        }
      }

      const priorityLabel = update.priority.charAt(0).toUpperCase() + update.priority.slice(1);
      const breakingLabel = update.breaking ? `, ${colors.red}breaking${colors.reset}` : '';
      lines.push(`      ${color}[${priorityLabel}]${colors.reset}${breakingLabel}`);
      lines.push('');
    }

    // Usage hint
    lines.push(`${colors.dim}Run with --apply to execute all updates${colors.reset}`);
    lines.push(`${colors.dim}Run with --apply=1,2 to execute specific updates${colors.reset}`);
    lines.push('');
  }

  return lines.join('\n');
}

/**
 * Format plan as JSON (for programmatic use)
 * @param {object} plan - Generated plan
 * @returns {string} - JSON output
 */
function formatPlanJson(plan) {
  return JSON.stringify(plan, null, 2);
}

/**
 * Filter updates by IDs or numbers
 * @param {object} plan - Generated plan
 * @param {string} selector - Comma-separated IDs or numbers (e.g., "1,2" or "import-map,vibes-switch")
 * @returns {object[]} - Filtered updates
 */
function filterUpdates(plan, selector) {
  if (!selector || selector === 'all' || selector === true) {
    return plan.updates;
  }

  const selections = selector.split(',').map(s => s.trim());

  return plan.updates.filter(update => {
    // Match by number
    if (selections.includes(String(update.number))) {
      return true;
    }
    // Match by ID
    if (selections.includes(update.id)) {
      return true;
    }
    return false;
  });
}

/**
 * Format batch summary for multiple files
 * @param {object[]} plans - Array of generated plans
 * @returns {string} - Formatted summary
 */
function formatBatchSummary(plans) {
  const lines = [];

  lines.push('');
  lines.push(`${colors.bold}Batch analysis summary:${colors.reset}`);
  lines.push('');

  const upToDate = plans.filter(p => p.success && !p.hasUpdates);
  const needsUpdate = plans.filter(p => p.success && p.hasUpdates);
  const failed = plans.filter(p => !p.success);

  if (upToDate.length > 0) {
    lines.push(`${colors.green}✓ Up to date (${upToDate.length}):${colors.reset}`);
    for (const plan of upToDate) {
      lines.push(`    ${plan.analysis.fileName}`);
    }
    lines.push('');
  }

  if (needsUpdate.length > 0) {
    lines.push(`${colors.yellow}! Needs update (${needsUpdate.length}):${colors.reset}`);
    for (const plan of needsUpdate) {
      const updateCount = plan.updates.length;
      lines.push(`    ${plan.analysis.fileName} (${updateCount} update${updateCount === 1 ? '' : 's'})`);
    }
    lines.push('');
  }

  if (failed.length > 0) {
    lines.push(`${colors.red}✗ Failed (${failed.length}):${colors.reset}`);
    for (const plan of failed) {
      lines.push(`    ${plan.error}`);
    }
    lines.push('');
  }

  // Total summary
  lines.push(`${colors.dim}Total: ${plans.length} file(s) analyzed${colors.reset}`);
  lines.push('');

  return lines.join('\n');
}

export {
  generatePlan,
  formatPlanOutput,
  formatPlanJson,
  filterUpdates,
  formatBatchSummary,
  colors
};
