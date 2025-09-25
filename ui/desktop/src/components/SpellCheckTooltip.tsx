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

  // Adjust position to keep tooltip within viewport
  useEffect(() => {
    if (isVisible && tooltipRef.current) {
      const tooltip = tooltipRef.current;
      const rect = tooltip.getBoundingClientRect();
      const viewportWidth = window.innerWidth;
      const viewportHeight = window.innerHeight;

      let adjustedX = position.x;
      let adjustedY = position.y;

      // Adjust horizontal position if tooltip would overflow
      if (position.x + rect.width > viewportWidth) {
        adjustedX = viewportWidth - rect.width - 10;
      }
      if (adjustedX < 10) {
        adjustedX = 10;
      }

      // Adjust vertical position if tooltip would overflow
      if (position.y + rect.height > viewportHeight) {
        adjustedY = position.y - rect.height - 10;
      }
      if (adjustedY < 10) {
        adjustedY = 10;
      }

      setAdjustedPosition({ x: adjustedX, y: adjustedY });
    }
  }, [isVisible, position]);

  if (!isVisible) return null;

  return (
    <div
      ref={tooltipRef}
      data-spell-tooltip="true"
      className="fixed z-[9999] bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-lg py-2 px-0 min-w-[200px] max-w-[300px]"
      style={{
        left: `${adjustedPosition.x}px`,
        top: `${adjustedPosition.y + 25}px`, // Offset below the word
        transform: 'translateX(-50%)', // Center horizontally
      }}
      onMouseEnter={onMouseEnter}
      onMouseLeave={onMouseLeave}
    >
      {/* Header */}
      <div className="px-3 pb-2 border-b border-gray-200 dark:border-gray-700">
        <div className="text-sm font-medium text-gray-900 dark:text-gray-100">
          Misspelled: <span className="text-red-600 dark:text-red-400">{misspelledWord}</span>
        </div>
      </div>

      {/* Suggestions */}
      {suggestions.length > 0 && (
        <div className="py-1">
          <div className="px-3 py-1 text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wide">
            Suggestions
          </div>
          {suggestions.slice(0, 5).map((suggestion, index) => (
            <button
              key={index}
              className="w-full px-3 py-2 text-left text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 focus:bg-gray-100 dark:focus:bg-gray-700 focus:outline-none transition-colors duration-150"
              onClick={() => onSuggestionSelect(suggestion)}
            >
              {suggestion}
            </button>
          ))}
        </div>
      )}

      {/* No suggestions message */}
      {suggestions.length === 0 && (
        <div className="px-3 py-2 text-sm text-gray-500 dark:text-gray-400">
          No suggestions available
        </div>
      )}

      {/* Actions */}
      <div className="border-t border-gray-200 dark:border-gray-700 pt-1">
        <button
          className="w-full px-3 py-2 text-left text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 focus:bg-gray-100 dark:focus:bg-gray-700 focus:outline-none transition-colors duration-150"
          onClick={onAddToDictionary}
        >
          Add to Dictionary
        </button>
        <button
          className="w-full px-3 py-2 text-left text-sm text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700 focus:bg-gray-100 dark:focus:bg-gray-700 focus:outline-none transition-colors duration-150"
          onClick={onIgnore}
        >
          Ignore
        </button>
      </div>
    </div>
  );
};

export default SpellCheckTooltip;
