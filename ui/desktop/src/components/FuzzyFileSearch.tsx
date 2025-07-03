import React, { useState, useEffect, useRef, useMemo } from 'react';
import { Card } from './ui/card';
import { ScrollArea } from './ui/scroll-area';
import { Close } from './icons';

interface FileItem {
  path: string;
  name: string;
  isDirectory: boolean;
  relativePath: string;
}

interface FuzzyFileSearchProps {
  isOpen: boolean;
  onClose: () => void;
  onSelect: (filePath: string) => void;
  workingDirectory: string;
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

// Highlight matched characters in the text
const HighlightedText: React.FC<{ text: string; matches: number[] }> = ({ text, matches }) => {
  if (matches.length === 0) return <span>{text}</span>;
  
  const elements: React.ReactNode[] = [];
  let lastIndex = 0;
  
  matches.forEach((matchIndex, i) => {
    // Add text before the match
    if (matchIndex > lastIndex) {
      elements.push(text.slice(lastIndex, matchIndex));
    }
    
    // Add the highlighted match
    elements.push(
      <span key={i} className="bg-yellow-200 dark:bg-yellow-800 text-yellow-900 dark:text-yellow-100">
        {text[matchIndex]}
      </span>
    );
    
    lastIndex = matchIndex + 1;
  });
  
  // Add remaining text
  if (lastIndex < text.length) {
    elements.push(text.slice(lastIndex));
  }
  
  return <span>{elements}</span>;
};

export default function FuzzyFileSearch({ isOpen, onClose, onSelect, workingDirectory }: FuzzyFileSearchProps) {
  const [query, setQuery] = useState('');
  const [files, setFiles] = useState<FileItem[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [isLoading, setIsLoading] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  
  // Scan files when component opens or working directory changes
  useEffect(() => {
    if (isOpen && workingDirectory) {
      scanFiles();
    }
  }, [isOpen, workingDirectory]);
  
  // Focus input when opened
  useEffect(() => {
    if (isOpen && inputRef.current) {
      inputRef.current.focus();
    }
  }, [isOpen]);
  
  // Reset state when opened/closed
  useEffect(() => {
    if (isOpen) {
      setQuery('');
      setSelectedIndex(0);
    }
  }, [isOpen]);
  
  const scanFiles = async () => {
    setIsLoading(true);
    try {
      const scannedFiles = await scanDirectory(workingDirectory);
      setFiles(scannedFiles);
    } catch (error) {
      console.error('Error scanning files:', error);
      setFiles([]);
    } finally {
      setIsLoading(false);
    }
  };
  
  const scanDirectory = async (dirPath: string, relativePath = ''): Promise<FileItem[]> => {
    try {
      const items = await window.electron.listFiles(dirPath);
      const results: FileItem[] = [];
      
      // Add current directory files
      for (const item of items) {
        const fullPath = `${dirPath}/${item}`;
        const itemRelativePath = relativePath ? `${relativePath}/${item}` : item;
        
        // Skip hidden files and common ignore patterns
        if (item.startsWith('.') || 
            item === 'node_modules' || 
            item === '.git' || 
            item === '__pycache__' || 
            item === '.vscode' ||
            item === 'target' ||
            item === 'dist' ||
            item === 'build') {
          continue;
        }
        
        try {
          // Check if it's a directory by trying to list its contents
          const subItems = await window.electron.listFiles(fullPath);
          
          // It's a directory
          results.push({
            path: fullPath,
            name: item,
            isDirectory: true,
            relativePath: itemRelativePath
          });
          
          // Recursively scan subdirectories (limit depth to avoid performance issues)
          if (relativePath.split('/').length < 3) {
            const subFiles = await scanDirectory(fullPath, itemRelativePath);
            results.push(...subFiles);
          }
        } catch {
          // It's a file
          results.push({
            path: fullPath,
            name: item,
            isDirectory: false,
            relativePath: itemRelativePath
          });
        }
      }
      
      return results;
    } catch (error) {
      console.error(`Error scanning directory ${dirPath}:`, error);
      return [];
    }
  };
  
  // Filter and sort files based on query
  const filteredFiles = useMemo(() => {
    if (!query.trim()) {
      return files.slice(0, 50); // Show first 50 files when no query
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
      .slice(0, 50); // Limit to 50 results
    
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
  
  return (
    <div className="fixed inset-0 bg-black bg-opacity-50 flex items-start justify-center pt-20 z-50">
      <Card className="w-full max-w-2xl mx-4 bg-bgApp border-borderStandard">
        <div className="p-4">
          <div className="flex items-center gap-2 mb-4">
            <div className="flex-1 relative">
              <input
                ref={inputRef}
                type="text"
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder="Type to search files..."
                className="w-full px-3 py-2 border border-borderSubtle rounded-md bg-bgApp text-textStandard placeholder-textPlaceholder focus:outline-none focus:border-borderProminent"
              />
              {isLoading && (
                <div className="absolute right-3 top-1/2 transform -translate-y-1/2">
                  <div className="animate-spin rounded-full h-4 w-4 border-t-2 border-b-2 border-textSubtle"></div>
                </div>
              )}
            </div>
            <button
              onClick={onClose}
              className="p-2 text-textSubtle hover:text-textStandard rounded-md hover:bg-bgSubtle"
            >
              <Close className="w-4 h-4" />
            </button>
          </div>
          
          <div className="text-xs text-textSubtle mb-2">
            {isLoading ? 'Scanning files...' : `${filteredFiles.length} files found`}
          </div>
          
          <ScrollArea className="h-96">
            <div ref={listRef} className="space-y-1">
              {filteredFiles.map((file, index) => (
                <div
                  key={file.path}
                  onClick={() => handleItemClick(index)}
                  className={`p-3 rounded-md cursor-pointer transition-colors ${
                    index === selectedIndex
                      ? 'bg-bgProminent text-textProminentInverse'
                      : 'hover:bg-bgSubtle'
                  }`}
                >
                  <div className="flex items-center gap-2">
                    <span className={`text-xs px-1.5 py-0.5 rounded ${
                      file.isDirectory 
                        ? 'bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200'
                        : 'bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-200'
                    }`}>
                      {file.isDirectory ? 'DIR' : 'FILE'}
                    </span>
                    <div className="flex-1 min-w-0">
                      <div className="font-medium truncate">
                        <HighlightedText 
                          text={file.name} 
                          matches={file.matchedText === file.name ? (file as any).matches : []} 
                        />
                      </div>
                      <div className="text-sm text-textSubtle truncate">
                        <HighlightedText 
                          text={file.relativePath} 
                          matches={file.matchedText === file.relativePath ? (file as any).matches : []} 
                        />
                      </div>
                    </div>
                  </div>
                </div>
              ))}
              
              {!isLoading && filteredFiles.length === 0 && query && (
                <div className="p-8 text-center text-textSubtle">
                  No files found matching "{query}"
                </div>
              )}
              
              {!isLoading && filteredFiles.length === 0 && !query && (
                <div className="p-8 text-center text-textSubtle">
                  Start typing to search for files
                </div>
              )}
            </div>
          </ScrollArea>
          
          <div className="mt-4 text-xs text-textSubtle border-t border-borderSubtle pt-2">
            <div className="flex justify-between">
              <span>↑↓ Navigate • Enter Select • Esc Close</span>
              <span>{workingDirectory}</span>
            </div>
          </div>
        </div>
      </Card>
    </div>
  );
}