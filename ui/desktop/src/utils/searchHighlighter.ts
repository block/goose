/* eslint-disable no-cond-assign */
/**
 * Utility class for highlighting search matches in text content.
 * Supports case-sensitive search and maintains a "current" highlight state.
 */
export class SearchHighlighter {
  private readonly container: HTMLElement;

  /**
   * Creates a new SearchHighlighter instance.
   * @param container - The root HTML element to search within
   */
  constructor(container: HTMLElement) {
    this.container = container;
  }

  /**
   * Highlights all occurrences of a search term within the container.
   * @param term - The text to search for
   * @param caseSensitive - Whether to perform a case-sensitive search
   */
  highlight(term: string, caseSensitive = false): void {
    this.clearHighlights();

    if (!term.trim()) return;

    const walker = document.createTreeWalker(this.container, window.NodeFilter.SHOW_TEXT, {
      acceptNode: (node: Node): number => {
        // Check if this node or any of its ancestors have the excluded classes
        let element = node.parentElement;
        while (element) {
          if (
            element.classList.contains('search-input') ||
            element.classList.contains('search-results') ||
            element.classList.contains('case-sensitive-btn')
          ) {
            return window.NodeFilter.FILTER_REJECT;
          }
          element = element.parentElement;
        }
        return window.NodeFilter.FILTER_ACCEPT;
      },
    });

    const matches: Node[] = [];
    let node: Node | null;

    // Find all text nodes containing the search term
    while ((node = walker.nextNode())) {
      const nodeText = node.textContent || '';
      const searchText = caseSensitive ? nodeText : nodeText.toLowerCase();
      const searchTerm = caseSensitive ? term : term.toLowerCase();

      if (searchText.includes(searchTerm)) {
        matches.push(node);
      }
    }

    // Highlight matches
    matches.forEach((textNode) => {
      const text = textNode.textContent || '';
      const searchTerm = caseSensitive ? term : term.toLowerCase();
      const regex = new RegExp(
        searchTerm.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'),
        caseSensitive ? 'g' : 'gi'
      );
      const fragment = document.createDocumentFragment();
      let lastIndex = 0;
      let match;

      while ((match = regex.exec(text)) !== null) {
        // Add text before the match
        if (match.index > lastIndex) {
          fragment.appendChild(document.createTextNode(text.slice(lastIndex, match.index)));
        }

        // Add highlighted match
        const mark = document.createElement('mark');
        mark.textContent = match[0]; // Use the actual matched text to preserve case
        fragment.appendChild(mark);

        lastIndex = regex.lastIndex;
      }

      // Add remaining text
      if (lastIndex < text.length) {
        fragment.appendChild(document.createTextNode(text.slice(lastIndex)));
      }

      textNode.parentNode?.replaceChild(fragment, textNode);
    });
  }

  /**
   * Removes all search highlights from the container
   */
  clearHighlights(): void {
    const marks = this.container.getElementsByTagName('mark');
    while (marks.length > 0) {
      const mark = marks[0];
      const parent = mark.parentNode;
      if (parent) {
        parent.replaceChild(document.createTextNode(mark.textContent || ''), mark);
        parent.normalize();
      }
    }
  }
}
