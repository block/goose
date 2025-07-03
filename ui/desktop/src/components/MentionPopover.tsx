import React, { useState, useEffect, useRef, useMemo } from 'react';

interface FileItem {
  path: string;
  name: string;
  isDirectory: boolean;
  relativePath: string;
}

interface FileItemWithMatch extends FileItem {
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
}

// Simple fuzzy matching algorithm
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
      score += consecutiveMatches * 2;
      
      // Bonus for matches at word boundaries
      if (i === 0 || textLower[i - 1] === '/' || textLower[i - 1] === '_' || textLower[i - 1] === '-') {
        score += 5;
      }
    } else {
      consecutiveMatches = 0;
    }
  }
  
  // Only return a score if all pattern characters were matched
  if (patternIndex === patternLower.length) {
    // Penalty for longer strings
    score -= text.length * 0.1;
    return { score, matches };
  }
  
  return { score: -1, matches: [] };
};

// Get file icon based on file type
const getFileIcon = (fileName: string, isDirectory: boolean): string => {
  if (isDirectory) {
    return '📁';
  }
  
  const ext = fileName.split('.').pop()?.toLowerCase();
  
  switch (ext) {
    case 'js':
    case 'jsx':
    case 'ts':
    case 'tsx':
      return '📄';
    case 'py':
      return '🐍';
    case 'java':
      return '☕';
    case 'cpp':
    case 'c':
    case 'h':
      return '⚙️';
    case 'html':
    case 'htm':
      return '🌐';
    case 'css':
      return '🎨';
    case 'json':
      return '📋';
    case 'md':
    case 'markdown':
      return '📝';
    case 'txt':
      return '📄';
    case 'pdf':
      return '📕';
    case 'png':
    case 'jpg':
    case 'jpeg':
    case 'gif':
    case 'svg':
      return '🖼️';
    case 'mp4':
    case 'mov':
    case 'avi':
      return '🎬';
    case 'mp3':
    case 'wav':
    case 'flac':
      return '🎵';
    case 'zip':
    case 'tar':
    case 'gz':
      return '📦';
    default:
      return '📄';
  }
};

export default function MentionPopover({ 
  isOpen, 
  onClose, 
  onSelect, 
  position, 
  query
}: MentionPopoverProps) {
  const [files, setFiles] = useState<FileItem[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const popoverRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  // Scan files when component opens
  useEffect(() => {
    if (isOpen) {
      scanFilesFromRoot();
    }
  }, [isOpen]);

  // Focus the popover when it opens to enable keyboard navigation
  useEffect(() => {
    if (isOpen && popoverRef.current) {
      popoverRef.current.focus();
    }
  }, [isOpen]);

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

  const scanFilesFromRoot = async () => {
    setIsLoading(true);
    try {
      // Start from common user directories for better performance
      let startPath = '/Users'; // Default to macOS
      if (window.electron.platform === 'win32') {
        startPath = 'C:\\Users';
      } else if (window.electron.platform === 'linux') {
        startPath = '/home';
      }
      
      const scannedFiles = await scanDirectoryFromRoot(startPath);
      setFiles(scannedFiles);
    } catch (error) {
      console.error('Error scanning files from root:', error);
      setFiles([]);
    } finally {
      setIsLoading(false);
    }
  };

  const scanDirectoryFromRoot = async (dirPath: string, relativePath = '', depth = 0): Promise<FileItem[]> => {
    // Limit depth for performance when searching from root
    if (depth > 3) return [];
    
    try {
      const items = await window.electron.listFiles(dirPath);
      const results: FileItem[] = [];
      
      // Common directories to prioritize or skip
      const priorityDirs = ['Desktop', 'Documents', 'Downloads', 'Projects', 'Development', 'Code'];
      const skipDirs = [
        '.git', '.svn', '.hg', 'node_modules', '__pycache__', '.vscode', '.idea',
        'target', 'dist', 'build', '.cache', '.npm', '.yarn', 'Library', 
        'System', 'Applications', '.Trash', 'Music', 'Movies', 'Pictures'
      ];
      
      // Sort items to prioritize certain directories
      const sortedItems = items.sort((a, b) => {
        const aPriority = priorityDirs.includes(a);
        const bPriority = priorityDirs.includes(b);
        if (aPriority && !bPriority) return -1;
        if (!aPriority && bPriority) return 1;
        return a.localeCompare(b);
      });
      
      for (const item of sortedItems.slice(0, 30)) { // Limit items per directory
        const fullPath = `${dirPath}/${item}`;
        const itemRelativePath = relativePath ? `${relativePath}/${item}` : item;
        
        // Skip hidden files and common ignore patterns
        if (item.startsWith('.') || skipDirs.includes(item)) {
          continue;
        }
        
        try {
          // Check if it's a directory by trying to list its contents
          await window.electron.listFiles(fullPath);
          
          // It's a directory
          results.push({
            path: fullPath,
            name: item,
            isDirectory: true,
            relativePath: itemRelativePath
          });
          
          // Recursively scan important directories
          if (depth < 2 && (priorityDirs.includes(item) || depth === 0)) {
            const subFiles = await scanDirectoryFromRoot(fullPath, itemRelativePath, depth + 1);
            results.push(...subFiles);
          }
        } catch {
          // It's a file - only include common file types
          const ext = item.split('.').pop()?.toLowerCase();
          const commonExtensions = [
            'txt', 'md', 'js', 'ts', 'jsx', 'tsx', 'py', 'java', 'cpp', 'c', 'h',
            'css', 'html', 'json', 'xml', 'yaml', 'yml', 'toml', 'ini', 'cfg',
            'sh', 'bat', 'ps1', 'rb', 'go', 'rs', 'php', 'sql', 'r', 'scala',
            'swift', 'kt', 'dart', 'vue', 'svelte', 'astro'
          ];
          
          if (ext && commonExtensions.includes(ext)) {
            results.push({
              path: fullPath,
              name: item,
              isDirectory: false,
              relativePath: itemRelativePath
            });
          }
        }
      }
      
      return results;
    } catch (error) {
      console.error(`Error scanning directory ${dirPath}:`, error);
      return [];
    }
  };

  // Filter and sort files based on query
  const filteredFiles = useMemo((): FileItemWithMatch[] => {
    if (!query.trim()) {
      return files.slice(0, 10).map(file => ({
        ...file,
        matchScore: 0,
        matches: [],
        matchedText: file.name
      })); // Show first 10 files when no query
    }
    
    const results = files
      .map(file => {
        const nameMatch = fuzzyMatch(query, file.name);
        const pathMatch = fuzzyMatch(query, file.relativePath);
        
        // Use the better of the two matches
        const bestMatch = nameMatch.score > pathMatch.score ? nameMatch : pathMatch;
        
        return {
          ...file,
          matchScore: bestMatch.score,
          matches: bestMatch.matches,
          matchedText: nameMatch.score > pathMatch.score ? file.name : file.relativePath
        };
      })
      .filter(file => file.matchScore > 0)
      .sort((a, b) => b.matchScore - a.matchScore)
      .slice(0, 10); // Limit to 10 results
    
    return results;
  }, [files, query]);

  // Update selected index when filtered results change
  useEffect(() => {
    setSelectedIndex(0);
  }, [filteredFiles]);

  // Scroll selected item into view
  useEffect(() => {
    if (listRef.current) {
      const selectedElement = listRef.current.children[selectedIndex] as HTMLElement;
      if (selectedElement) {
        selectedElement.scrollIntoView({ block: 'nearest' });
      }
    }
  }, [selectedIndex]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    switch (e.key) {
      case 'Escape':
        e.preventDefault();
        onClose();
        break;
      case 'ArrowDown':
        e.preventDefault();
        setSelectedIndex(prev => Math.min(prev + 1, filteredFiles.length - 1));
        break;
      case 'ArrowUp':
        e.preventDefault();
        setSelectedIndex(prev => Math.max(prev - 1, 0));
        break;
      case 'Enter':
        e.preventDefault();
        if (filteredFiles[selectedIndex]) {
          onSelect(filteredFiles[selectedIndex].path);
          onClose();
        }
        break;
    }
  };

  const handleItemClick = (index: number) => {
    setSelectedIndex(index);
    onSelect(filteredFiles[index].path);
    onClose();
  };

  if (!isOpen) return null;

  const displayedFiles = filteredFiles.slice(0, 5);
  const remainingCount = filteredFiles.length - displayedFiles.length;

  return (
    <div
      ref={popoverRef}
      className="fixed z-50 bg-bgApp border border-borderStandard rounded-lg shadow-lg min-w-96 max-w-lg"
      style={{
        left: position.x,
        top: position.y - 10, // Position above the chat input
        transform: 'translateY(-100%)', // Move it fully above
      }}
      onKeyDown={handleKeyDown}
      tabIndex={-1}
    >
      <div className="p-3">
        {isLoading ? (
          <div className="flex items-center justify-center py-4">
            <div className="animate-spin rounded-full h-4 w-4 border-t-2 border-b-2 border-textSubtle"></div>
            <span className="ml-2 text-sm text-textSubtle">Scanning files...</span>
          </div>
        ) : (
          <>
            <div ref={listRef} className="space-y-1">
              {displayedFiles.map((file, index) => (
                <div
                  key={file.path}
                  onClick={() => handleItemClick(index)}
                  className={`flex items-center gap-3 p-2 rounded-md cursor-pointer transition-colors ${
                    index === selectedIndex
                      ? 'bg-bgProminent text-textProminentInverse'
                      : 'hover:bg-bgSubtle'
                  }`}
                >
                  <span className="text-lg flex-shrink-0">
                    {getFileIcon(file.name, file.isDirectory)}
                  </span>
                  <div className="flex-1 min-w-0">
                    <div className="font-medium text-sm truncate">
                      {file.name}
                    </div>
                    <div className="text-xs text-textSubtle truncate">
                      {file.path}
                    </div>
                  </div>
                </div>
              ))}
              
              {!isLoading && displayedFiles.length === 0 && query && (
                <div className="p-4 text-center text-textSubtle text-sm">
                  No files found matching "{query}"
                </div>
              )}
              
              {!isLoading && displayedFiles.length === 0 && !query && (
                <div className="p-4 text-center text-textSubtle text-sm">
                  Start typing to search for files
                </div>
              )}
            </div>
            
            {remainingCount > 0 && (
              <div className="mt-2 pt-2 border-t border-borderSubtle">
                <div className="text-xs text-textSubtle text-center">
                  Show {remainingCount} more...
                </div>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}