/**
 * SearchHighlighter provides overlay-based text search highlighting
 * with support for navigation and scrolling control.
 */
export class SearchHighlighter {
  private readonly container: HTMLElement;
  private readonly overlay: HTMLElement;
  private highlights: HTMLElement[] = [];
  private resizeObserver: ResizeObserver;
  private mutationObserver: MutationObserver;
  private scrollContainer: HTMLElement | null = null;
  private currentTerm: string = '';
  private caseSensitive: boolean = false;
  private scrollHandler: (() => void) | null = null;
  private onMatchesChange?: (count: number) => void;
  private currentMatchIndex: number = -1;
  private isUpdatingPositions: boolean = false;
  private updatePending: boolean = false;
  private shouldScrollToMatch: boolean = false;

  /**
   * Creates a new SearchHighlighter instance.
   * @param container - The root HTML element to search within
   * @param onMatchesChange - Optional callback that receives the count of matches when changed
   */
  constructor(container: HTMLElement, onMatchesChange?: (count: number) => void) {
    this.container = container;
    this.onMatchesChange = onMatchesChange;

    // Create overlay
    this.overlay = document.createElement('div');
    this.overlay.className = 'search-highlight-overlay';
    this.overlay.style.cssText = `
      position: absolute;
      pointer-events: none;
      top: 0;
      left: 0;
      right: 0;
      bottom: 0;
      z-index: 1;
    `;

    // Find scroll container (usually the radix scroll area viewport)
    this.scrollContainer = container.closest('[data-radix-scroll-area-viewport]');
    if (this.scrollContainer) {
      this.scrollContainer.style.position = 'relative';
      this.scrollContainer.appendChild(this.overlay);

      // Add scroll handler with debouncing to prevent performance issues
      this.scrollHandler = () => {
        if (this.isUpdatingPositions) {
          this.updatePending = true;
          return;
        }

        this.isUpdatingPositions = true;
        requestAnimationFrame(() => {
          this.updateHighlightPositions();
          this.isUpdatingPositions = false;

          if (this.updatePending) {
            this.updatePending = false;
            this.scrollHandler?.();
          }
        });
      };
      this.scrollContainer.addEventListener('scroll', this.scrollHandler);
    } else {
      container.style.position = 'relative';
      container.appendChild(this.overlay);
    }

    // Handle content changes with debouncing
    this.resizeObserver = new ResizeObserver(() => {
      if (this.highlights.length > 0) {
        if (this.isUpdatingPositions) {
          this.updatePending = true;
          return;
        }

        this.isUpdatingPositions = true;
        requestAnimationFrame(() => {
          this.updateHighlightPositions();
          this.isUpdatingPositions = false;

          if (this.updatePending) {
            this.updatePending = false;
            // Re-run the update
            requestAnimationFrame(() => this.updateHighlightPositions());
          }
        });
      }
    });
    this.resizeObserver.observe(container);

    // Watch for DOM changes (new messages)
    this.mutationObserver = new MutationObserver((mutations) => {
      let shouldUpdate = false;
      for (const mutation of mutations) {
        if (mutation.type === 'childList' && mutation.addedNodes.length > 0) {
          shouldUpdate = true;
          break;
        }
      }
      if (shouldUpdate && this.currentTerm) {
        if (this.isUpdatingPositions) {
          this.updatePending = true;
          return;
        }

        this.isUpdatingPositions = true;
        requestAnimationFrame(() => {
          this.highlight(this.currentTerm, this.caseSensitive);
          this.isUpdatingPositions = false;

          if (this.updatePending) {
            this.updatePending = false;
            // Re-run the update
            requestAnimationFrame(() => this.highlight(this.currentTerm, this.caseSensitive));
          }
        });
      }
    });
    this.mutationObserver.observe(container, { childList: true, subtree: true });
  }

  /**
   * Highlights all occurrences of a search term within the container.
   * @param term - The text to search for
   * @param caseSensitive - Whether to perform a case-sensitive search
   * @returns Array of highlight elements created
   */
  highlight(term: string, caseSensitive = false) {
    // Store the current match index before clearing
    const currentIndex = this.currentMatchIndex;

    this.clearHighlights();
    this.currentTerm = term;
    this.caseSensitive = caseSensitive;

    if (!term.trim()) return [];

    const range = document.createRange();
    const regex = new RegExp(
      term.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'),
      caseSensitive ? 'g' : 'gi'
    );

    // Find all text nodes in the container
    const walker = document.createTreeWalker(this.container, NodeFilter.SHOW_TEXT, {
      acceptNode: (node) => {
        // Skip search UI elements
        const parent = node.parentElement;
        if (parent?.closest('.search-bar, .search-results')) {
          return NodeFilter.FILTER_REJECT;
        }
        return NodeFilter.FILTER_ACCEPT;
      },
    });

    const matches: { node: Text; startOffset: number; endOffset: number }[] = [];
    let node: Text | null;

    // Find all matches
    while ((node = walker.nextNode() as Text)) {
      const text = node.textContent || '';
      let match;

      // Reset lastIndex to ensure we find all matches
      regex.lastIndex = 0;
      while ((match = regex.exec(text)) !== null) {
        matches.push({
          node,
          startOffset: match.index,
          endOffset: match.index + match[0].length,
        });
      }
    }

    // Create highlight elements
    this.highlights = matches.map(({ node, startOffset, endOffset }) => {
      range.setStart(node, startOffset);
      range.setEnd(node, endOffset);

      const rects = range.getClientRects();
      const highlight = document.createElement('div');
      highlight.className = 'search-highlight-container';

      // Handle multi-line highlights
      Array.from(rects).forEach((rect) => {
        const highlightRect = document.createElement('div');
        highlightRect.className = 'search-highlight';

        const scrollTop = this.scrollContainer ? this.scrollContainer.scrollTop : window.scrollY;
        const scrollLeft = this.scrollContainer ? this.scrollContainer.scrollLeft : window.scrollX;
        const containerTop = this.scrollContainer?.getBoundingClientRect().top || 0;
        const containerLeft = this.scrollContainer?.getBoundingClientRect().left || 0;

        highlightRect.style.cssText = `
          position: absolute;
          pointer-events: none;
          top: ${rect.top + scrollTop - containerTop}px;
          left: ${rect.left + scrollLeft - containerLeft}px;
          width: ${rect.width}px;
          height: ${rect.height}px;
        `;
        highlight.appendChild(highlightRect);
      });

      this.overlay.appendChild(highlight);
      return highlight;
    });

    // Notify about updated match count
    this.onMatchesChange?.(this.highlights.length);

    // Restore current match if it was set
    if (currentIndex >= 0 && this.highlights.length > 0) {
      // Use the stored index, not this.currentMatchIndex which was reset in clearHighlights
      this.setCurrentMatch(currentIndex, false); // Don't scroll when restoring highlight
    }

    return this.highlights;
  }

  /**
   * Sets the current match and optionally scrolls to it.
   * @param index - Zero-based index of the match to set as current
   * @param shouldScroll - Whether to scroll to the match (true for explicit navigation)
   */
  setCurrentMatch(index: number, shouldScroll = true) {
    // Store the current match index
    this.currentMatchIndex = index;

    // Save the scroll flag
    this.shouldScrollToMatch = shouldScroll;

    // Remove current class from all highlights
    this.overlay.querySelectorAll('.search-highlight').forEach((el) => {
      el.classList.remove('current');
    });

    // Add current class to the matched highlight
    if (this.highlights.length > 0) {
      // Ensure index wraps around
      const wrappedIndex =
        ((index % this.highlights.length) + this.highlights.length) % this.highlights.length;

      // Find all highlight elements within the current highlight container
      const highlightElements = this.highlights[wrappedIndex].querySelectorAll('.search-highlight');

      // Add 'current' class to all parts of the highlight (for multi-line matches)
      highlightElements.forEach((el) => {
        el.classList.add('current');
      });

      // Only scroll if explicitly requested (e.g., when navigating)
      if (shouldScroll) {
        // Ensure we call scrollToMatch with the correct index
        setTimeout(() => this.scrollToMatch(wrappedIndex), 0);
      }
    }
  }

  /**
   * Scrolls to center the specified match in the viewport.
   * @param index - Zero-based index of the match to scroll to
   */
  private scrollToMatch(index: number) {
    if (!this.scrollContainer || !this.highlights[index]) return;

    const currentHighlight = this.highlights[index].querySelector(
      '.search-highlight'
    ) as HTMLElement;
    if (!currentHighlight) return;

    const rect = currentHighlight.getBoundingClientRect();
    const containerRect = this.scrollContainer.getBoundingClientRect();

    // Calculate how far the element is from the top of the viewport
    const elementRelativeToViewport = rect.top - containerRect.top;

    // Calculate the new scroll position that would center the element
    const currentScrollTop = this.scrollContainer.scrollTop;
    const targetPosition =
      currentScrollTop + elementRelativeToViewport - (containerRect.height - rect.height) / 2;

    // Ensure we don't scroll past the bottom
    const maxScroll = this.scrollContainer.scrollHeight - this.scrollContainer.clientHeight;
    const finalPosition = Math.max(0, Math.min(targetPosition, maxScroll));

    this.scrollContainer.scrollTo({
      top: finalPosition,
      behavior: 'smooth',
    });
  }

  /**
   * Updates the positions of all highlights after content changes.
   * This preserves the current match selection but doesn't scroll.
   */
  private updateHighlightPositions() {
    if (this.currentTerm) {
      // Store the current index for restoration
      const currentIndex = this.currentMatchIndex;

      // Clear and recreate all highlights
      this.overlay.innerHTML = '';
      this.highlights = [];

      // Re-highlight with the current term
      this.highlight(this.currentTerm, this.caseSensitive);

      // Ensure the current match is still highlighted, but don't scroll
      if (currentIndex >= 0 && this.highlights.length > 0) {
        this.setCurrentMatch(currentIndex, false);
      }
    }
  }

  /**
   * Removes all search highlights from the container.
   */
  clearHighlights() {
    this.highlights.forEach((h) => h.remove());
    this.highlights = [];
    this.currentTerm = '';
    this.currentMatchIndex = -1;
    this.shouldScrollToMatch = false;
    this.overlay.innerHTML = ''; // Ensure all highlights are removed
  }

  /**
   * Cleans up all resources used by the highlighter.
   * Should be called when the component using this highlighter unmounts.
   */
  destroy() {
    this.resizeObserver.disconnect();
    this.mutationObserver.disconnect();
    if (this.scrollHandler && this.scrollContainer) {
      this.scrollContainer.removeEventListener('scroll', this.scrollHandler);
    }
    this.overlay.remove();
  }
}
