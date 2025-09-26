import {
  useState,
  useEffect,
  useRef,
  useMemo,
  forwardRef,
  useImperativeHandle,
  useCallback,
} from 'react';
import { FileIcon } from './FileIcon';

interface FileItem {
  path: string;
  name: string;
  isDirectory: boolean;
  relativePath: string;
}

export interface FileItemWithMatch extends FileItem {
  matchScore: number;
  matches: number[];
  matchedText: string;
}

interface MentionPopoverProps {
  isOpen: boolean;
  onClose: () => void;
  onSelect: (filePath: string) => void;
  position: { x: number; y: number };
  query: string;
  selectedIndex: number;
  onSelectedIndexChange: (index: number) => void;
}

// Enhanced fuzzy matching algorithm
const fuzzyMatch = (pattern: string, text: string): { score: number; matches: number[] } => {
  if (!pattern) return { score: 0, matches: [] };

  const patternLower = pattern.toLowerCase();
  const textLower = text.toLowerCase();
  const matches: number[] = [];

  let patternIndex = 0;
  let score = 0;
  let consecutiveMatches = 0;

  for (let i = 0; i < textLower.length && patternIndex < patternLower.length; i++) {
    if (textLower[i] === patternLower[patternIndex]) {
      matches.push(i);
      patternIndex++;
      consecutiveMatches++;

      // Bonus for consecutive matches
      score += consecutiveMatches * 3;

      // Bonus for matches at word boundaries or path separators
      if (
        i === 0 ||
        textLower[i - 1] === '/' ||
        textLower[i - 1] === '_' ||
        textLower[i - 1] === '-' ||
        textLower[i - 1] === '.'
      ) {
        score += 10;
      }

      // Bonus for matching the start of the filename (after last /)
      const lastSlash = textLower.lastIndexOf('/', i);
      if (lastSlash !== -1 && i === lastSlash + 1) {
        score += 15;
      }
    } else {
      consecutiveMatches = 0;
    }
  }

  // Only return a score if all pattern characters were matched
  if (patternIndex === patternLower.length) {
    // Less penalty for longer strings to allow nested files to rank well
    score -= text.length * 0.05;

    // Bonus for exact substring matches
    if (textLower.includes(patternLower)) {
      score += 20;
    }

    // Bonus for matching the filename specifically (not just the path)
    const fileName = text.split('/').pop()?.toLowerCase() || '';
    if (fileName.includes(patternLower)) {
      score += 25;
    }

    return { score, matches };
  }

  return { score: -1, matches: [] };
};

const MentionPopover = forwardRef<
  { getDisplayFiles: () => FileItemWithMatch[]; selectFile: (index: number) => void },
  MentionPopoverProps
>(({ isOpen, onClose, onSelect, position, query, selectedIndex, onSelectedIndexChange }, ref) => {
  const [files, setFiles] = useState<FileItem[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const popoverRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const currentWorkingDir = window.appConfig.get('GOOSE_WORKING_DIR') as string;

  const compareByType = (a: FileItemWithMatch, b: FileItemWithMatch) =>
    a.isDirectory !== b.isDirectory ? (a.isDirectory ? 1 : -1) : 0;

  // Filter and sort files based on query
  const displayFiles = useMemo((): FileItemWithMatch[] => {
    if (!query.trim()) {
      return files
        .map((file) => ({
          ...file,
          matchScore: 0,
          matches: [],
          matchedText: file.name,
          depth: currentWorkingDir
            ? file.path.replace(currentWorkingDir, '').split('/').length - 1
            : 0,
        }))
        .sort((a, b) => {
          if (a.depth !== b.depth) return a.depth - b.depth;
          const typeComparison = compareByType(a, b);
          return typeComparison || a.name.localeCompare(b.name);
        });
    }

    const results = files
      .map((file) => {
        const matches = [
          { match: fuzzyMatch(query, file.name), text: file.name },
          { match: fuzzyMatch(query, file.relativePath), text: file.relativePath },
          { match: fuzzyMatch(query, file.path), text: file.path },
        ];

        const { match: bestMatch, text: matchedText } = matches.reduce((best, current) =>
          current.match.score > best.match.score ? current : best
        );

        let finalScore = bestMatch.score;
        if (finalScore > 0 && currentWorkingDir) {
          const depth = file.path.replace(currentWorkingDir, '').split('/').length - 1;
          finalScore += depth <= 1 ? 50 : depth <= 2 ? 30 : depth <= 3 ? 15 : 0;
        }

        return {
          ...file,
          matchScore: finalScore,
          matches: bestMatch.matches,
          matchedText,
        };
      })
      .filter((file) => file.matchScore > 0)
      .sort((a, b) => {
        // Sort by score first, then prefer files over directories, then alphabetically
        const scoreDiff = b.matchScore - a.matchScore;
        if (Math.abs(scoreDiff) >= 1) return scoreDiff;
        const typeComparison = compareByType(a, b);
        return typeComparison || a.name.localeCompare(b.name);
      });

    return results;
  }, [files, query, currentWorkingDir]);

  // Expose methods to parent component
  useImperativeHandle(
    ref,
    () => ({
      getDisplayFiles: () => displayFiles,
      selectFile: (index: number) => {
        if (displayFiles[index]) {
          onSelect(displayFiles[index].path);
          onClose();
        }
      },
    }),
    [displayFiles, onSelect, onClose]
  );

  // Scan files when component opens
  useEffect(() => {
    if (isOpen && files.length === 0) {
      scanFilesFromRoot();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOpen, files.length]); // scanFilesFromRoot intentionally omitted to avoid circular dependency

  // Handle clicks outside the popover
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (popoverRef.current && !popoverRef.current.contains(event.target as Node)) {
        onClose();
      }
    };

    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside);
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [isOpen, onClose]);

  const scanDirectoryFromRoot = useCallback(
    async (dirPath: string, relativePath = '', depth = 0): Promise<FileItem[]> => {
      // Increase depth limit for better file discovery
      if (depth > 5) return [];

      try {
        const items = await window.electron.listFiles(dirPath);
        const results: FileItem[] = [];

        // Common directories to prioritize or skip
        const priorityDirs = [
          'Desktop',
          'Documents',
          'Downloads',
          'Projects',
          'Development',
          'Code',
          'src',
          'components',
          'icons',
        ];
        const skipDirs = [
          '.git',
          '.svn',
          '.hg',
          'node_modules',
          '__pycache__',
          '.vscode',
          '.idea',
          'target',
          'dist',
          'build',
          '.cache',
          '.npm',
          '.yarn',
          'Library',
          'System',
          'Applications',
          '.Trash',
        ];

        // Don't skip as many directories at deeper levels to find more files
        const skipDirsAtDepth =
          depth > 2 ? ['.git', '.svn', '.hg', 'node_modules', '__pycache__'] : skipDirs;

        // Sort items to prioritize certain directories
        const sortedItems = items.sort((a, b) => {
          const aPriority = priorityDirs.includes(a);
          const bPriority = priorityDirs.includes(b);
          if (aPriority && !bPriority) return -1;
          if (!aPriority && bPriority) return 1;
          return a.localeCompare(b);
        });

        // Increase item limit per directory for better coverage
        const itemLimit = depth === 0 ? 50 : depth === 1 ? 40 : 30;

        for (const item of sortedItems.slice(0, itemLimit)) {
          const fullPath = `${dirPath}/${item}`;
          const itemRelativePath = relativePath ? `${relativePath}/${item}` : item;

          // Skip hidden files and common ignore patterns
          if (item.startsWith('.') || skipDirsAtDepth.includes(item)) {
            continue;
          }

          // First, check if this looks like a file based on extension
          const hasExtension = item.includes('.');
          const ext = item.split('.').pop()?.toLowerCase();
          const commonExtensions = [
            // Code files
            'txt',
            'md',
            'js',
            'ts',
            'jsx',
            'tsx',
            'py',
            'java',
            'cpp',
            'c',
            'h',
            'css',
            'html',
            'json',
            'xml',
            'yaml',
            'yml',
            'toml',
            'ini',
            'cfg',
            'sh',
            'bat',
            'ps1',
            'rb',
            'go',
            'rs',
            'php',
            'sql',
            'r',
            'scala',
            'swift',
            'kt',
            'dart',
            'vue',
            'svelte',
            'astro',
            'scss',
            'less',
            // Documentation
            'readme',
            'license',
            'changelog',
            'contributing',
            // Config files
            'gitignore',
            'dockerignore',
            'editorconfig',
            'prettierrc',
            'eslintrc',
            // Images and assets
            'png',
            'jpg',
            'jpeg',
            'gif',
            'svg',
            'ico',
            'webp',
            'bmp',
            'tiff',
            'tif',
            // Vector and design files
            'ai',
            'eps',
            'sketch',
            'fig',
            'xd',
            'psd',
            // Other common files
            'pdf',
            'doc',
            'docx',
            'xls',
            'xlsx',
            'ppt',
            'pptx',
          ];

          // If it has a known file extension, treat it as a file
          if (hasExtension && ext && commonExtensions.includes(ext)) {
            results.push({
              path: fullPath,
              name: item,
              isDirectory: false,
              relativePath: itemRelativePath,
            });
            continue;
          }

          // If it's a known file without extension (README, LICENSE, etc.)
          const knownFiles = [
            'readme',
            'license',
            'changelog',
            'contributing',
            'dockerfile',
            'makefile',
          ];
          if (!hasExtension && knownFiles.includes(item.toLowerCase())) {
            results.push({
              path: fullPath,
              name: item,
              isDirectory: false,
              relativePath: itemRelativePath,
            });
            continue;
          }

          // Otherwise, try to determine if it's a directory
          try {
            await window.electron.listFiles(fullPath);

            // It's a directory
            results.push({
              path: fullPath,
              name: item,
              isDirectory: true,
              relativePath: itemRelativePath,
            });

            // Recursively scan directories more aggressively
            if (depth < 4 || priorityDirs.includes(item)) {
              const subFiles = await scanDirectoryFromRoot(fullPath, itemRelativePath, depth + 1);
              results.push(...subFiles);
            }
          } catch {
            // If we can't list it and it doesn't have a known extension, skip it
            // This could be a file with an unknown extension or a permission issue
          }
        }

        return results;
      } catch (error) {
        console.error(`Error scanning directory ${dirPath}:`, error);
        return [];
      }
    },
    []
  );

  const scanFilesFromRoot = useCallback(async () => {
    setIsLoading(true);
    try {
      let startPath = currentWorkingDir;

      if (!startPath) {
        if (window.electron.platform === 'win32') {
          startPath = 'C:\\Users';
        } else if (window.electron.platform === 'linux') {
          startPath = '/home';
        } else {
          startPath = '/Users'; // Default to macOS
        }
      }

      const scannedFiles = await scanDirectoryFromRoot(startPath);
      setFiles(scannedFiles);
    } catch (error) {
      console.error('Error scanning files from root:', error);
      setFiles([]);
    } finally {
      setIsLoading(false);
    }
  }, [scanDirectoryFromRoot, currentWorkingDir]);

  // Scroll selected item into view
  useEffect(() => {
    if (listRef.current && selectedIndex >= 0 && selectedIndex < displayFiles.length) {
      const selectedElement = listRef.current.children[selectedIndex] as HTMLElement;
      if (selectedElement) {
        selectedElement.scrollIntoView({
          block: 'nearest',
          behavior: 'smooth',
        });
      }
    }
  }, [selectedIndex, displayFiles.length]);

  const handleItemClick = (index: number) => {
    if (index >= 0 && index < displayFiles.length) {
      onSelectedIndexChange(index);
      onSelect(displayFiles[index].path);
      onClose();
    }
  };

  if (!isOpen) return null;

  return (
    <div
      ref={popoverRef}
      className="fixed z-50 bg-background-default border border-borderStandard rounded-lg shadow-lg min-w-96 max-w-lg max-h-80"
      style={{
        left: position.x,
        top: position.y - 10, // Position above the chat input
        transform: 'translateY(-100%)', // Move it fully above
      }}
    >
      <div className="p-3 flex flex-col max-h-80">
        {isLoading ? (
          <div className="flex items-center justify-center py-4">
            <div className="animate-spin rounded-full h-4 w-4 border-t-2 border-b-2 border-textSubtle"></div>
            <span className="ml-2 text-sm text-textSubtle">Scanning files...</span>
          </div>
        ) : (
          <>
            {displayFiles.length > 0 && (
              <div className="text-xs text-textSubtle mb-2 px-1">
                {displayFiles.length} file{displayFiles.length !== 1 ? 's' : ''} found
              </div>
            )}
            <div
              ref={listRef}
              className="space-y-1 overflow-y-auto flex-1 scrollbar-thin scrollbar-thumb-borderStandard scrollbar-track-transparent"
              style={{ maxHeight: '280px' }}
            >
              {displayFiles.map((file, index) => (
                <div
                  key={file.path}
                  onClick={() => handleItemClick(index)}
                  className={`flex items-center gap-3 p-2 rounded-lg cursor-pointer transition-all ${
                    index === selectedIndex
                      ? 'bg-bgProminent text-textProminentInverse'
                      : 'hover:bg-background-muted'
                  }`}
                >
                  <div className="flex-shrink-0 text-textSubtle">
                    <FileIcon fileName={file.name} isDirectory={file.isDirectory} />
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="text-sm truncate text-textStandard">{file.name}</div>
                    <div className="text-xs text-textSubtle truncate">{file.path}</div>
                  </div>
                </div>
              ))}

              {!isLoading && displayFiles.length === 0 && query && (
                <div className="p-4 text-center text-textSubtle text-sm">
                  No files found matching "{query}"
                </div>
              )}

              {!isLoading && displayFiles.length === 0 && !query && (
                <div className="p-4 text-center text-textSubtle text-sm">
                  Start typing to search for files
                </div>
              )}
            </div>
          </>
        )}
      </div>
    </div>
  );
});

MentionPopover.displayName = 'MentionPopover';

export default MentionPopover;
