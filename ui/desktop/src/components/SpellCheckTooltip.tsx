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

  console.log('🖱️ TOOLTIP COMPONENT: Rendering with props:', {
    isVisible,
    position,
    suggestions,
    misspelledWord
  });

  if (!isVisible) {
    console.log('🖱️ TOOLTIP COMPONENT: Not visible, returning null');
    return null;
  }

  console.log('🖱️ TOOLTIP COMPONENT: Rendering visible tooltip');

  return (
    <div
      ref={tooltipRef}
      className="fixed z-50 bg-background-default border border-borderStandard rounded-lg shadow-lg py-2 min-w-48 max-w-64"
      style={{
        left: position.x,
        top: position.y - 1, // Only 1px from the top of the misspelled word
        transform: 'translateY(-100%)', // Position above the word
      }}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
    >
      {/* Header */}
      <div className="px-3 py-1 text-xs text-text-muted border-b border-borderSubtle mb-1">
        Suggestions for "{misspelledWord}"
      </div>

      {/* Suggestions */}
      {suggestions.length > 0 ? (
        <div className="max-h-32 overflow-y-auto">
          {suggestions.slice(0, 5).map((suggestion, index) => (
            <button
              key={index}
              onClick={() => onSuggestionSelect(suggestion)}
              className="w-full text-left px-3 py-2 text-sm text-text-default hover:bg-bgSubtle transition-colors flex items-center gap-2"
            >
              <span className="w-4 h-4 flex items-center justify-center text-xs bg-blue-500 text-white rounded text-[10px]">
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
      <div className="border-t border-borderSubtle my-1" />

      {/* Additional actions */}
      <button
        onClick={onAddToDictionary}
        className="w-full text-left px-3 py-1.5 text-xs text-text-muted hover:bg-bgSubtle transition-colors"
      >
        Add to dictionary
      </button>
      
      <button
        onClick={onIgnore}
        className="w-full text-left px-3 py-1.5 text-xs text-text-muted hover:bg-bgSubtle transition-colors"
      >
        Ignore word
      </button>
    </div>
  );
};

export default SpellCheckTooltip;
