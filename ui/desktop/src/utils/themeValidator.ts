/**
 * Theme Validator
 * 
 * Validates theme structure and ensures all required colors are present.
 */

import { isValidColor } from './colorUtils';
import { validateThemeContrast } from './contrastUtils';

export interface ThemeColors {
  light: Record<string, string>;
  dark: Record<string, string>;
}

export interface ValidationError {
  field: string;
  message: string;
  severity: 'error' | 'warning';
}

export interface ValidationResult {
  valid: boolean;
  errors: ValidationError[];
  warnings: ValidationError[];
}

/**
 * Required MCP color variable names
 */
export const REQUIRED_COLOR_VARIABLES = [
  // Backgrounds
  'color-background-primary',
  'color-background-secondary',
  'color-background-tertiary',
  'color-background-inverse',
  
  // Borders
  'color-border-primary',
  'color-border-secondary',
  
  // Text
  'color-text-primary',
  'color-text-secondary',
  'color-text-inverse',
  
  // Ring
  'color-ring-primary',
] as const;

/**
 * Optional semantic color variables
 */
export const OPTIONAL_COLOR_VARIABLES = [
  'color-background-danger',
  'color-background-info',
  'color-border-danger',
  'color-border-info',
  'color-text-danger',
  'color-text-success',
  'color-text-warning',
  'color-text-info',
] as const;

/**
 * Validate theme structure
 */
export function validateTheme(theme: ThemeColors): ValidationResult {
  const errors: ValidationError[] = [];
  const warnings: ValidationError[] = [];
  
  // Check if light and dark modes exist
  if (!theme.light) {
    errors.push({
      field: 'light',
      message: 'Light mode colors are required',
      severity: 'error',
    });
  }
  
  if (!theme.dark) {
    errors.push({
      field: 'dark',
      message: 'Dark mode colors are required',
      severity: 'error',
    });
  }
  
  // Validate light mode
  if (theme.light) {
    const lightErrors = validateColorSet(theme.light, 'light');
    errors.push(...lightErrors.filter(e => e.severity === 'error'));
    warnings.push(...lightErrors.filter(e => e.severity === 'warning'));
  }
  
  // Validate dark mode
  if (theme.dark) {
    const darkErrors = validateColorSet(theme.dark, 'dark');
    errors.push(...darkErrors.filter(e => e.severity === 'error'));
    warnings.push(...darkErrors.filter(e => e.severity === 'warning'));
  }
  
  return {
    valid: errors.length === 0,
    errors,
    warnings,
  };
}

/**
 * Validate a set of colors (light or dark mode)
 */
function validateColorSet(
  colors: Record<string, string>,
  mode: 'light' | 'dark'
): ValidationError[] {
  const errors: ValidationError[] = [];
  
  // Check for required variables
  REQUIRED_COLOR_VARIABLES.forEach(variable => {
    if (!colors[variable]) {
      errors.push({
        field: `${mode}.${variable}`,
        message: `Required color variable "${variable}" is missing`,
        severity: 'error',
      });
    } else if (!isValidColor(colors[variable])) {
      errors.push({
        field: `${mode}.${variable}`,
        message: `Invalid color value for "${variable}": ${colors[variable]}`,
        severity: 'error',
      });
    }
  });
  
  // Check optional variables if present
  OPTIONAL_COLOR_VARIABLES.forEach(variable => {
    if (colors[variable] && !isValidColor(colors[variable])) {
      errors.push({
        field: `${mode}.${variable}`,
        message: `Invalid color value for "${variable}": ${colors[variable]}`,
        severity: 'error',
      });
    }
  });
  
  // Warn about missing optional variables
  OPTIONAL_COLOR_VARIABLES.forEach(variable => {
    if (!colors[variable]) {
      errors.push({
        field: `${mode}.${variable}`,
        message: `Optional color variable "${variable}" is missing`,
        severity: 'warning',
      });
    }
  });
  
  return errors;
}

/**
 * Validate color pairs for contrast
 */
export function validateColorPairs(theme: ThemeColors): ValidationResult {
  const errors: ValidationError[] = [];
  const warnings: ValidationError[] = [];
  
  // Extract backgrounds and text colors for validation
  const lightBackgrounds: Record<string, string> = {};
  const lightTextColors: Record<string, string> = {};
  const darkBackgrounds: Record<string, string> = {};
  const darkTextColors: Record<string, string> = {};
  
  Object.entries(theme.light).forEach(([key, value]) => {
    if (key.startsWith('color-background-')) {
      lightBackgrounds[key] = value;
    } else if (key.startsWith('color-text-')) {
      lightTextColors[key] = value;
    }
  });
  
  Object.entries(theme.dark).forEach(([key, value]) => {
    if (key.startsWith('color-background-')) {
      darkBackgrounds[key] = value;
    } else if (key.startsWith('color-text-')) {
      darkTextColors[key] = value;
    }
  });
  
  // Validate light mode contrast
  const lightResult = validateThemeContrast({
    backgrounds: lightBackgrounds,
    textColors: lightTextColors,
  });
  
  lightResult.issues.forEach(issue => {
    warnings.push({
      field: `light.${issue.background}-${issue.text}`,
      message: `Low contrast (${issue.ratio.toFixed(2)}:1) between ${issue.text} and ${issue.background}. Suggested: ${issue.suggestion}`,
      severity: 'warning',
    });
  });
  
  // Validate dark mode contrast
  const darkResult = validateThemeContrast({
    backgrounds: darkBackgrounds,
    textColors: darkTextColors,
  });
  
  darkResult.issues.forEach(issue => {
    warnings.push({
      field: `dark.${issue.background}-${issue.text}`,
      message: `Low contrast (${issue.ratio.toFixed(2)}:1) between ${issue.text} and ${issue.background}. Suggested: ${issue.suggestion}`,
      severity: 'warning',
    });
  });
  
  return {
    valid: errors.length === 0,
    errors,
    warnings,
  };
}

/**
 * Get overall theme accessibility score
 */
export function getAccessibilityScore(theme: ThemeColors): {
  score: number;
  grade: 'A' | 'B' | 'C' | 'D' | 'F';
  issues: number;
} {
  const validation = validateColorPairs(theme);
  const issues = validation.warnings.length;
  
  // Calculate score based on number of issues
  // Assuming ~20 color pair checks, score decreases by 5 per issue
  const maxIssues = 20;
  const score = Math.max(0, 100 - (issues * 5));
  
  let grade: 'A' | 'B' | 'C' | 'D' | 'F';
  if (score >= 90) grade = 'A';
  else if (score >= 80) grade = 'B';
  else if (score >= 70) grade = 'C';
  else if (score >= 60) grade = 'D';
  else grade = 'F';
  
  return {
    score,
    grade,
    issues,
  };
}

/**
 * Check if theme is complete (has all required variables)
 */
export function isThemeComplete(theme: ThemeColors): boolean {
  const validation = validateTheme(theme);
  return validation.valid;
}

/**
 * Get missing required variables
 */
export function getMissingVariables(theme: ThemeColors): {
  light: string[];
  dark: string[];
} {
  const missingLight: string[] = [];
  const missingDark: string[] = [];
  
  REQUIRED_COLOR_VARIABLES.forEach(variable => {
    if (!theme.light[variable]) {
      missingLight.push(variable);
    }
    if (!theme.dark[variable]) {
      missingDark.push(variable);
    }
  });
  
  return {
    light: missingLight,
    dark: missingDark,
  };
}

/**
 * Sanitize theme by removing invalid colors
 */
export function sanitizeTheme(theme: ThemeColors): ThemeColors {
  const sanitized: ThemeColors = {
    light: {},
    dark: {},
  };
  
  // Filter out invalid colors
  Object.entries(theme.light).forEach(([key, value]) => {
    if (isValidColor(value)) {
      sanitized.light[key] = value;
    }
  });
  
  Object.entries(theme.dark).forEach(([key, value]) => {
    if (isValidColor(value)) {
      sanitized.dark[key] = value;
    }
  });
  
  return sanitized;
}
