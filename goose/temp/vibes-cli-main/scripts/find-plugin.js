#!/usr/bin/env node
/**
 * find-plugin.js - Find the vibes plugin directory with validation
 *
 * Usage:
 *   node scripts/find-plugin.js          # Prints plugin path or exits with error
 *   node scripts/find-plugin.js --quiet  # Only prints path, no error messages
 *
 * In bash:
 *   VIBES_DIR=$(node path/to/find-plugin.js)
 *
 * In other Node scripts:
 *   import { findPluginDir } from './find-plugin.js';
 *   const pluginDir = findPluginDir();
 */

import { existsSync, readdirSync } from 'fs';
import { join } from 'path';
import { homedir } from 'os';

/**
 * Find the vibes plugin directory
 * @param {object} options - Options
 * @param {boolean} options.quiet - Suppress error messages
 * @returns {string|null} - Plugin directory path (with trailing slash) or null
 */
export function findPluginDir(options = {}) {
  const { quiet = false } = options;
  const cacheBase = join(homedir(), '.claude', 'plugins', 'cache', 'vibes-cli', 'vibes');

  // Check if cache directory exists
  if (!existsSync(cacheBase)) {
    if (!quiet) {
      console.error('Error: Vibes plugin not found.');
      console.error('Please install with: /plugin install vibes@vibes-cli');
    }
    return null;
  }

  // Find all version directories and sort them
  let versions;
  try {
    versions = readdirSync(cacheBase)
      .filter(name => !name.startsWith('.'))
      .sort((a, b) => {
        // Version sort: split by dots and compare numerically
        const partsA = a.split('.').map(n => parseInt(n, 10) || 0);
        const partsB = b.split('.').map(n => parseInt(n, 10) || 0);
        for (let i = 0; i < Math.max(partsA.length, partsB.length); i++) {
          const diff = (partsA[i] || 0) - (partsB[i] || 0);
          if (diff !== 0) return diff;
        }
        return 0;
      });
  } catch (e) {
    if (!quiet) {
      console.error('Error: Could not read plugin cache directory.');
    }
    return null;
  }

  if (versions.length === 0) {
    if (!quiet) {
      console.error('Error: Vibes plugin not found.');
      console.error('Please install with: /plugin install vibes@vibes-cli');
    }
    return null;
  }

  // Use the latest version
  const latestVersion = versions[versions.length - 1];
  const pluginDir = join(cacheBase, latestVersion) + '/';

  // Verify key files exist
  const assembleScript = join(pluginDir, 'scripts', 'assemble.js');
  if (!existsSync(assembleScript)) {
    if (!quiet) {
      console.error('Error: Vibes plugin installation appears incomplete.');
      console.error(`Missing: ${assembleScript}`);
      console.error('Please reinstall with: /plugin install vibes@vibes-cli');
    }
    return null;
  }

  return pluginDir;
}

/**
 * Validate that a specific script exists in the plugin
 * @param {string} pluginDir - Plugin directory
 * @param {string} scriptName - Script name (e.g., 'assemble.js', 'generate-riff.js')
 * @returns {boolean} - True if script exists
 */
export function validateScript(pluginDir, scriptName) {
  const scriptPath = join(pluginDir, 'scripts', scriptName);
  return existsSync(scriptPath);
}

// CLI mode: print the path
if (process.argv[1] && process.argv[1].endsWith('find-plugin.js')) {
  const quiet = process.argv.includes('--quiet');
  const pluginDir = findPluginDir({ quiet });

  if (pluginDir) {
    console.log(pluginDir);
    process.exit(0);
  } else {
    process.exit(1);
  }
}
