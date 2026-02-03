/**
 * Template merging utilities
 *
 * Pure functions for merging base + delta templates.
 * Extracted from merge-templates.js for testability.
 */

/**
 * Merge a skill template from base + delta + components
 * @param {object} skill - Skill configuration { name, title }
 * @param {string} baseTemplate - Base HTML template content
 * @param {string} components - Built components JavaScript
 * @param {string} delta - Skill-specific delta HTML
 * @returns {string} - Merged template
 */
export function mergeTemplate(skill, baseTemplate, components, delta) {
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
 * Check if a template contains required placeholders
 * @param {string} template - Template content
 * @returns {{valid: boolean, missing: string[]}} - Validation result
 */
export function validateBasePlaceholders(template) {
  const required = [
    "__TITLE__",
    "// === COMPONENTS_PLACEHOLDER ===",
    "<!-- === DELTA_PLACEHOLDER === -->"
  ];

  const missing = required.filter(p => !template.includes(p));

  return {
    valid: missing.length === 0,
    missing
  };
}
