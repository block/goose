import React, { useState, useEffect, useRef } from 'react';
import FuzzyFileSearch from './FuzzyFileSearch';

interface MentionPopoverProps {
  isOpen: boolean;
  onClose: () => void;
  onSelect: (filePath: string) => void;
  position: { x: number; y: number };
  query: string;
}

export default function MentionPopover({ 
  isOpen, 
  onClose, 
  onSelect, 
  position, 
  query
}: MentionPopoverProps) {
  const [isFuzzySearchOpen, setIsFuzzySearchOpen] = useState(false);
  const popoverRef = useRef<HTMLDivElement>(null);

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

  const handleFileSearch = () => {
    setIsFuzzySearchOpen(true);
  };

  const handleFuzzyFileSelect = (filePath: string) => {
    onSelect(filePath);
    setIsFuzzySearchOpen(false);
    onClose();
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      onClose();
    } else if (e.key === 'Enter') {
      handleFileSearch();
    }
  };

  if (!isOpen) return null;

  return (
    <>
      <div
        ref={popoverRef}
        className="fixed z-50 bg-bgApp border border-borderStandard rounded-lg shadow-lg min-w-64"
        style={{
          left: position.x,
          top: position.y,
        }}
      >
        <div className="p-3">
          <div className="text-sm font-medium text-textStandard mb-2">
            Attach files
          </div>
          <div className="space-y-2">
            <button
              onClick={handleFileSearch}
              onKeyDown={handleKeyDown}
              className="w-full text-left px-3 py-2 text-sm text-textStandard hover:bg-bgSubtle rounded-md transition-colors flex items-center gap-2"
            >
              <span className="text-textSubtle">📁</span>
              <div>
                <div>Search files on computer</div>
                <div className="text-xs text-textSubtle">
                  {query ? `Search for "${query}"` : 'Browse and search all files'}
                </div>
              </div>
            </button>
            <button
              onClick={() => {
                // Trigger the existing file selector
                window.electron.selectFileOrDirectory().then((path) => {
                  if (path) {
                    onSelect(path);
                    onClose();
                  }
                });
              }}
              className="w-full text-left px-3 py-2 text-sm text-textStandard hover:bg-bgSubtle rounded-md transition-colors flex items-center gap-2"
            >
              <span className="text-textSubtle">📎</span>
              <div>
                <div>Select file or folder</div>
                <div className="text-xs text-textSubtle">
                  Open file picker dialog
                </div>
              </div>
            </button>
          </div>
          <div className="mt-3 pt-2 border-t border-borderSubtle text-xs text-textSubtle">
            Press Enter to search • Esc to close
          </div>
        </div>
      </div>

      <FuzzyFileSearch
        isOpen={isFuzzySearchOpen}
        onClose={() => setIsFuzzySearchOpen(false)}
        onSelect={handleFuzzyFileSelect}
        searchFromRoot={true}
      />
    </>
  );
}