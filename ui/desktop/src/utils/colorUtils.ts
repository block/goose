/**
 * Color Utility Functions
 * 
 * Comprehensive color manipulation utilities for theme customization.
 * Uses chroma-js for color operations and conversions.
 */

import chroma from 'chroma-js';

/**
 * Parse a color string into a chroma color object
 * Supports hex, rgb, rgba, hsl, hsla formats
 */
export function parseColor(color: string): chroma.Color | null {
  try {
    return chroma(color);
  } catch (error) {
    console.error('Invalid color:', color, error);
    return null;
  }
}

/**
 * Check if a color string is valid
 */
export function isValidColor(color: string): boolean {
  try {
    chroma(color);
    return true;
  } catch {
    return false;
  }
}

/**
 * Convert color to hex format
 */
export function toHex(color: string): string {
  const parsed = parseColor(color);
  return parsed ? parsed.hex() : color;
}

/**
 * Convert color to RGB format
 */
export function toRgb(color: string): string {
  const parsed = parseColor(color);
  return parsed ? parsed.css() : color;
}

/**
 * Convert color to HSL format
 */
export function toHsl(color: string): string {
  const parsed = parseColor(color);
  return parsed ? parsed.css('hsl') : color;
}

/**
 * Get RGB components as object
 */
export function getRgbComponents(color: string): { r: number; g: number; b: number } | null {
  const parsed = parseColor(color);
  if (!parsed) return null;
  
  const [r, g, b] = parsed.rgb();
  return { r, g, b };
}

/**
 * Get HSL components as object
 */
export function getHslComponents(color: string): { h: number; s: number; l: number } | null {
  const parsed = parseColor(color);
  if (!parsed) return null;
  
  const [h, s, l] = parsed.hsl();
  return { 
    h: isNaN(h) ? 0 : h, // Handle achromatic colors
    s: isNaN(s) ? 0 : s,
    l: isNaN(l) ? 0 : l
  };
}

/**
 * Lighten a color by a percentage (0-100)
 */
export function lighten(color: string, amount: number): string {
  const parsed = parseColor(color);
  if (!parsed) return color;
  
  // Chroma uses 0-1 scale, convert from percentage
  return parsed.brighten(amount / 100).hex();
}

/**
 * Darken a color by a percentage (0-100)
 */
export function darken(color: string, amount: number): string {
  const parsed = parseColor(color);
  if (!parsed) return color;
  
  // Chroma uses 0-1 scale, convert from percentage
  return parsed.darken(amount / 100).hex();
}

/**
 * Increase saturation by a percentage (0-100)
 */
export function saturate(color: string, amount: number): string {
  const parsed = parseColor(color);
  if (!parsed) return color;
  
  return parsed.saturate(amount / 100).hex();
}

/**
 * Decrease saturation by a percentage (0-100)
 */
export function desaturate(color: string, amount: number): string {
  const parsed = parseColor(color);
  if (!parsed) return color;
  
  return parsed.desaturate(amount / 100).hex();
}

/**
 * Rotate hue by degrees (0-360)
 */
export function rotateHue(color: string, degrees: number): string {
  const parsed = parseColor(color);
  if (!parsed) return color;
  
  const hsl = getHslComponents(color);
  if (!hsl) return color;
  
  const newHue = (hsl.h + degrees) % 360;
  return chroma.hsl(newHue, hsl.s, hsl.l).hex();
}

/**
 * Adjust brightness by percentage (-100 to +100)
 * Negative values darken, positive values lighten
 */
export function adjustBrightness(color: string, amount: number): string {
  if (amount === 0) return color;
  return amount > 0 ? lighten(color, amount) : darken(color, Math.abs(amount));
}

/**
 * Adjust saturation by percentage (-100 to +100)
 * Negative values desaturate, positive values saturate
 */
export function adjustSaturation(color: string, amount: number): string {
  if (amount === 0) return color;
  return amount > 0 ? saturate(color, amount) : desaturate(color, Math.abs(amount));
}

/**
 * Get complementary color (opposite on color wheel)
 */
export function getComplementary(color: string): string {
  return rotateHue(color, 180);
}

/**
 * Get analogous colors (adjacent on color wheel)
 */
export function getAnalogous(color: string): [string, string, string] {
  return [
    rotateHue(color, -30),
    color,
    rotateHue(color, 30)
  ];
}

/**
 * Get triadic colors (evenly spaced on color wheel)
 */
export function getTriadic(color: string): [string, string, string] {
  return [
    color,
    rotateHue(color, 120),
    rotateHue(color, 240)
  ];
}

/**
 * Get tetradic colors (two complementary pairs)
 */
export function getTetradic(color: string): [string, string, string, string] {
  return [
    color,
    rotateHue(color, 90),
    rotateHue(color, 180),
    rotateHue(color, 270)
  ];
}

/**
 * Get split complementary colors
 */
export function getSplitComplementary(color: string): [string, string, string] {
  return [
    color,
    rotateHue(color, 150),
    rotateHue(color, 210)
  ];
}

/**
 * Generate monochromatic palette (same hue, different lightness)
 */
export function getMonochromaticPalette(color: string, count: number = 5): string[] {
  const parsed = parseColor(color);
  if (!parsed) return [color];
  
  const hsl = getHslComponents(color);
  if (!hsl) return [color];
  
  const palette: string[] = [];
  const step = 0.8 / (count - 1); // Range from 0.1 to 0.9 lightness
  
  for (let i = 0; i < count; i++) {
    const lightness = 0.1 + (step * i);
    palette.push(chroma.hsl(hsl.h, hsl.s, lightness).hex());
  }
  
  return palette;
}

/**
 * Mix two colors together
 */
export function mixColors(color1: string, color2: string, ratio: number = 0.5): string {
  const parsed1 = parseColor(color1);
  const parsed2 = parseColor(color2);
  
  if (!parsed1 || !parsed2) return color1;
  
  return chroma.mix(parsed1, parsed2, ratio).hex();
}

/**
 * Get color luminance (0-1)
 */
export function getLuminance(color: string): number {
  const parsed = parseColor(color);
  return parsed ? parsed.luminance() : 0;
}

/**
 * Check if color is light (luminance > 0.5)
 */
export function isLight(color: string): boolean {
  return getLuminance(color) > 0.5;
}

/**
 * Check if color is dark (luminance <= 0.5)
 */
export function isDark(color: string): boolean {
  return !isLight(color);
}

/**
 * Get appropriate text color (black or white) for a background
 */
export function getTextColorForBackground(backgroundColor: string): string {
  return isLight(backgroundColor) ? '#000000' : '#ffffff';
}

/**
 * Generate tints (lighter variations) of a color
 */
export function generateTints(color: string, count: number = 5): string[] {
  const tints: string[] = [];
  const step = 100 / count;
  
  for (let i = 0; i < count; i++) {
    tints.push(lighten(color, step * i));
  }
  
  return tints;
}

/**
 * Generate shades (darker variations) of a color
 */
export function generateShades(color: string, count: number = 5): string[] {
  const shades: string[] = [];
  const step = 100 / count;
  
  for (let i = 0; i < count; i++) {
    shades.push(darken(color, step * i));
  }
  
  return shades;
}

/**
 * Generate a complete color scale (tints + base + shades)
 */
export function generateColorScale(color: string, steps: number = 9): string[] {
  const halfSteps = Math.floor(steps / 2);
  const tints = generateTints(color, halfSteps).reverse();
  const shades = generateShades(color, halfSteps).slice(1);
  
  return [...tints, color, ...shades];
}

/**
 * Ensure color has minimum contrast with background
 * Adjusts lightness until minimum contrast is met
 */
export function ensureContrast(
  foreground: string,
  background: string,
  minContrast: number = 4.5
): string {
  const parsed = parseColor(foreground);
  if (!parsed) return foreground;
  
  let adjusted = foreground;
  let iterations = 0;
  const maxIterations = 20;
  
  // Import contrast function (will be defined in contrastUtils)
  // For now, we'll use a placeholder
  const getContrast = (fg: string, bg: string): number => {
    const fgLum = getLuminance(fg);
    const bgLum = getLuminance(bg);
    const lighter = Math.max(fgLum, bgLum);
    const darker = Math.min(fgLum, bgLum);
    return (lighter + 0.05) / (darker + 0.05);
  };
  
  while (getContrast(adjusted, background) < minContrast && iterations < maxIterations) {
    // If background is light, darken foreground; if dark, lighten foreground
    adjusted = isLight(background) 
      ? darken(adjusted, 5)
      : lighten(adjusted, 5);
    iterations++;
  }
  
  return adjusted;
}
