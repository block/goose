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
      // Only update if we're not in the middle of a programmatic resize
      if (!resizeInProgressRef.current) {
        setWindowState((prev) => ({
          ...prev,
          currentWidth: window.innerWidth,
          // If window was manually resized, update original width when not expanded
          originalWidth: prev.isExpanded ? prev.originalWidth : window.innerWidth,
        }));
      }
    };

    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

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

    try {
      resizeInProgressRef.current = true;

      setWindowState((prev) => ({
        ...prev,
        isTransitioning: true,
      }));

      if (windowState.isExpanded) {
        // Collapsing: unmount component first, then resize
        setIsComponentMounted(false);

        // Small delay to allow component to unmount cleanly
        await new Promise((resolve) => window.setTimeout(resolve, 100));

        // Reset to original size
        const success = await window.electron.resizeWindow(0);

        if (success) {
          setWindowState((prev) => ({
            ...prev,
            isExpanded: false,
            currentWidth: prev.originalWidth,
            isTransitioning: false,
          }));
        } else {
          throw new Error('Failed to resize window to original size');
        }
      } else {
        // Expanding: resize first, then mount component
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
