import React, { useState, useRef, useEffect } from 'react';

interface SpellCheckTooltipProps {
  isVisible: boolean;
  position: { x: number; y: number };
  suggestions: string[];
  misspelledWord: string;
  onSuggestionSelect: (suggestion: string) => void;
  onAddToDictionary: () => void;
  onIgnore: () => void;
  onMouseEnter: () => void;
  onMouseLeave: () => void;
}

const SpellCheckTooltip: React.FC<SpellCheckTooltipProps> = ({
  isVisible,
  position,
  suggestions,
  misspelledWord,
  onSuggestionSelect,
  onAddToDictionary,
  onIgnore,
  onMouseEnter,
  onMouseLeave,
}) => {
  const tooltipRef = useRef<HTMLDivElement>(null);
  const [adjustedPosition, setAdjustedPosition] = useState(position);
  const [selectedIndex, setSelectedIndex] = useState(0);

  // Adjust position to keep tooltip within viewport
  useEffect(() => {
    if (isVisible && tooltipRef.current) {
      const tooltip = tooltipRef.current;
      const rect = tooltip.getBoundingClientRect();
      const viewportWidth = window.innerWidth;
      const viewportHeight = window.innerHeight;
      const tooltipWidth = rect.width || 200; // fallback width
      const tooltipHeight = rect.height || 100; // fallback height

      let adjustedX = position.x;
      let adjustedY = position.y;

      // Calculate the tooltip's left edge when centered
      const tooltipLeftEdge = position.x - (tooltipWidth / 2);
      const tooltipRightEdge = position.x + (tooltipWidth / 2);

      // Adjust horizontal position if tooltip would overflow
      if (tooltipLeftEdge < 10) {
        // If left edge would be cut off, align to left edge with padding
        adjustedX = (tooltipWidth / 2) + 10;
      } else if (tooltipRightEdge > viewportWidth - 10) {
        // If right edge would be cut off, align to right edge with padding
        adjustedX = viewportWidth - (tooltipWidth / 2) - 10;
      }

      // Adjust vertical position if tooltip would overflow
      if (position.y + tooltipHeight + 50 > viewportHeight) {
        // Show above the word instead of below
        adjustedY = position.y - tooltipHeight - 10;
      }
      
      // Ensure tooltip doesn't go above viewport
      if (adjustedY < 10) {
        adjustedY = 10;
      }

      setAdjustedPosition({ x: adjustedX, y: adjustedY });
    }
  }, [isVisible, position]);

  if (!isVisible) return null;

  console.log('🖱️ TOOLTIP COMPONENT: Rendering visible tooltip');

  return (
    <div
      ref={tooltipRef}
      tabIndex={-1} // Make it focusable for keyboard events
      data-spell-tooltip="true" // For click detection
      className="fixed z-50 bg-background-default border border-border-default rounded-lg shadow-xl py-2 min-w-48 max-w-64 outline-none"
      style={{
        left: `${adjustedPosition.x}px`,
        top: `${adjustedPosition.y - 5}px`, // Position right above the word
        transform: 'translateX(-50%)', // Only center horizontally
        boxShadow: '0 10px 25px rgba(0, 0, 0, 0.15), 0 4px 6px rgba(0, 0, 0, 0.1)',
      }}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
    >
      {/* Header */}
      <div className="px-3 py-1 text-xs text-text-muted border-b border-border-subtle mb-1 font-medium">
        Suggestions for "<span className="text-red-600 dark:text-red-400 font-semibold">{misspelledWord}</span>"
      </div>

      {/* Suggestions */}
      {suggestions.length > 0 ? (
        <div className="max-h-32 overflow-y-auto">
          {suggestions.slice(0, 5).map((suggestion, index) => (
            <button
              key={index}
              onClick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                console.log('🖱️ SUGGESTION CLICKED:', suggestion);
                onSuggestionSelect(suggestion);
              }}
              onMouseEnter={() => setSelectedIndex(index)}
              className={`w-full text-left px-3 py-2 text-sm transition-all duration-150 flex items-center gap-2 ${
                selectedIndex === index
                  ? 'bg-blue-50 dark:bg-blue-900/20 text-blue-900 dark:text-blue-100 border-l-2 border-blue-500'
                  : 'text-text-default hover:bg-background-subtle'
              }`}
            >
              <span 
                className={`w-5 h-5 flex items-center justify-center text-xs rounded text-[10px] font-bold ${
                  selectedIndex === index
                    ? 'bg-blue-500 text-white'
                    : 'bg-text-muted text-white'
                }`}
              >
                {index + 1}
              </span>
              <span className="font-medium truncate">{suggestion}</span>
            </button>
          ))}
        </div>
      ) : (
        <div className="px-3 py-2 text-sm text-text-muted italic">
          No suggestions available
        </div>
      )}

      {/* Separator */}
      <div className="border-t border-border-subtle my-1" />

      {/* Additional actions */}
      <button
        onClick={(e) => {
          e.preventDefault();
          e.stopPropagation();
          console.log('🖱️ ADD TO DICTIONARY CLICKED');
          onAddToDictionary();
        }}
        className="w-full text-left px-3 py-1.5 text-xs text-text-muted hover:bg-background-subtle transition-colors flex items-center gap-2"
      >
        <span className="text-green-600 dark:text-green-400">+</span>
        Add to dictionary
      </button>
      
      <button
        onClick={(e) => {
          e.preventDefault();
          e.stopPropagation();
          console.log('🖱️ IGNORE CLICKED');
          onIgnore();
        }}
        className="w-full text-left px-3 py-1.5 text-xs text-text-muted hover:bg-background-subtle transition-colors flex items-center gap-2"
      >
        <span className="text-text-muted">×</span>
        Ignore word
      </button>

      {/* Keyboard hints */}
      <div className="px-3 py-1 text-[10px] text-text-muted border-t border-border-subtle mt-1">
        Press 1-5 to select • ↑↓ to navigate • Enter to apply • Esc to close
      </div>
    </div>
  );
};

export default SpellCheckTooltip;
