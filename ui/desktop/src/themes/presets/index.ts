/**
 * Theme Presets Registry
 * 
 * Central registry of all built-in theme presets
 */

import { ThemePreset } from './types';
import { gooseClassic } from './goose-classic';
import { nord } from './nord';
import { dracula } from './dracula';

/**
 * All available theme presets
 */
export const themePresets: ThemePreset[] = [
  gooseClassic,
  nord,
  dracula,
];

/**
 * Get a theme preset by ID
 */
export function getThemePreset(id: string): ThemePreset | undefined {
  return themePresets.find(preset => preset.id === id);
}

/**
 * Get theme presets by tag
 */
export function getThemePresetsByTag(tag: string): ThemePreset[] {
  return themePresets.filter(preset => preset.tags.includes(tag));
}

/**
 * Search theme presets by name or description
 */
export function searchThemePresets(query: string): ThemePreset[] {
  const lowerQuery = query.toLowerCase();
  return themePresets.filter(
    preset =>
      preset.name.toLowerCase().includes(lowerQuery) ||
      preset.description.toLowerCase().includes(lowerQuery) ||
      preset.tags.some(tag => tag.toLowerCase().includes(lowerQuery))
  );
}

/**
 * Get all available tags
 */
export function getAllTags(): string[] {
  const tags = new Set<string>();
  themePresets.forEach(preset => {
    preset.tags.forEach(tag => tags.add(tag));
  });
  return Array.from(tags).sort();
}

export * from './types';
export { gooseClassic, nord, dracula };
