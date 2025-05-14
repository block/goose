import React, { useEffect, useState, useRef, KeyboardEvent } from 'react';
import { Search as SearchIcon } from 'lucide-react';
import { ArrowDown, ArrowUp, Close } from '../icons';
import { debounce } from 'lodash';

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
  /** Optional ref for the search input element */
  inputRef?: React.RefObject<HTMLInputElement>;
  /** Initial search term */
  initialSearchTerm?: string;
}

/**
 * SearchBar provides a search input with case-sensitive toggle and result navigation.
 */
export const SearchBar: React.FC<SearchBarProps> = ({
  onSearch,
  onClose,
  onNavigate,
  searchResults,
  inputRef: externalInputRef,
  initialSearchTerm = '',
}: SearchBarProps) => {
  const [searchTerm, setSearchTerm] = useState(initialSearchTerm);
  const [caseSensitive, setCaseSensitive] = useState(false);
  const [isExiting, setIsExiting] = useState(false);
  const internalInputRef = React.useRef<HTMLInputElement>(null);
  const inputRef = externalInputRef || internalInputRef;
  const debouncedSearchRef = useRef<ReturnType<typeof debounce>>();

  // Create debounced search function
  useEffect(() => {
    const debouncedFn = debounce((term: string, caseSensitive: boolean) => {
      onSearch(term, caseSensitive);
    }, 200);

    debouncedSearchRef.current = debouncedFn;

    return () => {
      debouncedFn.cancel();
    };
  }, [onSearch]);

  useEffect(() => {
    inputRef.current?.focus();
  }, [inputRef]);

  // Handle changes to initialSearchTerm
  useEffect(() => {
    if (initialSearchTerm) {
      setSearchTerm(initialSearchTerm);
      if (initialSearchTerm.length >= 2) {
        debouncedSearchRef.current?.(initialSearchTerm, caseSensitive);
      }
    }
  }, [initialSearchTerm, caseSensitive, debouncedSearchRef]);

  const [localSearchResults, setLocalSearchResults] = useState<typeof searchResults>(null);

  // Sync external search results with local state
  useEffect(() => {
    // Only set results if we have a search term
    if (!searchTerm) {
      setLocalSearchResults(null);
    } else {
      setLocalSearchResults(searchResults);
    }
  }, [searchResults, searchTerm]);

  const handleSearch = (event: React.ChangeEvent<HTMLInputElement>) => {
    const value = event.target.value;

    // Always cancel pending searches first
    if (debouncedSearchRef.current) {
      debouncedSearchRef.current.cancel();
    }

    // Update display term immediately for UI feedback
    setSearchTerm(value);

    // Only trigger search if we have 2 or more characters
    if (value.length >= 2) {
      debouncedSearchRef.current?.(value, caseSensitive);
    } else {
      // Clear results if less than 2 characters
      onSearch('', caseSensitive);
    }
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'ArrowUp') {
      handleNavigate('prev', event);
    } else if (event.key === 'ArrowDown' || event.key === 'Enter') {
      handleNavigate('next', event);
    } else if (event.key === 'Escape') {
      event.preventDefault();
      handleClose();
    }
  };

  const handleNavigate = (direction: 'next' | 'prev', e?: React.MouseEvent | KeyboardEvent) => {
    e?.preventDefault();
    if (searchResults && searchResults.count > 0) {
      inputRef.current?.focus();
      onNavigate?.(direction);
    }
  };

  const toggleCaseSensitive = () => {
    const newCaseSensitive = !caseSensitive;
    setCaseSensitive(newCaseSensitive);
    // Immediately trigger a new search with updated case sensitivity
    if (searchTerm) {
      debouncedSearchRef.current?.(searchTerm, newCaseSensitive);
    }
    inputRef.current?.focus();
  };

  const handleClose = () => {
    setIsExiting(true);
    debouncedSearchRef.current?.cancel(); // Cancel any pending searches
    setTimeout(() => {
      onClose();
    }, 150); // Match animation duration
  };

  const hasResults = searchResults && searchResults.count > 0;

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
              className="w-full text-sm pl-9 pr-24 py-3 bg-bgAppInverse
                      placeholder:text-textSubtleInverse focus:outline-none 
                       active:border-borderProminent"
            />
          </div>

          <div className="absolute right-3 flex h-full items-center justify-end">
            <div className="flex items-center gap-1">
              <div className="w-16 text-right text-sm text-textStandardInverse flex items-center justify-end">
                {(() => {
                  return localSearchResults?.count > 0 && searchTerm
                    ? `${localSearchResults.currentIndex}/${localSearchResults.count}`
                    : null;
                })()}
              </div>
            </div>
          </div>
        </div>

        <div className="flex items-center justify-center h-auto px-4 gap-2">
          <button
            onClick={toggleCaseSensitive}
            className={`flex items-center justify-center min-w-[32px] h-[28px] rounded transition-all duration-150 ${
              caseSensitive
                ? 'bg-white/20 shadow-[inset_0_1px_2px_rgba(0,0,0,0.2)]'
                : 'text-textSubtleInverse hover:text-textStandardInverse hover:bg-white/5'
            }`}
            title="Case Sensitive"
          >
            <span className="text-md font-normal">Aa</span>
          </button>

          <div className="flex items-center gap-2">
            <button onClick={(e) => handleNavigate('prev', e)} className="p-1" title="Previous (↑)">
              <ArrowUp
                className={`h-5 w-5 transition-opacity ${
                  !hasResults
                    ? 'opacity-30'
                    : 'text-textSubtleInverse hover:text-textStandardInverse'
                }`}
              />
            </button>
            <button
              onClick={(e) => handleNavigate('next', e)}
              className="p-1"
              title="Next (↓ or Enter)"
            >
              <ArrowDown
                className={`h-5 w-5 transition-opacity ${
                  !hasResults
                    ? 'opacity-30'
                    : 'text-textSubtleInverse hover:text-textStandardInverse'
                }`}
              />
            </button>
          </div>

          <button onClick={handleClose} className="p-1" title="Close (Esc)">
            <Close className="h-5 w-5 text-textSubtleInverse hover:text-textStandardInverse" />
          </button>
        </div>
      </div>
    </div>
  );
};

export default SearchBar;
