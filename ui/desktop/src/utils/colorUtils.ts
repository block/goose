/**
 * Color utility functions for custom theme generation using native CSS color-mix()
 */

export const DEFAULT_THEME_COLOR = '#32353b';

/**
 * Validate if a string is a valid hex color
 */
export function isValidHexColor(color: string): boolean {
  return /^#[0-9A-Fa-f]{6}$/.test(color);
}

/**
 * Apply custom theme - tints background neutrals more, text neutrals less
 */
export function applyCustomTheme(baseColor: string, isDarkMode: boolean): void {
  if (typeof document === 'undefined') return;

  const root = document.documentElement;

  if (isDarkMode) {
    // Dark mode: Tint the dark background colors (950-600) more heavily
    // Keep text colors (500-50) less tinted to maintain readability
    root.style.setProperty(
      '--color-neutral-950',
      `color-mix(in srgb, ${baseColor} 8%, #22252a 92%)`
    );
    root.style.setProperty(
      '--color-neutral-900',
      `color-mix(in srgb, ${baseColor} 8%, #32353b 92%)`
    );
    root.style.setProperty(
      '--color-neutral-800',
      `color-mix(in srgb, ${baseColor} 7%, #3f434b 93%)`
    );
    root.style.setProperty(
      '--color-neutral-700',
      `color-mix(in srgb, ${baseColor} 6%, #474e57 94%)`
    );
    root.style.setProperty(
      '--color-neutral-600',
      `color-mix(in srgb, ${baseColor} 5%, #525b68 95%)`
    );
    root.style.setProperty(
      '--color-neutral-500',
      `color-mix(in srgb, ${baseColor} 4%, #606c7a 96%)`
    );
    root.style.setProperty(
      '--color-neutral-400',
      `color-mix(in srgb, ${baseColor} 3%, #878787 97%)`
    );
    root.style.setProperty(
      '--color-neutral-300',
      `color-mix(in srgb, ${baseColor} 2%, #a7b0b9 98%)`
    );
    root.style.setProperty(
      '--color-neutral-200',
      `color-mix(in srgb, ${baseColor} 2%, #cbd1d6 98%)`
    );
    root.style.setProperty(
      '--color-neutral-100',
      `color-mix(in srgb, ${baseColor} 2%, #e3e6ea 98%)`
    );
    root.style.setProperty(
      '--color-neutral-50',
      `color-mix(in srgb, ${baseColor} 2%, #f4f6f7 98%)`
    );
  } else {
    // Light mode: Tint the light background colors (50-300) more heavily
    // Keep darker colors less tinted
    root.style.setProperty(
      '--color-neutral-50',
      `color-mix(in srgb, ${baseColor} 8%, #f4f6f7 92%)`
    );
    root.style.setProperty(
      '--color-neutral-100',
      `color-mix(in srgb, ${baseColor} 7%, #e3e6ea 93%)`
    );
    root.style.setProperty(
      '--color-neutral-200',
      `color-mix(in srgb, ${baseColor} 6%, #cbd1d6 94%)`
    );
    root.style.setProperty(
      '--color-neutral-300',
      `color-mix(in srgb, ${baseColor} 5%, #a7b0b9 95%)`
    );
    root.style.setProperty(
      '--color-neutral-400',
      `color-mix(in srgb, ${baseColor} 4%, #878787 96%)`
    );
    root.style.setProperty(
      '--color-neutral-500',
      `color-mix(in srgb, ${baseColor} 3%, #606c7a 97%)`
    );
    root.style.setProperty(
      '--color-neutral-600',
      `color-mix(in srgb, ${baseColor} 3%, #525b68 97%)`
    );
    root.style.setProperty(
      '--color-neutral-700',
      `color-mix(in srgb, ${baseColor} 2%, #474e57 98%)`
    );
    root.style.setProperty(
      '--color-neutral-800',
      `color-mix(in srgb, ${baseColor} 2%, #3f434b 98%)`
    );
    root.style.setProperty(
      '--color-neutral-900',
      `color-mix(in srgb, ${baseColor} 2%, #32353b 98%)`
    );
    root.style.setProperty(
      '--color-neutral-950',
      `color-mix(in srgb, ${baseColor} 2%, #22252a 98%)`
    );
  }

  // Apply accent color (lighter in dark mode for visibility)
  const accentColor = isDarkMode ? `color-mix(in srgb, ${baseColor} 70%, white 30%)` : baseColor;

  root.style.setProperty('--color-accent', accentColor);
}

/**
 * Reset theme colors to default
 */
export function resetThemeColors(): void {
  if (typeof document === 'undefined') return;

  const root = document.documentElement;
  const neutralKeys = ['50', '100', '200', '300', '400', '500', '600', '700', '800', '900', '950'];

  neutralKeys.forEach((key) => {
    root.style.removeProperty(`--color-neutral-${key}`);
  });

  root.style.removeProperty('--color-accent');
}
