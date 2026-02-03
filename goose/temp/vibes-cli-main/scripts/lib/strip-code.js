/**
 * Utility functions for stripping import/export statements from JSX code
 * before injecting into templates.
 */

/**
 * Remove all import statements from code
 * @param {string} code - Source code
 * @returns {string} Code with imports removed
 */
export function stripImports(code) {
  return code
    // Multi-line imports: import { ... } from "..."
    .replace(/^import\s+\{[\s\S]*?\}\s+from\s+["'].*?["'];?\s*$/gm, '')
    // Single-line named/default imports: import X from "..."
    .replace(/^import\s+.*?from\s+["'].*?["'];?\s*$/gm, '')
    // Side-effect imports: import "..."
    .replace(/^import\s+["'].*?["'];?\s*$/gm, '');
}

/**
 * Remove "export default" from function/class declarations
 * @param {string} code - Source code
 * @returns {string} Code with export default removed
 */
export function stripExportDefault(code) {
  return code.replace(/^export\s+default\s+/m, '');
}

/**
 * Remove CONFIG object declarations
 * @param {string} code - Source code
 * @returns {string} Code with CONFIG removed
 */
export function stripConfig(code) {
  return code.replace(/^const\s+CONFIG\s*=\s*\{[\s\S]*?\n\};?\s*$/gm, '');
}

/**
 * Remove declarations of template-provided constants
 * @param {string} code - Source code
 * @param {string[]} constants - Array of constant names to remove
 * @returns {string} Code with constants removed
 */
export function stripConstants(code, constants) {
  let result = code;
  for (const constant of constants) {
    result = result.replace(new RegExp(`^const\\s+${constant}\\s*=.*$`, 'gm'), '');
  }
  return result;
}

/**
 * Strip all template conflicts from app code (imports, exports, CONFIG, constants)
 * @param {string} code - Source code
 * @param {string[]} [templateConstants] - Additional constants the template provides
 * @returns {string} Cleaned code ready for template injection
 */
export function stripForTemplate(code, templateConstants = []) {
  let result = code.trim();
  result = stripImports(result);
  result = stripExportDefault(result);
  result = stripConfig(result);
  if (templateConstants.length > 0) {
    result = stripConstants(result, templateConstants);
  }
  return result;
}

