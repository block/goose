import React, { useEffect, KeyboardEvent, useState } from 'react';
import { Search as SearchIcon, X as XIcon, ArrowUp, ArrowDown } from 'lucide-react';

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
      event.preventDefault();
      onNavigate?.('prev');
    } else if (event.key === 'ArrowDown') {
      event.preventDefault();
      onNavigate?.('next');
    } else if (event.key === 'Escape') {
      event.preventDefault();
      handleClose();
    }
  };

  const handleNavigate = (direction: 'next' | 'prev') => {
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
      className={`fixed top-[36px] left-0 right-0 bg-bgSubtle border-b border-borderSubtle z-50 rounded-b-xl ${
        isExiting ? 'search-bar-exit' : 'search-bar-enter'
      }`}
    >
      <div className="flex items-center w-full max-w-5xl mx-auto p-4 pb-3">
        <div className="relative flex items-center flex-1">
          <SearchIcon className="h-4 w-4 text-textSubtle absolute left-3" />
          <div className="flex-1">
            <input
              ref={inputRef}
              id="search-input"
              type="text"
              value={searchTerm}
              onChange={handleSearch}
              onKeyDown={handleKeyDown}
              placeholder="Search conversation..."
              className="w-full pl-9 pr-10 py-1.5 bg-bgApp rounded border border-borderSubtle 
                       text-textStandard placeholder:text-textSubtle focus:outline-none focus:ring-1 
                       focus:ring-borderHover"
            />
          </div>
        </div>

        <button
          onClick={toggleCaseSensitive}
          className={`ml-2 p-1 hover:bg-bgHover rounded case-sensitive-btn ${
            caseSensitive
              ? 'text-textStandard bg-bgHover'
              : 'text-textSubtle hover:text-textStandard'
          }`}
          title="Case Sensitive"
        >
          <span className="text-sm font-medium">Cc</span>
        </button>

        <div className="flex items-center ml-4 space-x-2">
          {searchResults && searchResults.count > 0 && (
            <>
              <span className="text-sm text-textSubtle">
                {searchResults.currentIndex} of {searchResults.count}
              </span>
              <div className="flex space-x-1">
                <button
                  onClick={() => handleNavigate('prev')}
                  className="p-1 hover:bg-bgHover rounded text-textSubtle hover:text-textStandard"
                  title="Previous (↑)"
                >
                  <ArrowUp className="h-4 w-4" />
                </button>
                <button
                  onClick={() => handleNavigate('next')}
                  className="p-1 hover:bg-bgHover rounded text-textSubtle hover:text-textStandard"
                  title="Next (↓)"
                >
                  <ArrowDown className="h-4 w-4" />
                </button>
              </div>
            </>
          )}
          <button
            onClick={handleClose}
            className="p-1 hover:bg-bgHover rounded text-textSubtle hover:text-textStandard"
            title="Close (Esc)"
          >
            <XIcon className="h-4 w-4" />
          </button>
        </div>
      </div>
    </div>
  );
};
