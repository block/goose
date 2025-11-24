import { useState, useEffect, useRef } from 'react';
import { useUnifiedSidecarContextOptional } from '../contexts/UnifiedSidecarContext';

export type CommentDisplayMode = 'full' | 'condensed';

interface UseCommentDisplayModeOptions {
  /**
   * Width threshold in pixels below which comments condense
   * @default 1200
   */
  breakpoint?: number;
  
  /**
   * Force condensed mode when sidecar is open
   * @default true
   */
  condenseWithSidecar?: boolean;
  
  /**
   * Container element to monitor for width changes
   */
  containerRef?: React.RefObject<HTMLElement>;
}

interface UseCommentDisplayModeReturn {
  displayMode: CommentDisplayMode;
  availableWidth: number;
  hasSidecar: boolean;
  isCondensed: boolean;
}

/**
 * Hook to determine whether comments should display in full or condensed mode
 * based on available screen space and sidecar state
 */
export function useCommentDisplayMode(
  options: UseCommentDisplayModeOptions = {}
): UseCommentDisplayModeReturn {
  const {
    breakpoint = 1200,
    condenseWithSidecar = true,
    containerRef,
  } = options;

  const sidecarContext = useUnifiedSidecarContextOptional();
  const [availableWidth, setAvailableWidth] = useState<number>(
    typeof window !== 'undefined' ? window.innerWidth : 1920
  );
  const [hasSidecar, setHasSidecar] = useState(false);

  // Monitor sidecar state
  useEffect(() => {
    if (!sidecarContext) {
      setHasSidecar(false);
      return;
    }

    const checkSidecar = () => {
      const activeSidecars = sidecarContext.getActiveSidecars();
      setHasSidecar(activeSidecars.length > 0);
    };

    // Check immediately
    checkSidecar();

    // Set up polling to check for sidecar changes
    // (since the context doesn't provide change events)
    const interval = setInterval(checkSidecar, 500);

    return () => clearInterval(interval);
  }, [sidecarContext]);

  // Monitor container or window width
  useEffect(() => {
    const targetElement = containerRef?.current || window;
    
    const updateWidth = () => {
      if (containerRef?.current) {
        setAvailableWidth(containerRef.current.offsetWidth);
      } else {
        setAvailableWidth(window.innerWidth);
      }
    };

    // Initial measurement
    updateWidth();

    // Use ResizeObserver for container, window resize for window
    if (containerRef?.current) {
      const resizeObserver = new ResizeObserver(updateWidth);
      resizeObserver.observe(containerRef.current);
      return () => resizeObserver.disconnect();
    } else {
      window.addEventListener('resize', updateWidth);
      return () => window.removeEventListener('resize', updateWidth);
    }
  }, [containerRef]);

  // Determine display mode
  const isCondensed = 
    availableWidth < breakpoint || 
    (condenseWithSidecar && hasSidecar);

  const displayMode: CommentDisplayMode = isCondensed ? 'condensed' : 'full';

  return {
    displayMode,
    availableWidth,
    hasSidecar,
    isCondensed,
  };
}
