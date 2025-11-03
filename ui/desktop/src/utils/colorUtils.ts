/**
 * Color utility functions for generating theme palettes from a base color
 */

export const DEFAULT_THEME_COLOR = '#32353b';

interface RGB {
  r: number;
  g: number;
  b: number;
}

interface HSL {
  h: number;
  s: number;
  l: number;
}

/**
 * Convert hex color to RGB
 */
export function hexToRgb(hex: string): RGB | null {
  const result = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  return result
    ? {
        r: parseInt(result[1], 16),
        g: parseInt(result[2], 16),
        b: parseInt(result[3], 16),
      }
    : null;
}

/**
 * Convert RGB to hex
 */
export function rgbToHex(r: number, g: number, b: number): string {
  return '#' + [r, g, b].map((x) => x.toString(16).padStart(2, '0')).join('');
}

/**
 * Convert RGB to HSL
 */
export function rgbToHsl(r: number, g: number, b: number): HSL {
  r /= 255;
  g /= 255;
  b /= 255;

  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  let h = 0;
  let s = 0;
  const l = (max + min) / 2;

  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);

    switch (max) {
      case r:
        h = ((g - b) / d + (g < b ? 6 : 0)) / 6;
        break;
      case g:
        h = ((b - r) / d + 2) / 6;
        break;
      case b:
        h = ((r - g) / d + 4) / 6;
        break;
    }
  }

  return { h: h * 360, s: s * 100, l: l * 100 };
}

/**
 * Convert HSL to RGB
 */
export function hslToRgb(h: number, s: number, l: number): RGB {
  h /= 360;
  s /= 100;
  l /= 100;

  let r, g, b;

  if (s === 0) {
    r = g = b = l;
  } else {
    const hue2rgb = (p: number, q: number, t: number) => {
      if (t < 0) t += 1;
      if (t > 1) t -= 1;
      if (t < 1 / 6) return p + (q - p) * 6 * t;
      if (t < 1 / 2) return q;
      if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
      return p;
    };

    const q = l < 0.5 ? l * (1 + s) : l + s - l * s;
    const p = 2 * l - q;

    r = hue2rgb(p, q, h + 1 / 3);
    g = hue2rgb(p, q, h);
    b = hue2rgb(p, q, h - 1 / 3);
  }

  return {
    r: Math.round(r * 255),
    g: Math.round(g * 255),
    b: Math.round(b * 255),
  };
}

/**
 * Generate a neutral color scale from a base color
 */
export function generateNeutralScale(baseColor: string): Record<string, string> {
  const rgb = hexToRgb(baseColor);
  if (!rgb) return {};

  const hsl = rgbToHsl(rgb.r, rgb.g, rgb.b);

  // Generate neutral shades by reducing saturation and varying lightness
  const neutrals: Record<string, string> = {};
  const saturation = Math.min(hsl.s * 0.15, 10); // Very low saturation for neutrals

  const lightnesses = {
    50: 97,
    100: 93,
    200: 85,
    300: 70,
    400: 55,
    500: 45,
    600: 35,
    700: 28,
    800: 22,
    900: 16,
    950: 10,
  };

  Object.entries(lightnesses).forEach(([key, lightness]) => {
    const neutralRgb = hslToRgb(hsl.h, saturation, lightness);
    neutrals[key] = rgbToHex(neutralRgb.r, neutralRgb.g, neutralRgb.b);
  });

  return neutrals;
}

/**
 * Generate accent color variations
 */
export function generateAccentColors(baseColor: string): Record<string, string> {
  const rgb = hexToRgb(baseColor);
  if (!rgb) return {};

  const hsl = rgbToHsl(rgb.r, rgb.g, rgb.b);

  // Generate lighter and darker versions
  const colors: Record<string, string> = {
    base: baseColor,
  };

  // Light version (for dark mode)
  const lightRgb = hslToRgb(hsl.h, Math.min(hsl.s, 80), Math.min(hsl.l + 20, 85));
  colors.light = rgbToHex(lightRgb.r, lightRgb.g, lightRgb.b);

  // Dark version (for light mode)
  const darkRgb = hslToRgb(hsl.h, Math.min(hsl.s + 10, 90), Math.max(hsl.l - 15, 25));
  colors.dark = rgbToHex(darkRgb.r, darkRgb.g, darkRgb.b);

  return colors;
}

/**
 * Generate semantic colors (success, danger, warning, info) based on the base color
 */
export function generateSemanticColors(baseColor: string): Record<string, Record<string, string>> {
  const rgb = hexToRgb(baseColor);
  if (!rgb) return {};

  const hsl = rgbToHsl(rgb.r, rgb.g, rgb.b);

  const semanticColors: Record<string, Record<string, string>> = {
    success: {},
    danger: {},
    warning: {},
    info: {},
  };

  // Success (green-ish)
  const successRgb100 = hslToRgb(120, 45, 70);
  const successRgb200 = hslToRgb(120, 48, 65);
  semanticColors.success = {
    100: rgbToHex(successRgb100.r, successRgb100.g, successRgb100.b),
    200: rgbToHex(successRgb200.r, successRgb200.g, successRgb200.b),
  };

  // Danger (red-ish)
  const dangerRgb100 = hslToRgb(0, 100, 70);
  const dangerRgb200 = hslToRgb(0, 95, 62);
  semanticColors.danger = {
    100: rgbToHex(dangerRgb100.r, dangerRgb100.g, dangerRgb100.b),
    200: rgbToHex(dangerRgb200.r, dangerRgb200.g, dangerRgb200.b),
  };

  // Warning (yellow-ish)
  const warningRgb100 = hslToRgb(45, 100, 70);
  const warningRgb200 = hslToRgb(45, 98, 63);
  semanticColors.warning = {
    100: rgbToHex(warningRgb100.r, warningRgb100.g, warningRgb100.b),
    200: rgbToHex(warningRgb200.r, warningRgb200.g, warningRgb200.b),
  };

  // Info (blue-ish, or use base color if it's already blue)
  const isBlueish = hsl.h >= 180 && hsl.h <= 240;
  if (isBlueish) {
    const infoRgb100 = hslToRgb(hsl.h, 80, 75);
    const infoRgb200 = hslToRgb(hsl.h, 85, 65);
    semanticColors.info = {
      100: rgbToHex(infoRgb100.r, infoRgb100.g, infoRgb100.b),
      200: rgbToHex(infoRgb200.r, infoRgb200.g, infoRgb200.b),
    };
  } else {
    const infoRgb100 = hslToRgb(210, 80, 75);
    const infoRgb200 = hslToRgb(210, 85, 65);
    semanticColors.info = {
      100: rgbToHex(infoRgb100.r, infoRgb100.g, infoRgb100.b),
      200: rgbToHex(infoRgb200.r, infoRgb200.g, infoRgb200.b),
    };
  }

  return semanticColors;
}

/**
 * Generate a complete theme palette from a base color
 */
export interface ThemePalette {
  neutrals: Record<string, string>;
  accent: Record<string, string>;
  semantic: Record<string, Record<string, string>>;
}

export function generateThemePalette(baseColor: string): ThemePalette {
  return {
    neutrals: generateNeutralScale(baseColor),
    accent: generateAccentColors(baseColor),
    semantic: generateSemanticColors(baseColor),
  };
}

/**
 * Apply custom theme colors to CSS variables
 */
export function applyCustomTheme(baseColor: string, isDarkMode: boolean): void {
  const palette = generateThemePalette(baseColor);
  const root = document.documentElement;

  // Apply neutral colors
  Object.entries(palette.neutrals).forEach(([key, value]) => {
    root.style.setProperty(`--color-neutral-${key}`, value);
  });

  // Apply accent colors
  if (isDarkMode) {
    root.style.setProperty('--color-accent', palette.accent.light);
  } else {
    root.style.setProperty('--color-accent', palette.accent.dark);
  }

  // Apply semantic colors
  root.style.setProperty('--color-red-100', palette.semantic.danger['100']);
  root.style.setProperty('--color-red-200', palette.semantic.danger['200']);
  root.style.setProperty('--color-green-100', palette.semantic.success['100']);
  root.style.setProperty('--color-green-200', palette.semantic.success['200']);
  root.style.setProperty('--color-yellow-100', palette.semantic.warning['100']);
  root.style.setProperty('--color-yellow-200', palette.semantic.warning['200']);
  root.style.setProperty('--color-blue-100', palette.semantic.info['100']);
  root.style.setProperty('--color-blue-200', palette.semantic.info['200']);
}

/**
 * Reset theme colors to default
 */
export function resetThemeColors(): void {
  const root = document.documentElement;
  const properties = [
    '--color-neutral-50',
    '--color-neutral-100',
    '--color-neutral-200',
    '--color-neutral-300',
    '--color-neutral-400',
    '--color-neutral-500',
    '--color-neutral-600',
    '--color-neutral-700',
    '--color-neutral-800',
    '--color-neutral-900',
    '--color-neutral-950',
    '--color-accent',
    '--color-red-100',
    '--color-red-200',
    '--color-green-100',
    '--color-green-200',
    '--color-yellow-100',
    '--color-yellow-200',
    '--color-blue-100',
    '--color-blue-200',
  ];

  properties.forEach((prop) => {
    root.style.removeProperty(prop);
  });
}

/**
 * Validate if a string is a valid hex color
 */
export function isValidHexColor(color: string): boolean {
  return /^#[0-9A-F]{6}$/i.test(color);
}
