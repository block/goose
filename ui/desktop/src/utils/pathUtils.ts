import path from 'node:path';
import os from 'node:os';

/**
 * Expands tilde (~) to the user's home directory
 * @param filePath - The file path that may contain tilde
 * @returns The expanded path with tilde replaced by home directory
 */
export function expandTilde(filePath: string): string {
  if (!filePath || typeof filePath !== 'string') return filePath;
  // Support "~", "~/..." and "~\\..." on Windows
  if (filePath === '~') {
    return os.homedir();
  }
  if (filePath.startsWith('~/') || (process.platform === 'win32' && filePath.startsWith('~\\'))) {
    // Remove the leading "~" and any separator that follows, then join
    const remainder = filePath.slice(2);
    return path.join(os.homedir(), remainder);
  }
  if (filePath.startsWith('~')) {
    // Generic fallback: replace only the first leading tilde
    return path.join(os.homedir(), filePath.slice(1));
  }
  return filePath;
}
