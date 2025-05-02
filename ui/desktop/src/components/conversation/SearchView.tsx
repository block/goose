import React, { useState, useEffect, PropsWithChildren, useCallback } from 'react';
import { SearchBar } from './SearchBar';
import { SearchHighlighter } from '../../utils/searchHighlighter';
import { debounce } from 'lodash';
import '../../styles/search.css';

/**
 * Props for the SearchView component
 */
interface SearchViewProps {
  /** Optional CSS class name */
  className?: string;
}

/**
 * SearchView wraps content in a searchable container with a search bar that appears
 * when Cmd/Ctrl+F is pressed. Supports case-sensitive search and result navigation.
 * Features debounced search for better performance with large content.
 */
export const SearchView: React.FC<PropsWithChildren<SearchViewProps>> = ({
  className = '',
  children,
}) => {
  const [isSearchVisible, setIsSearchVisible] = useState(false);
  const [initialSearchTerm, setInitialSearchTerm] = useState('');
  const [searchResults, setSearchResults] = useState<{
    currentIndex: number;
    count: number;
  } | null>(null);

  const searchInputRef = React.useRef<HTMLInputElement>(null);
  const highlighterRef = React.useRef<SearchHighlighter | null>(null);
  const containerRef = React.useRef<HTMLDivElement | null>(null);
  const lastSearchRef = React.useRef<{ term: string; caseSensitive: boolean }>({
    term: '',
    caseSensitive: false,
  });

  // Create debounced highlight function
  const debouncedHighlight = useCallback(
    (term: string, caseSensitive: boolean, highlighter: SearchHighlighter) => {
      debounce(
        (searchTerm: string, isCaseSensitive: boolean, searchHighlighter: SearchHighlighter) => {
          const highlights = searchHighlighter.highlight(searchTerm, isCaseSensitive);
          const count = highlights.length;

          if (count > 0) {
            setSearchResults({
              currentIndex: 1,
              count,
            });
            searchHighlighter.setCurrentMatch(0, true); // Explicitly scroll when setting initial match
          } else {
            setSearchResults(null);
          }
        },
        150
      )(term, caseSensitive, highlighter);
    },
    []
  );

  /**
   * Handles the search operation when a user enters a search term.
   * Uses debouncing to prevent excessive highlighting operations.
   * @param term - The text to search for
   * @param caseSensitive - Whether to perform a case-sensitive search
   */
  const handleSearch = useCallback(
    (term: string, caseSensitive: boolean) => {
      // Store the latest search parameters
      lastSearchRef.current = { term, caseSensitive };

      if (!term) {
        setSearchResults(null);
        if (highlighterRef.current) {
          highlighterRef.current.clearHighlights();
        }
        return;
      }

      const container = containerRef.current;
      if (!container) return;

      if (!highlighterRef.current) {
        highlighterRef.current = new SearchHighlighter(container, (count) => {
          // Only update if this is still the latest search
          if (
            lastSearchRef.current.term === term &&
            lastSearchRef.current.caseSensitive === caseSensitive
          ) {
            if (count > 0) {
              setSearchResults((prev) => ({
                currentIndex: prev?.currentIndex || 1,
                count,
              }));
            } else {
              setSearchResults(null);
            }
          }
        });
      }

      // Debounce the highlight operation
      debouncedHighlight(term, caseSensitive, highlighterRef.current);
    },
    [debouncedHighlight]
  );

  /**
   * Navigates between search results in the specified direction.
   * @param direction - Direction to navigate ('next' or 'prev')
   */
  const navigateResults = useCallback(
    (direction: 'next' | 'prev') => {
      if (!searchResults || searchResults.count === 0 || !highlighterRef.current) return;

      let newIndex: number;
      const currentIdx = searchResults.currentIndex - 1; // Convert to 0-based

      if (direction === 'next') {
        newIndex = currentIdx + 1;
        if (newIndex >= searchResults.count) {
          newIndex = 0;
        }
      } else {
        newIndex = currentIdx - 1;
        if (newIndex < 0) {
          newIndex = searchResults.count - 1;
        }
      }

      setSearchResults({
        ...searchResults,
        currentIndex: newIndex + 1,
      });

      highlighterRef.current.setCurrentMatch(newIndex, true); // Explicitly scroll when navigating
    },
    [searchResults]
  );

  const handleFindCommand = useCallback(() => {
    if (isSearchVisible && searchInputRef.current) {
      searchInputRef.current.focus();
      searchInputRef.current.select();
    } else {
      setIsSearchVisible(true);
    }
  }, [isSearchVisible]);

  const handleFindNext = useCallback(() => {
    if (isSearchVisible) {
      navigateResults('next');
    }
  }, [isSearchVisible, navigateResults]);

  const handleFindPrevious = useCallback(() => {
    if (isSearchVisible) {
      navigateResults('prev');
    }
  }, [isSearchVisible, navigateResults]);

  const handleUseSelectionFind = useCallback(() => {
    const selection = window.getSelection()?.toString().trim();
    if (selection) {
      setInitialSearchTerm(selection);
    }
  }, []);

  /**
   * Closes the search interface and cleans up highlights.
   */
  const handleCloseSearch = useCallback(() => {
    setIsSearchVisible(false);
    setSearchResults(null);
    if (highlighterRef.current) {
      highlighterRef.current.clearHighlights();
    }
    // Cancel any pending highlight operations
    debouncedHighlight.cancel?.();
  }, [debouncedHighlight]);

  // Clean up highlighter and debounced functions on unmount
  useEffect(() => {
    return () => {
      if (highlighterRef.current) {
        highlighterRef.current.destroy();
        highlighterRef.current = null;
      }
      debouncedHighlight.cancel?.();
    };
  }, [debouncedHighlight]);

  // Listen for keyboard events
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const isMac = window.electron.platform === 'darwin';

      // Handle ⌘F/Ctrl+F to show/focus search
      if ((isMac ? e.metaKey : e.ctrlKey) && !e.shiftKey && e.key === 'f') {
        e.preventDefault();
        if (isSearchVisible && searchInputRef.current) {
          // If search is already visible, focus and select the input
          searchInputRef.current.focus();
          searchInputRef.current.select();
        } else {
          // Otherwise show the search UI
          setIsSearchVisible(true);
        }
        return;
      }

      // Handle ⌘E to use selection for find (Mac only)
      if (isMac && e.metaKey && !e.shiftKey && e.key === 'e') {
        // Don't handle ⌘E if we're in the search input - let the native behavior work
        if (e.target instanceof HTMLInputElement && e.target.id === 'search-input') {
          return;
        }

        e.preventDefault();
        handleUseSelectionFind();
        return;
      }

      // Only handle ⌘G and ⇧⌘G if search is visible (Mac only)
      if (isSearchVisible && isMac && e.metaKey && e.key === 'g') {
        e.preventDefault();
        if (e.shiftKey) {
          // ⇧⌘G - Find Previous
          navigateResults('prev');
        } else {
          // ⌘G - Find Next
          navigateResults('next');
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [isSearchVisible, navigateResults, handleSearch, handleUseSelectionFind]);

  // Listen for Find menu commands
  useEffect(() => {
    window.electron.on('find-command', handleFindCommand);
    window.electron.on('find-next', handleFindNext);
    window.electron.on('find-previous', handleFindPrevious);
    window.electron.on('use-selection-find', handleUseSelectionFind);

    return () => {
      window.electron.off('find-command', handleFindCommand);
      window.electron.off('find-next', handleFindNext);
      window.electron.off('find-previous', handleFindPrevious);
      window.electron.off('use-selection-find', handleUseSelectionFind);
    };
  }, [handleFindCommand, handleFindNext, handleFindPrevious, handleUseSelectionFind]);

  return (
    <div ref={containerRef} className={`search-container ${className}`}>
      {isSearchVisible && (
        <SearchBar
          onSearch={handleSearch}
          onClose={handleCloseSearch}
          onNavigate={navigateResults}
          searchResults={searchResults}
          inputRef={searchInputRef}
          initialSearchTerm={initialSearchTerm}
        />
      )}
      {children}
    </div>
  );
};
