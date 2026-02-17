/**
 * Theme Preset Types
 *
 * Presets define partial color overrides (keyed WITHOUT the `--` prefix,
 * e.g. `color-background-primary`). At apply-time these are merged on top
 * of the full goose-classic defaults from theme-tokens.ts.
 */

export interface ThemePreset {
  id: string;
  name: string;
  author: string;
  description: string;
  tags: string[];
  thumbnail?: string;
  colors: {
    light: Record<string, string>;
    dark: Record<string, string>;
  };
  version: string;
  isCustom?: boolean;
}

export type ThemeCategory =
  | 'dark'
  | 'light'
  | 'high-contrast'
  | 'colorful'
  | 'minimal'
  | 'retro'
  | 'modern';
