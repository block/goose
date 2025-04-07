import React, { useEffect, KeyboardEvent, useState } from 'react';
import { Search as SearchIcon } from 'lucide-react';
import { ArrowDown, ArrowUp, Close } from '../icons';

/**
 * Props for the SearchBar component
 */
interface SearchBarProps {
  /** Callback fired when search term or case sensitivity changes */
  onSearch: (term: string, caseSensitive: boolean) => void;
  /** Callback fired when the search bar is closed */
  onClose: () => void;
  /** Optional callback for navigating between search results */
  onNavigate?: (direction: 'next' | 'prev') => void;
  /** Current search results state */
  searchResults?: {
    count: number;
    currentIndex: number;
  };
}

/**
 * SearchBar provides a search input with case-sensitive toggle and result navigation.
 * Features:
 * - Case-sensitive search toggle
 * - Result count display
 * - Navigation between results with arrows
 * - Keyboard shortcuts (↑/↓ for navigation, Esc to close)
 * - Smooth animations for enter/exit
 */
export const SearchBar: React.FC<SearchBarProps> = ({
  onSearch,
  onClose,
  onNavigate,
  searchResults,
}) => {
  const [searchTerm, setSearchTerm] = useState('');
  const [caseSensitive, setCaseSensitive] = useState(false);
  const [isExiting, setIsExiting] = useState(false);
  const inputRef = React.useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const handleSearch = (event: React.ChangeEvent<HTMLInputElement>) => {
    const value = event.target.value;
    setSearchTerm(value);
    onSearch(value, caseSensitive);
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'ArrowUp') {
      handleNavigate('prev', event);
    } else if (event.key === 'ArrowDown') {
      handleNavigate('next', event);
    } else if (event.key === 'Escape') {
      event.preventDefault();
      handleClose();
    }
  };

  const handleNavigate = (direction: 'next' | 'prev', e?: React.MouseEvent | KeyboardEvent) => {
    e?.preventDefault();
    onNavigate?.(direction);
    inputRef.current?.focus();
  };

  const toggleCaseSensitive = () => {
    setCaseSensitive(!caseSensitive);
    onSearch(searchTerm, !caseSensitive);
    inputRef.current?.focus();
  };

  const handleClose = () => {
    setIsExiting(true);
    setTimeout(() => {
      onClose();
    }, 150); // Match animation duration
  };

  return (
    <div
      className={`sticky top-0 bg-bgAppInverse text-textProminentInverse z-50 ${
        isExiting ? 'search-bar-exit' : 'search-bar-enter'
      }`}
    >
      <div className="flex w-full max-w-5xl mx-auto">
        <div className="relative flex flex-1 items-center h-full">
          <SearchIcon className="h-4 w-4 text-textSubtleInverse absolute left-3" />
          <div className="w-full">
            <input
              ref={inputRef}
              id="search-input"
              type="text"
              value={searchTerm}
              onChange={handleSearch}
              onKeyDown={handleKeyDown}
              placeholder="Search conversation..."
              className="w-full text-sm pl-9 pr-10 py-3 bg-bgAppInverse
                      placeholder:text-textSubtleInverse focus:outline-none 
                       active:border-borderProminent"
            />
          </div>

          <div className="absolute right-3 flex h-full items-center justify-center text-sm text-textStandardInverse">
            {searchResults && searchResults.count}
          </div>
        </div>

        <div className="flex items-center justify-center h-auto px-4 gap-2">
          <button
            onClick={toggleCaseSensitive}
            className={`flex items-center justify-center case-sensitive-btn px-2 ${
              caseSensitive
                ? 'text-textStandardInverse bg-bgHover'
                : 'text-textSubtleInverse hover:text-textStandardInverse'
            }`}
            title="Case Sensitive"
          >
            <span className="text-md font-medium">Aa</span>
          </button>

          <button
            onClick={(e) => handleNavigate('prev', e)}
            className={`p-1 text-textSubtleInverse ${!searchResults || searchResults.count === 0 ? '' : 'hover:text-textStandardInverse'}`}
            title="Previous (↑)"
            disabled={!searchResults || searchResults.count === 0}
          >
            <ArrowUp className="h-5 w-5" />
          </button>
          <button
            onClick={(e) => handleNavigate('next', e)}
            className={`p-1 text-textSubtleInverse ${!searchResults || searchResults.count === 0 ? '' : 'hover:text-textStandardInverse'}`}
            title="Next (↓)"
            disabled={!searchResults || searchResults.count === 0}
          >
            <ArrowDown className="h-5 w-5" />
          </button>

          <button
            onClick={handleClose}
            className="p-1 text-textSubtleInverse hover:text-textStandardInverse"
            title="Close (Esc)"
          >
            <Close className="h-5 w-5" />
          </button>
        </div>
      </div>
    </div>
  );
};
