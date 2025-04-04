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

  // temporarily disabling search for launch until issue https://github.com/block/goose/issues/1984 is resolved
  //
  // useEffect(() => {
  //   const handleKeyDown = (e: KeyboardEvent) => {
  //     if ((e.metaKey || e.ctrlKey) && e.key === 'f') {
  //       e.preventDefault();
  //       setIsSearchVisible(true);
  //     }
  //   };
  //
  //   window.addEventListener('keydown', handleKeyDown);
  //   return () => {
  //     window.removeEventListener('keydown', handleKeyDown);
  //   };
  // }, []);

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
    if (!mark) return;

    // Update highlight
    marks.forEach((m) => m.classList.remove('current'));
    mark.classList.add('current');

    // Find the viewport element
    const viewport = mark.closest('[data-radix-scroll-area-viewport]') as HTMLElement;
    if (!viewport) return;

    // Get measurements
    const viewportRect = viewport.getBoundingClientRect();
    const markRect = mark.getBoundingClientRect();
    const currentScrollTop = viewport.scrollTop;

    // Calculate how far the element is from the top of the viewport
    const elementRelativeToViewport = markRect.top - viewportRect.top;

    // Calculate the new scroll position that would center the element
    const targetPosition =
      currentScrollTop + elementRelativeToViewport - (viewportRect.height - markRect.height) / 2;

    // Ensure we don't scroll past the bottom
    const maxScroll = viewport.scrollHeight - viewport.clientHeight;
    const finalPosition = Math.max(0, Math.min(targetPosition, maxScroll));

    // Use requestAnimationFrame to ensure DOM measurements are accurate
    requestAnimationFrame(() => {
      scrollAreaRef.current?.scrollToPosition({
        top: finalPosition,
        behavior: 'smooth',
      });
    });
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
