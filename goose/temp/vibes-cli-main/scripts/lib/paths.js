/**
 * Centralized Path Resolution
 *
 * All plugin paths defined in one place to avoid hardcoded paths scattered
 * throughout the codebase. Makes the plugin portable and easier to maintain.
 */

import { dirname, join } from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// Plugin root is two levels up from scripts/lib/
export const PLUGIN_ROOT = join(__dirname, '..', '..');
export const CACHE_DIR = join(PLUGIN_ROOT, 'cache');
export const CONFIG_DIR = join(PLUGIN_ROOT, 'config');

/**
 * Template file paths
 */
export const TEMPLATES = {
  vibesBasic: join(PLUGIN_ROOT, 'skills/vibes/templates/index.html'),
  sellUnified: join(PLUGIN_ROOT, 'skills/sell/templates/unified.html'),
  adminComponent: join(PLUGIN_ROOT, 'skills/sell/components/admin-exe.jsx'),
};

/**
 * Cache file paths
 */
export const CACHE_FILES = {
  importMap: join(CACHE_DIR, 'import-map.json'),
  stylePrompt: join(CACHE_DIR, 'style-prompt.txt'),
  fireproof: join(CACHE_DIR, 'fireproof.txt'),
  vibesMenu: join(CACHE_DIR, 'vibes-menu.js'),
  cssVariables: join(CACHE_DIR, 'vibes-variables.css'),
};

/**
 * Shipped cache paths (git-tracked defaults)
 */
export const SHIPPED_CACHE = {
  importMap: join(PLUGIN_ROOT, 'skills/vibes/cache/import-map.json'),
};

/**
 * Config file paths
 */
export const CONFIG_FILES = {
  sources: join(CONFIG_DIR, 'sources.json'),
};

/**
 * Skill file paths
 */
export const SKILL_FILES = {
  vibesSkill: join(PLUGIN_ROOT, 'skills/vibes/SKILL.md'),
};

/**
 * Convert absolute path to plugin-relative path for display
 * Works cross-platform (handles both / and \ separators)
 *
 * @param {string} absolutePath - Absolute file path
 * @returns {string} - Path relative to plugin root
 */
export function relativeToPlugin(absolutePath) {
  if (!absolutePath) return '';
  return absolutePath
    .replace(PLUGIN_ROOT, '')
    .replace(/^[/\\]/, '');
}
