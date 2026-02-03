/**
 * Shared dependency auto-installer
 *
 * Checks if required npm dependencies are installed and auto-installs them if missing.
 * Used by deployment scripts that may run from plugin cache without node_modules.
 */

import { execSync } from 'child_process';
import { dirname } from 'path';
import { fileURLToPath } from 'url';

/**
 * Ensure a dependency is installed, auto-installing if missing
 * @param {string} moduleName - The npm module to check (e.g., 'ssh2')
 * @param {string} [scriptPath] - Path to the calling script (for finding package.json)
 * @returns {Promise<void>}
 */
export async function ensureDependency(moduleName, scriptPath) {
  try {
    await import(moduleName);
  } catch (e) {
    if (e.code === 'ERR_MODULE_NOT_FOUND') {
      const scriptsDir = scriptPath
        ? dirname(scriptPath)
        : dirname(fileURLToPath(import.meta.url));

      console.log('Installing dependencies...');
      try {
        execSync('npm install', { cwd: scriptsDir, stdio: 'inherit' });
        console.log('Dependencies installed.\n');
      } catch (installErr) {
        console.error('Failed to install dependencies:', installErr.message);
        console.error('Try running: cd ' + scriptsDir + ' && npm install');
        process.exit(1);
      }
    } else {
      throw e;
    }
  }
}

/**
 * Ensure ssh2 is installed (common case for deployment scripts)
 * @param {string} [scriptPath] - Path to the calling script
 * @returns {Promise<void>}
 */
export async function ensureSSH2(scriptPath) {
  return ensureDependency('ssh2', scriptPath);
}
