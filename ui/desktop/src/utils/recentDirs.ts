import fs from 'fs';
import path from 'path';
import { app } from 'electron';

const RECENT_DIRS_FILE = path.join(app.getPath('userData'), 'recent-dirs.json');
const MAX_RECENT_DIRS = 10;

interface RecentDirs {
  dirs: string[];
}

export function loadRecentDirs(): string[] {
  try {
    if (fs.existsSync(RECENT_DIRS_FILE)) {
      const data = fs.readFileSync(RECENT_DIRS_FILE, 'utf8');
      const recentDirs: RecentDirs = JSON.parse(data);
      return recentDirs.dirs;
    }
  } catch (error) {
    console.error('Error loading recent directories:', error);
  }
  return [];
}

export function addRecentDir(dir: string): void {
  try {
    let dirs = loadRecentDirs();
    // Remove the directory if it already exists
    dirs = dirs.filter((d) => d !== dir);
    // Add the new directory at the beginning
    dirs.unshift(dir);
    // Keep only the most recent MAX_RECENT_DIRS
    dirs = dirs.slice(0, MAX_RECENT_DIRS);

    fs.writeFileSync(RECENT_DIRS_FILE, JSON.stringify({ dirs }, null, 2));
  } catch (error) {
    console.error('Error saving recent directory:', error);
  }
}
