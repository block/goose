import React, { useState, useRef, useEffect } from 'react';

interface SpellCheckTooltipProps {
  isVisible: boolean;
  position: { x: number; y: number };
  suggestions: string[];
  misspelledWord: string;
  onSuggestionSelect: (suggestion: string) => void;
  onAddToDictionary: () => void;
  onIgnore: () => void;
  onMouseEnter?: () => void;
  onMouseLeave?: () => void;
}

export const SpellCheckTooltip: React.FC<SpellCheckTooltipProps> = ({
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
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [adjustedPosition, setAdjustedPosition] = useState(position);

  console.log('🖱️ TOOLTIP COMPONENT: Rendering with props:', {
    isVisible,
    position,
    suggestions,
    misspelledWord
  });

  // Calculate smart positioning to avoid window edges
  useEffect(() => {
    if (!isVisible) {
      setAdjustedPosition(position);
      return;
    }

    // Use a timeout to allow the tooltip to render first
    const timer = setTimeout(() => {
      if (!tooltipRef.current) return;

      const tooltip = tooltipRef.current;
      const tooltipRect = tooltip.getBoundingClientRect();
      const windowWidth = window.innerWidth;
      const windowHeight = window.innerHeight;
      
      let newX = position.x;
      let newY = position.y;
      let transform = 'translateX(-50%) translateY(-100%)'; // Default: center horizontally, above word

      // Check horizontal boundaries
      const tooltipWidth = tooltipRect.width || 200; // Fallback width
      const halfWidth = tooltipWidth / 2;
      const margin = 10; // Margin from window edges
      
      if (newX - halfWidth < margin) {
        // Too close to left edge - align to left
        newX = margin;
        transform = 'translateY(-100%)'; // Remove horizontal centering
      } else if (newX + halfWidth > windowWidth - margin) {
        // Too close to right edge - align to right
        newX = windowWidth - margin;
        transform = 'translateX(-100%) translateY(-100%)'; // Align to right edge
      }

      // Check vertical boundaries
      const tooltipHeight = tooltipRect.height || 150; // Fallback height
      
      if (newY - tooltipHeight < margin) {
        // Not enough space above - show below the word
        newY = position.y + 30; // Position below word
        if (transform.includes('translateX(-50%)')) {
          transform = 'translateX(-50%) translateY(0%)'; // Center horizontally, below word
        } else if (transform.includes('translateX(-100%)')) {
          transform = 'translateX(-100%) translateY(0%)'; // Right align, below word
        } else {
          transform = 'translateY(0%)'; // Left align, below word
        }
      }

      setAdjustedPosition({ x: newX, y: newY });
      
      // Apply the transform
      tooltip.style.transform = transform;
      
      console.log('🖱️ TOOLTIP POSITIONING:', {
        original: position,
        adjusted: { x: newX, y: newY },
        transform,
        windowSize: { width: windowWidth, height: windowHeight },
        tooltipSize: { width: tooltipWidth, height: tooltipHeight }
      });
    }, 0);

    return () => clearTimeout(timer);
  }, [isVisible, position]);

  // Handle keyboard navigation
  useEffect(() => {
    if (!isVisible) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      switch (e.key) {
        case 'ArrowDown':
          e.preventDefault();
          setSelectedIndex(prev => 
            Math.min(prev + 1, suggestions.length - 1)
          );
          break;
        case 'ArrowUp':
          e.preventDefault();
          setSelectedIndex(prev => Math.max(prev - 1, 0));
          break;
        case 'Enter':
          e.preventDefault();
          if (suggestions[selectedIndex]) {
            onSuggestionSelect(suggestions[selectedIndex]);
          }
          break;
        case 'Escape':
          e.preventDefault();
          onIgnore(); // Close tooltip on escape
          break;
        case '1':
        case '2':
        case '3':
        case '4':
        case '5':
          e.preventDefault();
          const numIndex = parseInt(e.key) - 1;
          if (suggestions[numIndex]) {
            onSuggestionSelect(suggestions[numIndex]);
          }
          break;
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [isVisible, suggestions, selectedIndex, onSuggestionSelect, onIgnore]);

  // Reset selected index when suggestions change
  useEffect(() => {
    setSelectedIndex(0);
  }, [suggestions]);

  // Auto-focus tooltip when it becomes visible
  useEffect(() => {
    if (isVisible && tooltipRef.current) {
      tooltipRef.current.focus();
    }
  }, [isVisible]);

  if (!isVisible) {
    console.log('🖱️ TOOLTIP COMPONENT: Not visible, returning null');
    return null;
  }

  console.log('🖱️ TOOLTIP COMPONENT: Rendering visible tooltip');

  return (
    <div
      ref={tooltipRef}
      tabIndex={-1} // Make it focusable for keyboard events
      data-spell-tooltip="true" // For click detection
      className="fixed z-50 bg-background-default border border-border-default rounded-lg shadow-xl py-2 min-w-48 max-w-64 outline-none"
      style={{
        left: adjustedPosition.x,
        top: adjustedPosition.y - 8, // Position slightly above the word
        transform: 'translateX(-50%) translateY(-100%)', // Will be overridden by smart positioning
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
