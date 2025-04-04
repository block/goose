import React, { useState, useEffect, PropsWithChildren } from 'react';
import { SearchBar } from './SearchBar';
import { SearchHighlighter } from '../../utils/searchHighlighter';
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

  // Clean up highlighter on unmount
  useEffect(() => {
    return () => {
      if (highlighterRef.current) {
        highlighterRef.current.destroy();
        highlighterRef.current = null;
      }
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
   * @param term - The text to search for
   * @param caseSensitive - Whether to perform a case-sensitive search
   */
  const handleSearch = (term: string, caseSensitive: boolean) => {
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
        if (count > 0) {
          setSearchResults((prev) => ({
            currentIndex: prev?.currentIndex || 1,
            count,
          }));
        } else {
          setSearchResults(null);
        }
      });
    }

    const highlights = highlighterRef.current.highlight(term, caseSensitive);
    const count = highlights.length;

    if (count > 0) {
      setSearchResults({
        currentIndex: 1,
        count,
      });
      highlighterRef.current.setCurrentMatch(0, true); // Explicitly scroll when setting initial match
    } else {
      setSearchResults(null);
    }
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
   * Closes the search interface and clears all highlights.
   */
  const handleCloseSearch = () => {
    setIsSearchVisible(false);
    setSearchResults(null);
    if (highlighterRef.current) {
      highlighterRef.current.clearHighlights();
    }
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
