/**
 * Theme Preset Types
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
}

export type ThemeCategory = 'dark' | 'light' | 'high-contrast' | 'colorful' | 'minimal' | 'retro' | 'modern';
