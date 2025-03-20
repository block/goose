import React, { useState, useEffect, PropsWithChildren } from 'react';
import { SearchBar } from './SearchBar';
import { SearchHighlighter } from '../../utils/searchHighlighter';
import { ScrollAreaHandle } from '../ui/scroll-area';
import '../../styles/search.css';

/**
 * Props for the SearchView component
 */
interface SearchViewProps {
  /** Optional CSS class name */
  className?: string;
  /** Reference to the scroll area for navigation */
  scrollAreaRef?: React.RefObject<ScrollAreaHandle>;
}

/**
 * SearchView wraps content in a searchable container with a search bar that appears
 * when Cmd/Ctrl+F is pressed. Supports case-sensitive search and result navigation.
 */
export const SearchView: React.FC<PropsWithChildren<SearchViewProps>> = ({
  className = '',
  children,
  scrollAreaRef,
}) => {
  const [isSearchVisible, setIsSearchVisible] = useState(false);
  const [searchResults, setSearchResults] = useState<{
    currentIndex: number;
    count: number;
  } | null>(null);

  const highlighterRef = React.useRef<SearchHighlighter | null>(null);
  const containerRef = React.useRef<HTMLDivElement | null>(null);

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

  const handleSearch = (term: string, caseSensitive: boolean) => {
    if (!term) {
      setSearchResults(null);
      clearHighlights();
      return;
    }

    const container = containerRef.current;
    if (!container) return;

    if (!highlighterRef.current) {
      highlighterRef.current = new SearchHighlighter(container);
    }

    highlighterRef.current.clearHighlights();
    highlighterRef.current.highlight(term, caseSensitive);

    const marks = container.querySelectorAll('mark');
    const count = marks.length;

    if (count > 0) {
      setSearchResults({
        currentIndex: 1,
        count: count,
      });
      scrollToMatch(0);
    } else {
      setSearchResults(null);
    }
  };

  const navigateResults = (direction: 'next' | 'prev') => {
    if (!searchResults || searchResults.count === 0) return;

    let newIndex: number;
    const currentIdx = searchResults.currentIndex - 1; // Convert to 0-based

    if (direction === 'next') {
      newIndex = (currentIdx + 1) % searchResults.count;
    } else {
      newIndex = (currentIdx - 1 + searchResults.count) % searchResults.count;
    }

    setSearchResults({
      ...searchResults,
      currentIndex: newIndex + 1,
    });

    scrollToMatch(newIndex);
  };

  const scrollToMatch = (index: number) => {
    if (!containerRef.current || !scrollAreaRef?.current) return;

    const marks = containerRef.current.querySelectorAll('mark');
    const mark = marks[index] as HTMLElement;

    if (mark) {
      // Update highlight
      marks.forEach((m) => m.classList.remove('current'));
      mark.classList.add('current');

      // Calculate position to center the mark in the viewport
      const markRect = mark.getBoundingClientRect();
      const viewportRect = mark
        .closest('[data-radix-scroll-area-viewport]')
        ?.getBoundingClientRect();

      if (viewportRect) {
        const targetPosition = mark.offsetTop - viewportRect.height / 2 + markRect.height / 2;
        scrollAreaRef.current.scrollToPosition({
          top: targetPosition,
          behavior: 'smooth',
        });
      }
    }
  };

  const clearHighlights = () => {
    if (highlighterRef.current) {
      highlighterRef.current.clearHighlights();
    }
  };

  const handleCloseSearch = () => {
    setIsSearchVisible(false);
    setSearchResults(null);
    clearHighlights();
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
