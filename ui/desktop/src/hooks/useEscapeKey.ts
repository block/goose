import { useEffect } from 'react';

/**
 * Custom hook to handle Esc key press for closing modals
 * @param isActive - Whether the hook should be active (typically when modal is open)
 * @param onEscape - Callback function to execute when Esc key is pressed
 */
export function useEscapeKey(isActive: boolean, onEscape: () => void) {
  useEffect(() => {
    if (!isActive) return;

    const handleEscKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onEscape();
      }
    };

    document.addEventListener('keydown', handleEscKey);
    return () => {
      document.removeEventListener('keydown', handleEscKey);
    };
  }, [isActive, onEscape]);
}
