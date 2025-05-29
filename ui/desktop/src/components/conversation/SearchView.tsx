import React, { useState, useEffect, PropsWithChildren, useCallback } from 'react';
import SearchBar from './SearchBar';
import { SearchHighlighter } from '../../utils/searchHighlighter';
import { debounce } from 'lodash';
import '../../styles/search.css';

/**
 * Props for the SearchView component
 */
interface SearchViewProps {
  /** Optional CSS class name */
  className?: string;
  /** Optional callback for search term changes */
  onSearch?: (term: string, caseSensitive: boolean) => void;
  /** Optional callback for navigating between search results */
  onNavigate?: (direction: 'next' | 'prev') => void;
  /** Current search results state */
  searchResults?: {
    count: number;
    currentIndex: number;
  } | null;
}

interface SearchContainerElement extends HTMLDivElement {
  _searchHighlighter: SearchHighlighter | null;
}

/**
 * SearchView wraps content in a searchable container with a search bar that appears
 * when Cmd/Ctrl+F is pressed. Supports case-sensitive search and result navigation.
 * Features debounced search for better performance with large content.
 */
export const SearchView: React.FC<PropsWithChildren<SearchViewProps>> = ({
  className = '',
  children,
  onSearch,
  onNavigate,
  searchResults,
}) => {
  const [isSearchVisible, setIsSearchVisible] = useState(false);
  const [initialSearchTerm, setInitialSearchTerm] = useState('');
  const [internalSearchResults, setInternalSearchResults] = useState<{
    currentIndex: number;
    count: number;
  } | null>(null);

  const searchInputRef = React.useRef<HTMLInputElement>(null);
  const highlighterRef = React.useRef<SearchHighlighter | null>(null);
  const containerRef = React.useRef<SearchContainerElement | null>(null);
  const lastSearchRef = React.useRef<{ term: string; caseSensitive: boolean }>({
    term: '',
    caseSensitive: false,
  });

  // Create debounced highlight function
  const debouncedHighlight = useCallback(
    (term: string, caseSensitive: boolean, highlighter: SearchHighlighter) => {
      const performHighlight = () => {
        const highlights = highlighter.highlight(term, caseSensitive);
        const count = highlights.length;

        if (count > 0) {
          setInternalSearchResults({
            currentIndex: 1,
            count,
          });
          highlighter.setCurrentMatch(0, true);
        } else {
          setInternalSearchResults(null);
        }
      };

      // If this is a case sensitivity change (same term, different case setting),
      // execute immediately
      if (
        term === lastSearchRef.current.term &&
        caseSensitive !== lastSearchRef.current.caseSensitive
      ) {
        performHighlight();
        return;
      }

      // Create a debounced version of performHighlight
      const debouncedFn = debounce(performHighlight, 150);
      debouncedFn();

      // Store the debounced function for potential cancellation
      return debouncedFn;
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
      const isCaseChange =
        term === lastSearchRef.current.term &&
        caseSensitive !== lastSearchRef.current.caseSensitive;

      lastSearchRef.current = { term, caseSensitive };

      // Call the onSearch callback if provided
      onSearch?.(term, caseSensitive);

      // If empty, clear everything and return
      if (!term) {
        setInternalSearchResults(null);
        if (highlighterRef.current) {
          highlighterRef.current.clearHighlights();
        }
        return;
      }

      const container = containerRef.current;
      if (!container) return;

      // For case sensitivity changes, reuse existing highlighter
      if (isCaseChange && highlighterRef.current) {
        debouncedHighlight(term, caseSensitive, highlighterRef.current);
        return;
      }

      // Otherwise create new highlighter
      if (highlighterRef.current) {
        highlighterRef.current.clearHighlights();
        highlighterRef.current.destroy();
      }

      highlighterRef.current = new SearchHighlighter(container, (count) => {
        // Only update if this is still the latest search
        if (
          lastSearchRef.current.term === term &&
          lastSearchRef.current.caseSensitive === caseSensitive
        ) {
          if (count > 0) {
            setInternalSearchResults({
              currentIndex: 1,
              count,
            });
          } else {
            setInternalSearchResults(null);
          }
        }
      });

      debouncedHighlight(term, caseSensitive, highlighterRef.current);
    },
    [debouncedHighlight, onSearch]
  );

  /**
   * Navigates between search results in the specified direction.
   * @param direction - Direction to navigate ('next' or 'prev')
   */
  const handleNavigate = useCallback(
    (direction: 'next' | 'prev') => {
      // If external navigation is provided, use that
      if (onNavigate) {
        onNavigate(direction);
        return;
      }

      // Otherwise use internal navigation
      if (!internalSearchResults || !highlighterRef.current) return;

      let newIndex: number;
      if (direction === 'next') {
        newIndex = (internalSearchResults.currentIndex % internalSearchResults.count) + 1;
      } else {
        newIndex =
          internalSearchResults.currentIndex === 1
            ? internalSearchResults.count
            : internalSearchResults.currentIndex - 1;
      }

      setInternalSearchResults({
        ...internalSearchResults,
        currentIndex: newIndex,
      });

      highlighterRef.current.setCurrentMatch(newIndex - 1, true);
    },
    [internalSearchResults, onNavigate]
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
      handleNavigate('next');
    }
  }, [isSearchVisible, handleNavigate]);

  const handleFindPrevious = useCallback(() => {
    if (isSearchVisible) {
      handleNavigate('prev');
    }
  }, [isSearchVisible, handleNavigate]);

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
    setInternalSearchResults(null);
    lastSearchRef.current = { term: '', caseSensitive: false };

    if (highlighterRef.current) {
      highlighterRef.current.clearHighlights();
      highlighterRef.current.destroy();
      highlighterRef.current = null;
    }

    // Clear search when closing
    onSearch?.('', false);
  }, [onSearch]);

  // Clean up highlighter on unmount
  useEffect(() => {
    return () => {
      if (highlighterRef.current) {
        highlighterRef.current.destroy();
        highlighterRef.current = null;
      }
    };
  }, []);

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
          handleNavigate('prev');
        } else {
          // ⌘G - Find Next
          handleNavigate('next');
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [isSearchVisible, handleNavigate, handleSearch, handleUseSelectionFind]);

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
    <div
      ref={(el) => {
        if (el) {
          containerRef.current = el;
          // Expose the highlighter instance
          containerRef.current._searchHighlighter = highlighterRef.current;
        }
      }}
      className={`search-container ${className}`}
    >
      {isSearchVisible && (
        <SearchBar
          onSearch={handleSearch}
          onClose={handleCloseSearch}
          onNavigate={handleNavigate}
          searchResults={searchResults || internalSearchResults}
          inputRef={searchInputRef}
          initialSearchTerm={initialSearchTerm}
        />
      )}
      {children}
    </div>
  );
};
