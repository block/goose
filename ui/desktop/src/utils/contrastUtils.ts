/**
 * WCAG Contrast Utilities
 * 
 * Functions for checking color contrast ratios and WCAG compliance.
 * Implements WCAG 2.1 Level AA and AAA standards.
 */

import { getLuminance, parseColor, lighten, darken, isLight } from './colorUtils';

export interface ContrastResult {
  ratio: number;
  meetsAA: boolean;
  meetsAAA: boolean;
  meetsAALarge: boolean;
  meetsAAALarge: boolean;
  foreground: string;
  background: string;
  suggestion?: string;
}

/**
 * Calculate contrast ratio between two colors
 * Returns a value between 1 and 21
 * WCAG formula: (L1 + 0.05) / (L2 + 0.05)
 * where L1 is the lighter color and L2 is the darker
 */
export function getContrastRatio(foreground: string, background: string): number {
  const fgLuminance = getLuminance(foreground);
  const bgLuminance = getLuminance(background);
  
  const lighter = Math.max(fgLuminance, bgLuminance);
  const darker = Math.min(fgLuminance, bgLuminance);
  
  return (lighter + 0.05) / (darker + 0.05);
}

/**
 * Check if contrast meets WCAG 2.1 Level AA standards
 * Normal text: 4.5:1
 * Large text (18pt+ or 14pt+ bold): 3:1
 */
export function meetsWCAGAA(
  foreground: string,
  background: string,
  isLargeText: boolean = false
): boolean {
  const ratio = getContrastRatio(foreground, background);
  return isLargeText ? ratio >= 3 : ratio >= 4.5;
}

/**
 * Check if contrast meets WCAG 2.1 Level AAA standards
 * Normal text: 7:1
 * Large text: 4.5:1
 */
export function meetsWCAGAAA(
  foreground: string,
  background: string,
  isLargeText: boolean = false
): boolean {
  const ratio = getContrastRatio(foreground, background);
  return isLargeText ? ratio >= 4.5 : ratio >= 7;
}

/**
 * Get comprehensive contrast check result
 */
export function checkContrast(
  foreground: string,
  background: string
): ContrastResult {
  const ratio = getContrastRatio(foreground, background);
  
  return {
    ratio,
    meetsAA: ratio >= 4.5,
    meetsAAA: ratio >= 7,
    meetsAALarge: ratio >= 3,
    meetsAAALarge: ratio >= 4.5,
    foreground,
    background,
  };
}

/**
 * Suggest an accessible color that meets minimum contrast
 * Preserves hue while adjusting lightness
 */
export function suggestAccessibleColor(
  foreground: string,
  background: string,
  targetRatio: number = 4.5
): string {
  const parsed = parseColor(foreground);
  if (!parsed) return foreground;
  
  let adjusted = foreground;
  let currentRatio = getContrastRatio(adjusted, background);
  let iterations = 0;
  const maxIterations = 30;
  const step = 5; // Percentage to adjust each iteration
  
  // Determine direction: if background is light, darken foreground; if dark, lighten
  const shouldDarken = isLight(background);
  
  while (currentRatio < targetRatio && iterations < maxIterations) {
    adjusted = shouldDarken 
      ? darken(adjusted, step)
      : lighten(adjusted, step);
    
    currentRatio = getContrastRatio(adjusted, background);
    iterations++;
  }
  
  return adjusted;
}

/**
 * Get all contrast results for a color pair with suggestions
 */
export function getContrastWithSuggestion(
  foreground: string,
  background: string
): ContrastResult {
  const result = checkContrast(foreground, background);
  
  // If it doesn't meet AA, provide a suggestion
  if (!result.meetsAA) {
    result.suggestion = suggestAccessibleColor(foreground, background, 4.5);
  }
  
  return result;
}

/**
 * Check multiple color pairs and return results
 */
export function checkMultipleContrasts(
  pairs: Array<{ foreground: string; background: string; label?: string }>
): Array<ContrastResult & { label?: string }> {
  return pairs.map(({ foreground, background, label }) => ({
    ...getContrastWithSuggestion(foreground, background),
    label,
  }));
}

/**
 * Get WCAG level string for a contrast ratio
 */
export function getWCAGLevel(ratio: number, isLargeText: boolean = false): string {
  if (isLargeText) {
    if (ratio >= 4.5) return 'AAA';
    if (ratio >= 3) return 'AA';
    return 'Fail';
  }
  
  if (ratio >= 7) return 'AAA';
  if (ratio >= 4.5) return 'AA';
  return 'Fail';
}

/**
 * Get a color-coded status for contrast ratio
 */
export function getContrastStatus(ratio: number): 'pass' | 'warning' | 'fail' {
  if (ratio >= 4.5) return 'pass';
  if (ratio >= 3) return 'warning';
  return 'fail';
}

/**
 * Calculate accessibility score for a theme (0-100)
 * Based on how many color pairs meet WCAG AA standards
 */
export function calculateAccessibilityScore(
  colorPairs: Array<{ foreground: string; background: string }>
): {
  score: number;
  passing: number;
  failing: number;
  warnings: number;
  details: ContrastResult[];
} {
  const results = colorPairs.map(({ foreground, background }) =>
    checkContrast(foreground, background)
  );
  
  const passing = results.filter(r => r.meetsAA).length;
  const warnings = results.filter(r => !r.meetsAA && r.meetsAALarge).length;
  const failing = results.filter(r => !r.meetsAALarge).length;
  
  const score = Math.round((passing / results.length) * 100);
  
  return {
    score,
    passing,
    failing,
    warnings,
    details: results,
  };
}

/**
 * Get recommended minimum contrast ratios
 */
export const WCAG_STANDARDS = {
  AA: {
    normal: 4.5,
    large: 3,
  },
  AAA: {
    normal: 7,
    large: 4.5,
  },
} as const;

/**
 * Format contrast ratio for display
 */
export function formatContrastRatio(ratio: number): string {
  return `${ratio.toFixed(2)}:1`;
}

/**
 * Check if a color combination is safe for UI elements
 * Considers both text and interactive element requirements
 */
export function isSafeForUI(
  foreground: string,
  background: string,
  elementType: 'text' | 'interactive' | 'decorative' = 'text'
): boolean {
  const ratio = getContrastRatio(foreground, background);
  
  switch (elementType) {
    case 'text':
      return ratio >= 4.5;
    case 'interactive':
      return ratio >= 3; // WCAG 2.1 non-text contrast
    case 'decorative':
      return true; // No minimum requirement
    default:
      return ratio >= 4.5;
  }
}

/**
 * Batch check all text/background combinations in a theme
 */
export function validateThemeContrast(theme: {
  backgrounds: Record<string, string>;
  textColors: Record<string, string>;
}): {
  valid: boolean;
  issues: Array<{
    background: string;
    text: string;
    ratio: number;
    suggestion: string;
  }>;
} {
  const issues: Array<{
    background: string;
    text: string;
    ratio: number;
    suggestion: string;
  }> = [];
  
  // Check each text color against each background
  Object.entries(theme.backgrounds).forEach(([bgKey, bgValue]) => {
    Object.entries(theme.textColors).forEach(([textKey, textValue]) => {
      const result = getContrastWithSuggestion(textValue, bgValue);
      
      if (!result.meetsAA && result.suggestion) {
        issues.push({
          background: bgKey,
          text: textKey,
          ratio: result.ratio,
          suggestion: result.suggestion,
        });
      }
    });
  });
  
  return {
    valid: issues.length === 0,
    issues,
  };
}
