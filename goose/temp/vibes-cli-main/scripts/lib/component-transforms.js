/**
 * Component transformation utilities
 *
 * Pure functions for transforming component source code.
 * Extracted from build-components.js for testability.
 */

/**
 * Remove import statements from transpiled code
 * @param {string} code - Transpiled JavaScript code
 * @returns {string} - Code with imports removed
 */
export function removeImports(code) {
  return code
    .replace(/^import\s+\w+\s*,\s*\{[^}]+\}\s+from\s+["'][^"']+["'];?\n?/gm, "")  // import X, { y } from "..."
    .replace(/^import\s+\{[^}]+\}\s+from\s+["'][^"']+["'];?\n?/gm, "")            // import { x } from "..."
    .replace(/^import\s+[\w]+\s+from\s+["'][^"']+["'];?\n?/gm, "")                // import x from "..."
    .replace(/^import\s+type\s+[^\n]+\n?/gm, "")                                  // import type ...
    .replace(/^export\s+/gm, "");                                                 // export keyword
}

/**
 * Add React. prefix to hook calls
 * @param {string} code - JavaScript code
 * @returns {string} - Code with React-prefixed hooks
 */
export function prefixReactHooks(code) {
  return code
    .replace(/(?<!React\.)useState\(/g, "React.useState(")
    .replace(/(?<!React\.)useEffect\(/g, "React.useEffect(")
    .replace(/(?<!React\.)useRef\(/g, "React.useRef(")
    .replace(/(?<!React\.)useCallback\(/g, "React.useCallback(")
    .replace(/(?<!React\.)useMemo\(/g, "React.useMemo(")
    .replace(/(?<!React\.)useId\(/g, "React.useId(")
    .replace(/(?<!React\.)useLayoutEffect\b/g, "React.useLayoutEffect")
    .replace(/(?<!React\.)forwardRef\b/g, "React.forwardRef");
}

/**
 * Namespace conflicting function names for VibesButton
 * Both VibesButton.styles and HiddenMenuWrapper.styles define getContentWrapperStyle
 * @param {string} code - JavaScript code
 * @param {string} componentName - Name of the component being processed
 * @returns {string} - Code with namespaced function names
 */
export function namespaceVibesButtonFunctions(code, componentName) {
  if (componentName === "VibesButton.styles" || componentName === "VibesButton") {
    return code.replace(
      /\bgetContentWrapperStyle\b/g,
      "getVibesButtonContentWrapperStyle"
    );
  }
  return code;
}

/**
 * Functions that collide between LabelContainer.styles and VibesPanel.styles
 */
const COLLIDING_STYLE_FUNCTIONS = [
  "getContainerStyle",
  "getLabelStyle",
  "getButtonWrapperStyle",
  "getResponsiveLabelStyle",
  "getResponsiveButtonWrapperStyle",
  "getResponsiveContainerStyle"
];

/**
 * Namespace conflicting function names for LabelContainer and VibesPanel
 * Both components define identically-named style functions
 * @param {string} code - JavaScript code
 * @param {string} componentName - Name of the component being processed
 * @returns {string} - Code with namespaced function names
 */
export function namespaceCollidingFunctions(code, componentName) {
  // Handle LabelContainer.styles
  if (componentName === "LabelContainer.styles" || componentName === "LabelContainer") {
    for (const fn of COLLIDING_STYLE_FUNCTIONS) {
      const prefixed = fn.replace(/^get/, "getLabelContainer");
      code = code.replace(new RegExp(`\\b${fn}\\b`, "g"), prefixed);
    }
  }

  // Handle VibesPanel.styles
  if (componentName === "VibesPanel.styles" || componentName === "VibesPanel") {
    for (const fn of COLLIDING_STYLE_FUNCTIONS) {
      const prefixed = fn.replace(/^get/, "getVibesPanel");
      code = code.replace(new RegExp(`\\b${fn}\\b`, "g"), prefixed);
    }
  }

  return code;
}

/**
 * Apply all component transformations
 * @param {string} code - Transpiled JavaScript code
 * @param {string} componentName - Name of the component
 * @returns {string} - Fully transformed code
 */
export function transformComponent(code, componentName) {
  let result = removeImports(code);
  result = prefixReactHooks(result);
  result = namespaceVibesButtonFunctions(result, componentName);
  result = namespaceCollidingFunctions(result, componentName);
  return result;
}
