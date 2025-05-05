import React, { useEffect, useState, useRef, useCallback, KeyboardEvent } from 'react';
import { Search as SearchIcon, XCircle } from 'lucide-react';
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
 * Features:
 * - Case-sensitive search toggle
 * - Result count display
 * - Navigation between results with arrows
 * - Keyboard shortcuts (↑/↓/Enter for navigation, Esc to close)
 * - Smooth animations for enter/exit
 * - Debounced search for better performance
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
  const [displayTerm, setDisplayTerm] = useState(initialSearchTerm); // For immediate visual feedback
  const [caseSensitive, setCaseSensitive] = useState(false);
  const [isExiting, setIsExiting] = useState(false);
  const internalInputRef = React.useRef<HTMLInputElement>(null);
  const inputRef = externalInputRef || internalInputRef;
  const debouncedSearchRef = useRef<ReturnType<typeof debounce>>();

  // Create debounced search function
  useEffect(() => {
    const debouncedFn = debounce((term: string, caseSensitive: boolean) => {
      console.log('Debounced search executing:', { term, caseSensitive });
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
      setDisplayTerm(initialSearchTerm);
      debouncedSearchRef.current?.(initialSearchTerm, caseSensitive);
    }
  }, [initialSearchTerm, caseSensitive, debouncedSearchRef]);

  const [localSearchResults, setLocalSearchResults] = useState<typeof searchResults>(null);

  // Sync external search results with local state
  useEffect(() => {
    // Only set results if we have a search term
    if (!displayTerm) {
      setLocalSearchResults(null);
    } else {
      setLocalSearchResults(searchResults);
    }
  }, [searchResults, searchTerm, displayTerm]);

  const handleSearch = (event: React.ChangeEvent<HTMLInputElement>) => {
    const value = event.target.value;

    // Always cancel pending searches first
    if (debouncedSearchRef.current) {
      debouncedSearchRef.current.cancel();
    }

    // If clearing or empty, handle immediately
    if (!value || !value.trim()) {
      console.log('Clearing search - empty value');
      setDisplayTerm('');
      setSearchTerm('');
      setLocalSearchResults(null);
      onSearch('', caseSensitive);
      return;
    }

    setDisplayTerm(value);
    setSearchTerm(value);
    debouncedSearchRef.current?.(value, caseSensitive);
  };

  const clearSearch = useCallback(() => {
    console.log('Clearing search - clear button');
    if (debouncedSearchRef.current) {
      debouncedSearchRef.current.cancel();
    }
    setDisplayTerm('');
    setSearchTerm('');
    setLocalSearchResults(null);
    onSearch('', caseSensitive);
    inputRef.current?.focus();
  }, [caseSensitive, inputRef, onSearch]);

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
              value={displayTerm}
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
              {displayTerm && (
                <button
                  onClick={clearSearch}
                  className="flex items-center text-textSubtleInverse hover:text-textStandardInverse"
                  title="Clear search"
                >
                  <XCircle className="h-4 w-4 mt-[2px]" />
                </button>
              )}
              <div className="w-16 text-right text-sm text-textStandardInverse flex items-center justify-end">
                {(() => {
                  return localSearchResults?.count > 0 && displayTerm
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
            onClick={(e) => {
              if (searchResults && searchResults.count > 0) {
                handleNavigate('prev', e);
              }
            }}
            className={`p-1 ${
              !searchResults || searchResults.count === 0
                ? 'text-textSubtleInverse/50 cursor-not-allowed'
                : 'text-textSubtleInverse hover:text-textStandardInverse cursor-pointer'
            }`}
            title="Previous (↑)"
          >
            <ArrowUp className="h-5 w-5" />
          </button>
          <button
            onClick={(e) => {
              if (searchResults && searchResults.count > 0) {
                handleNavigate('next', e);
              }
            }}
            className={`p-1 ${
              !searchResults || searchResults.count === 0
                ? 'text-textSubtleInverse/50 cursor-not-allowed'
                : 'text-textSubtleInverse hover:text-textStandardInverse cursor-pointer'
            }`}
            title="Next (↓ or Enter)"
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

export default SearchBar;
