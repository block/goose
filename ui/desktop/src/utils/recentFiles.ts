import { execSync } from 'child_process';
import fs from 'fs';
import path from 'path';
import os from 'os';

export type RecentItemType = 'image' | 'document' | 'repo';

export interface RecentItem {
  type: RecentItemType;
  name: string;
  path: string;
  fullPath: string;
  suggestion: string;
  secondarySuggestions: string[];
}

const IMAGE_EXTENSIONS = ['.png', '.jpg', '.jpeg', '.gif', '.svg', '.webp', '.heic'];
const DOC_EXTENSIONS = ['.md', '.doc', '.docx', '.pdf', '.txt', '.rtf'];

function getItemType(filePath: string): RecentItemType | null {
  const ext = path.extname(filePath).toLowerCase();
  if (IMAGE_EXTENSIONS.includes(ext)) return 'image';
  if (DOC_EXTENSIONS.includes(ext)) return 'document';
  return null;
}

function getSuggestions(type: RecentItemType): { primary: string; secondary: string[] } {
  switch (type) {
    case 'image':
      return {
        primary: 'Analyze this image',
        secondary: ['Describe what you see', 'Extract text (OCR)'],
      };
    case 'document':
      return {
        primary: 'Discuss this document',
        secondary: ['Summarize contents', 'Answer questions about it'],
      };
    case 'repo':
      return {
        primary: 'Fix a bug',
        secondary: ['Review recent changes', 'Add a feature', 'Write tests'],
      };
  }
}

function findGitRoot(filePath: string): string | null {
  let dir = path.dirname(filePath);
  const home = os.homedir();

  while (dir !== '/' && dir !== home) {
    const gitDir = path.join(dir, '.git');
    try {
      if (fs.existsSync(gitDir)) {
        return dir;
      }
    } catch {
      // Ignore permission errors
    }
    dir = path.dirname(dir);
  }
  return null;
}

function shortenPath(fullPath: string): string {
  const home = os.homedir();
  if (fullPath.startsWith(home)) {
    return '~' + fullPath.slice(home.length);
  }
  return fullPath;
}

export function loadRecentFiles(hoursAgo: number = 24): RecentItem[] {
  const home = os.homedir();
  const seconds = hoursAgo * 3600;

  // Directories to scan
  const scanDirs = ['Documents', 'Downloads', 'Desktop', 'Development']
    .map((d) => path.join(home, d))
    .filter((d) => {
      try {
        return fs.existsSync(d);
      } catch {
        return false;
      }
    });

  if (scanDirs.length === 0) {
    return [];
  }

  // Build mdfind command - search for recently modified files
  const onlyinArgs = scanDirs.map((d) => `-onlyin "${d}"`).join(' ');
  const cmd = `mdfind ${onlyinArgs} 'kMDItemFSContentChangeDate >= $time.now(-${seconds})' 2>/dev/null`;

  let output: string;
  try {
    output = execSync(cmd, { encoding: 'utf8', maxBuffer: 10 * 1024 * 1024 });
  } catch {
    return [];
  }

  const files = output
    .split('\n')
    .filter(Boolean)
    .filter((f) => !f.includes('node_modules'))
    .filter((f) => !f.includes('/target/'))
    .filter((f) => !f.includes('/.git/'));

  const seenRepos = new Set<string>();
  const seenFiles = new Set<string>();
  const items: RecentItem[] = [];

  for (const file of files) {
    const type = getItemType(file);

    if (type) {
      // It's a document or image
      if (seenFiles.has(file)) continue;
      seenFiles.add(file);

      const suggestions = getSuggestions(type);
      items.push({
        type,
        name: path.basename(file),
        path: shortenPath(path.dirname(file)),
        fullPath: file,
        suggestion: suggestions.primary,
        secondarySuggestions: suggestions.secondary,
      });
    } else {
      // Check if it's part of a git repo
      const gitRoot = findGitRoot(file);
      if (gitRoot && !seenRepos.has(gitRoot)) {
        seenRepos.add(gitRoot);
        const suggestions = getSuggestions('repo');
        items.push({
          type: 'repo',
          name: path.basename(gitRoot),
          path: shortenPath(path.dirname(gitRoot)),
          fullPath: gitRoot,
          suggestion: suggestions.primary,
          secondarySuggestions: suggestions.secondary,
        });
      }
    }

    // Limit results
    if (items.length >= 20) break;
  }

  // Sort: repos first, then by type
  items.sort((a, b) => {
    const typeOrder = { repo: 0, document: 1, image: 2 };
    return typeOrder[a.type] - typeOrder[b.type];
  });

  return items.slice(0, 10);
}
