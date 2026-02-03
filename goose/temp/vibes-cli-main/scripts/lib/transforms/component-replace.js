/**
 * component-replace.js - Component replacement transform
 *
 * Deterministically replaces known components (VibesSwitch, HiddenMenuWrapper)
 * with updated versions from plugin templates.
 */

import { readFileSync, existsSync } from 'fs';
import { join } from 'path';
import { PLUGIN_ROOT } from '../paths.js';

/**
 * Component markers for identification
 */
const COMPONENT_MARKERS = {
  vibesSwitch: {
    // VibesSwitch has distinctive SVG and animation patterns
    startPatterns: [
      /function\s+VibesSwitch\s*\(/,
      /const\s+VibesSwitch\s*=\s*\(/,
      /\/\/\s*VibesSwitch\s+component/i
    ],
    endPatterns: [
      /\/\/\s*End\s+VibesSwitch/i,
      // Match end of function declaration
      /^(\s*}\s*;?\s*)$/m
    ],
    // Distinctive content to verify we have the right component
    contentMarkers: ['vibes-switch', 'toggle', 'svg', 'animate']
  },
  hiddenMenuWrapper: {
    startPatterns: [
      /function\s+HiddenMenuWrapper\s*\(/,
      /const\s+HiddenMenuWrapper\s*=\s*\(/,
      /\/\/\s*HiddenMenuWrapper\s+component/i
    ],
    endPatterns: [
      /\/\/\s*End\s+HiddenMenuWrapper/i
    ],
    contentMarkers: ['menu', 'slide', 'hidden']
  }
};

/**
 * Find component boundaries in HTML
 * @param {string} html - The HTML content
 * @param {string} componentName - Name of component to find
 * @returns {object|null} - Start/end positions or null
 */
function findComponentBoundaries(html, componentName) {
  const markers = COMPONENT_MARKERS[componentName];
  if (!markers) {
    return null;
  }

  // Find start position
  let startMatch = null;
  let startPos = -1;

  for (const pattern of markers.startPatterns) {
    const match = html.match(pattern);
    if (match) {
      startPos = html.indexOf(match[0]);
      startMatch = match[0];
      break;
    }
  }

  if (startPos === -1) {
    return null;
  }

  // Find end position - look for explicit end marker first
  let endPos = -1;

  for (const pattern of markers.endPatterns) {
    const afterStart = html.slice(startPos);
    const match = afterStart.match(pattern);
    if (match) {
      endPos = startPos + afterStart.indexOf(match[0]) + match[0].length;
      break;
    }
  }

  // If no explicit end marker, try to find the end of the function
  if (endPos === -1) {
    endPos = findFunctionEnd(html, startPos);
  }

  if (endPos === -1) {
    return null;
  }

  return {
    start: startPos,
    end: endPos,
    content: html.slice(startPos, endPos)
  };
}

/**
 * Find the end of a function starting at given position
 * Uses brace matching
 * @param {string} html - The HTML content
 * @param {number} startPos - Start position
 * @returns {number} - End position or -1
 */
function findFunctionEnd(html, startPos) {
  const afterStart = html.slice(startPos);

  // Find the first opening brace
  const firstBrace = afterStart.indexOf('{');
  if (firstBrace === -1) return -1;

  let braceCount = 0;
  let inString = false;
  let stringChar = '';
  let escaped = false;

  for (let i = firstBrace; i < afterStart.length; i++) {
    const char = afterStart[i];

    if (escaped) {
      escaped = false;
      continue;
    }

    if (char === '\\') {
      escaped = true;
      continue;
    }

    if (!inString && (char === '"' || char === "'" || char === '`')) {
      inString = true;
      stringChar = char;
      continue;
    }

    if (inString && char === stringChar) {
      inString = false;
      continue;
    }

    if (!inString) {
      if (char === '{') braceCount++;
      if (char === '}') braceCount--;

      if (braceCount === 0) {
        // Find the end of the statement (semicolon or newline)
        let endOffset = i + 1;
        while (endOffset < afterStart.length) {
          const nextChar = afterStart[endOffset];
          if (nextChar === ';' || nextChar === '\n') {
            endOffset++;
            break;
          }
          if (!/\s/.test(nextChar)) break;
          endOffset++;
        }
        return startPos + endOffset;
      }
    }
  }

  return -1;
}

/**
 * Load component code from plugin template
 * @param {string} componentName - Name of component
 * @returns {string|null} - Component code or null
 */
function loadComponentFromTemplate(componentName) {
  // Try to load from template
  const templatePath = join(PLUGIN_ROOT, 'skills', 'vibes', 'templates', 'index.html');

  if (!existsSync(templatePath)) {
    return null;
  }

  const template = readFileSync(templatePath, 'utf-8');
  const boundaries = findComponentBoundaries(template, componentName);

  if (!boundaries) {
    return null;
  }

  return boundaries.content;
}

/**
 * Replace a component in HTML with new version
 * @param {string} html - The HTML content
 * @param {string} componentName - Name of component to replace
 * @returns {object} - Transform result
 */
function replaceComponent(html, componentName) {
  // Find existing component
  const existingBoundaries = findComponentBoundaries(html, componentName);

  if (!existingBoundaries) {
    return {
      success: false,
      error: `Component ${componentName} not found in HTML`
    };
  }

  // Load new component
  const newComponent = loadComponentFromTemplate(componentName);

  if (!newComponent) {
    return {
      success: false,
      error: `Could not load ${componentName} from plugin template`
    };
  }

  // Perform replacement
  const newHtml =
    html.slice(0, existingBoundaries.start) +
    newComponent +
    html.slice(existingBoundaries.end);

  return {
    success: true,
    html: newHtml,
    diff: {
      component: componentName,
      before: existingBoundaries.content.slice(0, 100) + '...',
      after: newComponent.slice(0, 100) + '...'
    }
  };
}

/**
 * Check if a component has been modified from the original
 * @param {string} html - The HTML content
 * @param {string} componentName - Name of component
 * @returns {boolean} - True if component appears modified
 */
function isComponentModified(html, componentName) {
  const existingBoundaries = findComponentBoundaries(html, componentName);
  const originalComponent = loadComponentFromTemplate(componentName);

  if (!existingBoundaries || !originalComponent) {
    return true; // Assume modified if we can't compare
  }

  // Compare content (normalize whitespace)
  const normalizedExisting = existingBoundaries.content.replace(/\s+/g, ' ').trim();
  const normalizedOriginal = originalComponent.replace(/\s+/g, ' ').trim();

  return normalizedExisting !== normalizedOriginal;
}

export {
  findComponentBoundaries,
  findFunctionEnd,
  loadComponentFromTemplate,
  replaceComponent,
  isComponentModified,
  COMPONENT_MARKERS
};
