export const THEME_IDS = [
  'light',
  'dark',
  'aura',
  'catppuccin-latte',
  'catppuccin-frappe',
  'catppuccin-macchiato',
  'catppuccin-mocha',
] as const;

export type ThemeId = (typeof THEME_IDS)[number];

export const CATPPUCCIN_THEME_IDS = [
  'catppuccin-latte',
  'catppuccin-frappe',
  'catppuccin-macchiato',
  'catppuccin-mocha',
] as const;

export type CatppuccinThemeId = (typeof CATPPUCCIN_THEME_IDS)[number];

export const CATPPUCCIN_ACCENTS = [
  'rosewater',
  'flamingo',
  'pink',
  'mauve',
  'red',
  'maroon',
  'peach',
  'yellow',
  'green',
  'teal',
  'sky',
  'sapphire',
  'blue',
  'lavender',
] as const;

export type CatppuccinAccent = (typeof CATPPUCCIN_ACCENTS)[number];

export const DEFAULT_CATPPUCCIN_ACCENT: CatppuccinAccent = 'mauve';

export type ThemePreference = ThemeId | 'system';
export type ThemeVariant = 'light' | 'dark';

export function isThemeId(value: unknown): value is ThemeId {
  return typeof value === 'string' && (THEME_IDS as readonly string[]).includes(value);
}

export function isCatppuccinThemeId(value: unknown): value is CatppuccinThemeId {
  return (
    typeof value === 'string' && (CATPPUCCIN_THEME_IDS as readonly string[]).includes(value)
  );
}

export function isCatppuccinAccent(value: unknown): value is CatppuccinAccent {
  return typeof value === 'string' && (CATPPUCCIN_ACCENTS as readonly string[]).includes(value);
}

export function isThemePreference(value: unknown): value is ThemePreference {
  return value === 'system' || isThemeId(value);
}
