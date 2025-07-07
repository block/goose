/**
 * @module useWindowManager
 * @description
 * The `useWindowManager` hook provides functionality for dynamically resizing the application window 
 * and managing component mounting states based on window expansion/collapse operations.
 * 
 * @example
 * ```typescript
 * import { useWindowManager } from '../hooks/useWindowManager';
 * 
 * function MyComponent() {
 *   const { windowState, toggleWindow, isComponentMounted, canExpand } = useWindowManager({
 *     expandPercentage: 50,
 *     maxWidthForExpansion: 900,
 *     transitionDuration: 300
 *   });
 * 
 *   return (
 *     <div>
 *       <button onClick={toggleWindow} disabled={!canExpand}>
 *         {windowState.isExpanded ? 'Collapse' : 'Expand'} Window
 *       </button>
 *       
 *       {isComponentMounted && (
 *         <div>Content that appears when window is expanded</div>
 *       )}
 *     </div>
 *   );
 * }
 * ```
 * 
 * @property {object} windowState - Current window state information.
 * @property {boolean} windowState.isExpanded - Whether window is currently expanded.
 * @property {number} windowState.originalWidth - Original window width before expansion.
 * @property {number} windowState.currentWidth - Current window width.
 * @property {boolean} windowState.isTransitioning - Whether a resize operation is in progress.
 * 
 * @property {Function} toggleWindow - Async function to toggle window expansion/collapse.
 * 
 * @property {boolean} isComponentMounted - Boolean indicating if components should be rendered.
 * `true` when window is expanded and transition is complete, `false` otherwise.
 * 
 * @property {boolean} canExpand - Boolean indicating if window can be expanded based on `maxWidthForExpansion`.
 *
 * @see useWindowManager
 */
import { useState, useEffect, useCallback, useRef } from 'react';

export interface WindowState {
  isExpanded: boolean;
  originalWidth: number;
  currentWidth: number;
  isTransitioning: boolean;
}

export interface WindowManagerOptions {
  expandPercentage?: number;
  transitionDuration?: number;
  maxWidthForExpansion?: number;
}

export interface WindowManagerHook {
  windowState: WindowState;
  toggleWindow: () => Promise<void>;
  isComponentMounted: boolean;
  canExpand: boolean;
}

const DEFAULT_OPTIONS: Required<WindowManagerOptions> = {
  expandPercentage: 50,
  transitionDuration: 300,
  maxWidthForExpansion: 900,
};

export function useWindowManager(options: WindowManagerOptions = {}): WindowManagerHook {
  const opts = { ...DEFAULT_OPTIONS, ...options };

  // Use ref to track if we're in the middle of a resize operation
  const resizeInProgressRef = useRef(false);

  const [windowState, setWindowState] = useState<WindowState>({
    isExpanded: false,
    originalWidth: window.innerWidth,
    currentWidth: window.innerWidth,
    isTransitioning: false,
  });

  const [isComponentMounted, setIsComponentMounted] = useState(false);

  // Determine if window can be expanded based on current width
  const canExpand = windowState.currentWidth <= opts.maxWidthForExpansion;

  // Update window dimensions when window is resized externally
  useEffect(() => {
    const handleResize = () => {
      setWindowState((prev) => {
        const newWidth = window.innerWidth;

        // Always update current width to match actual window
        const updatedState = {
          ...prev,
          currentWidth: newWidth,
        };

        // If we're in the middle of a programmatic resize, don't change expanded state
        if (resizeInProgressRef.current) {
          return updatedState;
        }

        // Check if window was manually resized to a smaller size
        // We consider it manually collapsed if:
        // 1. It was previously expanded, AND
        // 2. The new width is significantly smaller than what we expect for expanded state
        if (prev.isExpanded) {
          // Calculate what the expanded width should be based on original width
          const expectedExpandedWidth = Math.floor(
            prev.originalWidth * (1 + opts.expandPercentage / 100)
          );
          const collapseThreshold = expectedExpandedWidth * 0.85; // 85% threshold for more reliable detection

          if (newWidth < collapseThreshold) {
            console.log('Window manually collapsed - resetting expanded state', {
              originalWidth: prev.originalWidth,
              expectedExpandedWidth,
              newWidth,
              threshold: collapseThreshold,
            });

            return {
              ...updatedState,
              isExpanded: false,
              originalWidth: newWidth, // Update original width to new smaller size
            };
          }
        }

        // If not expanded, update original width to track manual resizing
        if (!prev.isExpanded) {
          return {
            ...updatedState,
            originalWidth: newWidth,
          };
        }

        return updatedState;
      });
    };

    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, [opts.expandPercentage]);

  // Manage component mounting based on window state
  useEffect(() => {
    if (windowState.isExpanded && !windowState.isTransitioning) {
      // Mount component after expansion is complete
      const timer = window.setTimeout(() => {
        setIsComponentMounted(true);
      }, 50); // Small delay to ensure window resize is complete

      return () => {
        window.clearTimeout(timer);
      };
    } else if (!windowState.isExpanded) {
      // Unmount component immediately when collapsing
      setIsComponentMounted(false);
    }

    // Return undefined for cases where no cleanup is needed
    return undefined;
  }, [windowState.isExpanded, windowState.isTransitioning]);

  const toggleWindow = useCallback(async (): Promise<void> => {
    // Prevent multiple simultaneous resize operations
    if (resizeInProgressRef.current) {
      console.log('Resize already in progress, ignoring toggle request');
      return;
    }

    // Don't resize if window is already expanded
    if (windowState.isExpanded) {
      console.log('Window is already expanded, skipping resize operation');
      return;
    }

    try {
      resizeInProgressRef.current = true;

      setWindowState((prev) => ({
        ...prev,
        isTransitioning: true,
      }));

      // Only expanding logic remains since we skip when already expanded
      if (!canExpand) {
        console.log('Window too wide for expansion, skipping resize');
        setWindowState((prev) => ({
          ...prev,
          isTransitioning: false,
        }));
        // Still mount the component even if we don't resize
        setIsComponentMounted(true);
        return;
      }

      const success = await window.electron.resizeWindow(opts.expandPercentage);

      if (success) {
        const newWidth = Math.floor(windowState.currentWidth * (1 + opts.expandPercentage / 100));

        setWindowState((prev) => ({
          ...prev,
          isExpanded: true,
          currentWidth: newWidth,
          isTransitioning: false,
        }));

        // Component will be mounted by the useEffect above
      } else {
        throw new Error('Failed to resize window for expansion');
      }
    } catch (error) {
      console.error('Error during window toggle:', error);

      // Reset state on error
      setWindowState((prev) => ({
        ...prev,
        isTransitioning: false,
      }));

      // Ensure component state is consistent
      setIsComponentMounted(windowState.isExpanded);
    } finally {
      resizeInProgressRef.current = false;
    }
  }, [windowState.isExpanded, windowState.currentWidth, canExpand, opts.expandPercentage]);

  return {
    windowState,
    toggleWindow,
    isComponentMounted,
    canExpand,
  };
}
