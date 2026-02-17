/**
 * Theme Presets Registry
 *
 * Central registry of all built-in theme presets, plus helpers for
 * custom theme persistence via localStorage.
 */

import { ThemePreset } from './types';
import { gooseClassic } from './goose-classic';
import { nord } from './nord';
import { dracula } from './dracula';
import { solarized } from './solarized';
import { monokai } from './monokai';
import { github } from './github';
import { gruvbox } from './gruvbox';
import { tokyoNight } from './tokyo-night';
import { oneDark } from './one-dark';
import { highContrast } from './high-contrast';
import { lightTokens, darkTokens } from '../../theme/theme-tokens';
import type { McpUiStyleVariableKey } from '@modelcontextprotocol/ext-apps/app-bridge';

// ── Built-in presets ────────────────────────────────────────

export const builtInPresets: ThemePreset[] = [
  gooseClassic,
  highContrast,
  nord,
  dracula,
  solarized,
  monokai,
  github,
  gruvbox,
  tokyoNight,
  oneDark,
];

// ── Lookup helpers ──────────────────────────────────────────

export function getAllPresets(): ThemePreset[] {
  return [...builtInPresets, ...loadCustomThemes()];
}

export function getThemePreset(id: string): ThemePreset | undefined {
  return getAllPresets().find((preset) => preset.id === id);
}

export function getThemePresetsByTag(tag: string): ThemePreset[] {
  return getAllPresets().filter((preset) => preset.tags.includes(tag));
}

export function getAllTags(): string[] {
  const tags = new Set<string>();
  getAllPresets().forEach((preset) => {
    preset.tags.forEach((tag) => tags.add(tag));
  });
  return Array.from(tags).sort();
}

// ── Merge preset → full token map ──────────────────────────
//
// Presets only define a subset of color keys (without `--` prefix).
// This merges them on top of the goose-classic defaults so every
// McpUiStyleVariableKey has a value.

export function resolvePresetTokens(
  preset: ThemePreset,
  mode: 'light' | 'dark'
): Record<McpUiStyleVariableKey, string> {
  const defaults = mode === 'dark' ? { ...darkTokens } : { ...lightTokens };
  const overrides = mode === 'light' ? preset.colors.light : preset.colors.dark;

  for (const [key, value] of Object.entries(overrides)) {
    // Preset keys are stored without `--` prefix; token keys have it
    const tokenKey = key.startsWith('--') ? key : `--${key}`;
    if (tokenKey in defaults) {
      (defaults as Record<string, string>)[tokenKey] = value;
    }
  }

  return defaults;
}

// ── Active theme persistence (localStorage) ─────────────────

const ACTIVE_THEME_KEY = 'goose-active-theme-id';

export function getActiveThemeId(): string | null {
  return localStorage.getItem(ACTIVE_THEME_KEY);
}

export function setActiveThemeId(id: string | null): void {
  if (id) {
    localStorage.setItem(ACTIVE_THEME_KEY, id);
  } else {
    localStorage.removeItem(ACTIVE_THEME_KEY);
  }
}

// ── Custom theme persistence (localStorage) ─────────────────

const CUSTOM_THEMES_KEY = 'goose-custom-themes';

export function loadCustomThemes(): ThemePreset[] {
  try {
    const stored = localStorage.getItem(CUSTOM_THEMES_KEY);
    if (!stored) return [];
    const themes: ThemePreset[] = JSON.parse(stored);
    return themes.map((t) => ({ ...t, isCustom: true }));
  } catch {
    return [];
  }
}

export function saveCustomTheme(theme: ThemePreset): void {
  const existing = loadCustomThemes();
  const idx = existing.findIndex((t) => t.id === theme.id);
  const updated = idx >= 0 ? existing.map((t, i) => (i === idx ? { ...theme, isCustom: true } : t)) : [...existing, { ...theme, isCustom: true }];
  localStorage.setItem(CUSTOM_THEMES_KEY, JSON.stringify(updated));
}

export function deleteCustomTheme(id: string): void {
  const existing = loadCustomThemes();
  localStorage.setItem(
    CUSTOM_THEMES_KEY,
    JSON.stringify(existing.filter((t) => t.id !== id))
  );
}

// ── Re-exports ──────────────────────────────────────────────

export * from './types';
export {
  gooseClassic,
  highContrast,
  nord,
  dracula,
  solarized,
  monokai,
  github,
  gruvbox,
  tokyoNight,
  oneDark,
};
