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
  const [internalSearchResults, setInternalSearchResults] = useState<{
    currentIndex: number;
    count: number;
  } | null>(null);

  const highlighterRef = React.useRef<SearchHighlighter | null>(null);
  const containerRef = React.useRef<SearchContainerElement | null>(null);
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
            setInternalSearchResults({
              currentIndex: 1,
              count,
            });
            searchHighlighter.setCurrentMatch(0, true); // Explicitly scroll when setting initial match
          } else {
            setInternalSearchResults(null);
          }
        },
        150
      )(term, caseSensitive, highlighter);
    },
    []
  );

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

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'f') {
        e.preventDefault();
        setIsSearchVisible(true);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, []);

  /**
   * Handles the search operation when a user enters a search term.
   * Uses debouncing to prevent excessive highlighting operations.
   * @param term - The text to search for
   * @param caseSensitive - Whether to perform a case-sensitive search
   */
  const handleSearch = (term: string, caseSensitive: boolean) => {
    // Store the latest search parameters
    lastSearchRef.current = { term, caseSensitive };

    // Always clear internal results first
    setInternalSearchResults(null);

    // Always clear highlights first
    if (highlighterRef.current) {
      highlighterRef.current.clearHighlights();
      highlighterRef.current.destroy();
      highlighterRef.current = null;
    }

    // Call the onSearch callback if provided
    onSearch?.(term, caseSensitive);

    // If empty, we're done
    if (!term) {
      // Ensure we clear any external search results
      onSearch?.('', caseSensitive);
      return;
    }

    const container = containerRef.current;
    if (!container) return;

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

    // Debounce the highlight operation
    debouncedHighlight(term, caseSensitive, highlighterRef.current);
  };

  /**
   * Navigates between search results in the specified direction.
   * @param direction - Direction to navigate ('next' or 'prev')
   */
  const handleNavigate = (direction: 'next' | 'prev') => {
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
  };

  /**
   * Closes the search interface and cleans up highlights.
   */
  const handleCloseSearch = () => {
    setIsSearchVisible(false);
    setInternalSearchResults(null);
    lastSearchRef.current = { term: '', caseSensitive: false };

    if (highlighterRef.current) {
      highlighterRef.current.clearHighlights();
      highlighterRef.current.destroy();
      highlighterRef.current = null;
    }

    // Cancel any pending highlight operations
    debouncedHighlight.cancel?.();

    // Clear search when closing
    onSearch?.('', false);
  };

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
        />
      )}
      {children}
    </div>
  );
};
