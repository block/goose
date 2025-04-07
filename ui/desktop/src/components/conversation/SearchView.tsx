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
  const [searchResults, setSearchResults] = useState<{
    currentIndex: number;
    count: number;
  } | null>(null);

  const highlighterRef = React.useRef<SearchHighlighter | null>(null);
  const containerRef = React.useRef<HTMLDivElement | null>(null);
  const lastSearchRef = React.useRef<{ term: string; caseSensitive: boolean }>({
    term: '',
    caseSensitive: false,
  });

  // Create debounced highlight function
  const debouncedHighlight = useCallback(
    debounce((term: string, caseSensitive: boolean, highlighter: SearchHighlighter) => {
      const highlights = highlighter.highlight(term, caseSensitive);
      const count = highlights.length;

      if (count > 0) {
        setSearchResults({
          currentIndex: 1,
          count,
        });
        highlighter.setCurrentMatch(0, true); // Explicitly scroll when setting initial match
      } else {
        setSearchResults(null);
      }
    }, 150),
    []
  );

  // Clean up highlighter and debounced functions on unmount
  useEffect(() => {
    return () => {
      if (highlighterRef.current) {
        highlighterRef.current.destroy();
        highlighterRef.current = null;
      }
      debouncedHighlight.cancel();
    };
  }, []);

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
  };

  /**
   * Navigates between search results in the specified direction.
   * @param direction - Direction to navigate ('next' or 'prev')
   */
  const navigateResults = (direction: 'next' | 'prev') => {
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
  };

  /**
   * Closes the search interface and cleans up highlights.
   */
  const handleCloseSearch = () => {
    setIsSearchVisible(false);
    setSearchResults(null);
    if (highlighterRef.current) {
      highlighterRef.current.clearHighlights();
    }
    // Cancel any pending highlight operations
    debouncedHighlight.cancel();
  };

  return (
    <div ref={containerRef} className={`search-container ${className}`}>
      {isSearchVisible && (
        <SearchBar
          onSearch={handleSearch}
          onClose={handleCloseSearch}
          onNavigate={navigateResults}
          searchResults={searchResults}
        />
      )}
      {children}
    </div>
  );
};
