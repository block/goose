import { useEffect, useCallback } from 'react';

/**
 * Custom hook to handle Ctrl+R hotkey for toggling full tool output display
 * This hook adds a global keyboard listener that expands all truncated tool outputs
 */
export function useToggleToolOutput() {
  const toggleAllToolOutputs = useCallback(() => {
    // Find all collapsed tool argument values and expand them
    const collapsedButtons = document.querySelectorAll(
      'button[aria-label="Expand value"], button:has(.expand-icon), button[data-tool-arg-key]'
    );

    // Also find any elements with truncate classes that contain tool arguments
    const truncatedElements = document.querySelectorAll(
      '.truncate button[onClick*="toggleKey"], .truncate.min-w-0 button[onClick*="toggleKey"]'
    );

    // Dispatch click events to expand all collapsed items
    [...collapsedButtons, ...truncatedElements].forEach((element) => {
      if (element instanceof HTMLElement) {
        element.click();
      }
    });

    // Alternative approach: dispatch a custom event that ToolCallArguments can listen for
    const toggleEvent = new CustomEvent('toggleAllToolOutputs', {
      detail: { expand: true }
    });
    document.dispatchEvent(toggleEvent);
  }, []);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Check for Ctrl+R (or Cmd+R on Mac)
      if ((e.ctrlKey || e.metaKey) && e.key === 'r') {
        e.preventDefault(); // Prevent browser refresh
        toggleAllToolOutputs();
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [toggleAllToolOutputs]);
}