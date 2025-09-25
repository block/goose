import React, { useEffect, useRef } from 'react';

interface SpellCheckContextMenuProps {
  isOpen: boolean;
  position: { x: number; y: number };
  suggestions: string[];
  misspelledWord: string;
  onSuggestionSelect: (suggestion: string) => void;
  onAddToDictionary: () => void;
  onIgnore: () => void;
  onClose: () => void;
}

export const SpellCheckContextMenu: React.FC<SpellCheckContextMenuProps> = ({
  isOpen,
  position,
  suggestions,
  misspelledWord,
  onSuggestionSelect,
  onAddToDictionary,
  onIgnore,
  onClose,
}) => {
  const menuRef = useRef<HTMLDivElement>(null);

  // Close menu when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        onClose();
      }
    };

    if (isOpen) {
      document.addEventListener('mousedown', handleClickOutside);
      // Prevent the default context menu
      document.addEventListener('contextmenu', (e) => e.preventDefault());
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('contextmenu', (e) => e.preventDefault());
    };
  }, [isOpen, onClose]);

  // Close menu on escape key
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onClose();
      }
    };

    if (isOpen) {
      document.addEventListener('keydown', handleKeyDown);
    }

    return () => {
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  return (
    <div
      ref={menuRef}
      className="fixed z-50 bg-background-default border border-borderStandard rounded-lg shadow-lg py-2 min-w-48"
      style={{
        left: position.x,
        top: position.y,
      }}
    >
      {/* Misspelled word header */}
      <div className="px-3 py-1 text-xs text-text-muted border-b border-borderSubtle mb-1">
        Suggestions for "{misspelledWord}"
      </div>

      {/* Suggestions */}
      {suggestions.length > 0 ? (
        suggestions.map((suggestion, index) => (
          <button
            key={index}
            onClick={() => onSuggestionSelect(suggestion)}
            className="w-full text-left px-3 py-2 text-sm text-text-default hover:bg-bgSubtle transition-colors flex items-center gap-2"
          >
            <span className="w-4 h-4 flex items-center justify-center text-xs bg-blue-500 text-white rounded">
              {index + 1}
            </span>
            <span className="font-medium">{suggestion}</span>
          </button>
        ))
      ) : (
        <div className="px-3 py-2 text-sm text-text-muted italic">
          No suggestions available
        </div>
      )}

      {/* Separator */}
      <div className="border-t border-borderSubtle my-1" />

      {/* Additional actions */}
      <button
        onClick={onAddToDictionary}
        className="w-full text-left px-3 py-2 text-sm text-text-default hover:bg-bgSubtle transition-colors"
      >
        Add "{misspelledWord}" to dictionary
      </button>
      
      <button
        onClick={onIgnore}
        className="w-full text-left px-3 py-2 text-sm text-text-default hover:bg-bgSubtle transition-colors"
      >
        Ignore "{misspelledWord}"
      </button>
    </div>
  );
};

export default SpellCheckContextMenu;
