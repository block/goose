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
  private onMatchesChange?: (count: number) => void;
  private currentMatchIndex: number = -1;

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

    // Find scroll container (look for our custom data attribute first, then fallback to radix)
    this.scrollContainer =
      container
        .closest('[data-search-scroll-area]')
        ?.querySelector('[data-radix-scroll-area-viewport]') ||
      container.closest('[data-radix-scroll-area-viewport]');

    if (this.scrollContainer) {
      this.scrollContainer.style.position = 'relative';
      this.scrollContainer.appendChild(this.overlay);
    } else {
      container.style.position = 'relative';
      container.appendChild(this.overlay);
    }

    // Handle content changes
    this.resizeObserver = new ResizeObserver(() => {
      if (this.highlights.length > 0) {
        this.updateHighlightPositions();
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
        this.highlight(this.currentTerm, this.caseSensitive);
      }
    });
    this.mutationObserver.observe(container, { childList: true, subtree: true });
  }

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

        // Get the scroll container's position
        const containerRect = this.scrollContainer?.getBoundingClientRect() || { top: 0, left: 0 };
        const scrollTop = this.scrollContainer?.scrollTop || 0;
        const scrollLeft = this.scrollContainer?.scrollLeft || 0;

        // Calculate the highlight position relative to the scroll container
        const top = rect.top + scrollTop - containerRect.top;
        const left = rect.left + scrollLeft - containerRect.left;

        highlightRect.style.cssText = `
          position: absolute;
          pointer-events: none;
          top: ${top}px;
          left: ${left}px;
          width: ${rect.width}px;
          height: ${rect.height}px;
        `;
        highlight.appendChild(highlightRect);
      });

      // Store the original text node for scrolling
      highlight.dataset.originalNode = node.toString();
      this.overlay.appendChild(highlight);
      return highlight;
    });

    // Notify about updated match count
    this.onMatchesChange?.(this.highlights.length);

    // Restore current match if it exists and matches are found
    if (currentIndex >= 0 && this.highlights.length > 0) {
      // If the current index is beyond the new matches, wrap to the beginning
      const newIndex = currentIndex >= this.highlights.length ? 0 : currentIndex;
      this.setCurrentMatch(newIndex, true);
    }

    return this.highlights;
  }

  setCurrentMatch(index: number, shouldScroll = true) {
    if (!this.highlights.length) return;

    // Ensure index wraps around
    const wrappedIndex =
      ((index % this.highlights.length) + this.highlights.length) % this.highlights.length;

    // Store the current match index
    this.currentMatchIndex = wrappedIndex;

    // Remove current class from all highlights
    this.overlay.querySelectorAll('.search-highlight').forEach((el) => {
      el.classList.remove('current');
    });

    // Add current class to all parts of the highlight
    const currentHighlight = this.highlights[wrappedIndex];
    const highlightElements = currentHighlight.querySelectorAll('.search-highlight');
    highlightElements.forEach((el) => {
      el.classList.add('current');
    });

    // Only scroll if explicitly requested
    if (shouldScroll) {
      // Get the first highlight element of the current match
      const firstHighlight = highlightElements[0] as HTMLElement;
      if (firstHighlight) {
        // Use native scrollIntoView with smooth behavior
        firstHighlight.scrollIntoView({
          behavior: 'auto',
          block: 'center',
          inline: 'nearest',
        });
      }
    }
  }

  private updateHighlightPositions() {
    if (this.currentTerm) {
      const currentIndex = this.currentMatchIndex;
      this.highlight(this.currentTerm, this.caseSensitive);
      if (currentIndex >= 0 && this.highlights.length > 0) {
        this.setCurrentMatch(currentIndex, false);
      }
    }
  }

  clearHighlights() {
    this.highlights.forEach((h) => h.remove());
    this.highlights = [];
    this.currentTerm = '';
    this.currentMatchIndex = -1;
    this.overlay.innerHTML = '';
  }

  destroy() {
    this.resizeObserver.disconnect();
    this.mutationObserver.disconnect();
    this.overlay.remove();
  }
}
