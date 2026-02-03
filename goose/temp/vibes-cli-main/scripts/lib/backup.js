/**
 * Shared Backup Utilities
 *
 * Consolidated backup logic for file operations.
 * Creates timestamped backups in format: file.YYYYMMDD-HHMMSS.bak.html
 */

import { existsSync, copyFileSync, readdirSync } from 'fs';
import { dirname, join, basename } from 'path';

/**
 * Generate timestamp string for backup filenames
 * Format: YYYYMMDD-HHMMSS
 */
function getBackupTimestamp() {
  const now = new Date();
  const pad = (n) => n.toString().padStart(2, '0');
  return `${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}-${pad(now.getHours())}${pad(now.getMinutes())}${pad(now.getSeconds())}`;
}

/**
 * Create a timestamped backup of an existing file
 * Creates: file.YYYYMMDD-HHMMSS.bak.html
 *
 * @param {string} filePath - Path to the file to backup
 * @returns {string|null} - Backup path if created, null if file doesn't exist
 */
export function createBackup(filePath) {
  if (!existsSync(filePath)) {
    return null;
  }
  const timestamp = getBackupTimestamp();
  const backupPath = filePath.replace(/\.html$/, `.${timestamp}.bak.html`);
  copyFileSync(filePath, backupPath);
  return backupPath;
}

/**
 * Find the most recent backup for a file
 *
 * @param {string} filePath - Path to the original file
 * @returns {string|null} - Path to most recent backup, or null if none found
 */
export function findLatestBackup(filePath) {
  const dir = dirname(filePath);
  const baseName = basename(filePath, '.html');
  const pattern = new RegExp(`^${baseName}\\.\\d{8}-\\d{6}\\.bak\\.html$`);

  try {
    const entries = readdirSync(dir);
    const backups = entries
      .filter(e => pattern.test(e))
      .sort()
      .reverse(); // Most recent first

    if (backups.length === 0) {
      // Fall back to legacy .bak.html format
      const legacyBackup = `${baseName}.bak.html`;
      if (entries.includes(legacyBackup)) {
        return join(dir, legacyBackup);
      }
      return null;
    }

    return join(dir, backups[0]);
  } catch (e) {
    return null;
  }
}

/**
 * Restore file from backup (uses most recent backup)
 *
 * @param {string} filePath - Path to the file to restore
 * @returns {object} - { success: boolean, backupPath?: string, error?: string }
 */
export function restoreFromBackup(filePath) {
  const backupPath = findLatestBackup(filePath);

  if (!backupPath) {
    return {
      success: false,
      error: `No backup file found for: ${filePath}`
    };
  }

  copyFileSync(backupPath, filePath);
  return {
    success: true,
    backupPath
  };
}
