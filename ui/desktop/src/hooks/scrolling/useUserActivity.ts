import { useCallback, useEffect, useRef, useState } from 'react';

export enum UserActivityState {
  ACTIVELY_READING = 'actively_reading',
  IDLE_AT_BOTTOM = 'idle_at_bottom', 
  IDLE_ABOVE = 'idle_above',
  FOLLOWING = 'following'
}

interface UserActivityConfig {
  idleTimeout?: number; // ms to wait before considering user idle
  activityDebounce?: number; // ms to debounce activity detection
  scrollVelocityThreshold?: number; // px/ms threshold for intentional scrolling
}

interface UserActivityData {
  state: UserActivityState;
  isUserActive: boolean;
  lastActivityTime: number;
  scrollVelocity: number;
  isNearBottom: boolean;
  shouldAutoScroll: boolean;
}

const DEFAULT_CONFIG: Required<UserActivityConfig> = {
  idleTimeout: 4000, // 4 seconds
  activityDebounce: 100, // 100ms
  scrollVelocityThreshold: 0.5 // 0.5 px/ms
};

/**
 * Hook for detecting user activity and intent in chat scrolling context
 * 
 * Tracks various user interactions to determine when it's appropriate
 * to auto-scroll to bottom vs respecting user's reading position
 */
export function useUserActivity(
  scrollContainerRef: React.RefObject<HTMLElement | null>,
  config: UserActivityConfig = {}
): UserActivityData {
  const finalConfig = { ...DEFAULT_CONFIG, ...config };
  
  const [state, setState] = useState<UserActivityState>(UserActivityState.FOLLOWING);
  const [isUserActive, setIsUserActive] = useState(false);
  const [lastActivityTime, setLastActivityTime] = useState(Date.now());
  const [scrollVelocity, setScrollVelocity] = useState(0);
  const [isNearBottom, setIsNearBottom] = useState(true);
  
  // Refs for tracking scroll behavior
  const lastScrollTime = useRef(Date.now());
  const lastScrollTop = useRef(0);
  const activityTimeoutRef = useRef<number | null>(null);
  const idleTimeoutRef = useRef<number | null>(null);

  // Calculate if user is near bottom of scroll container
  const checkIsNearBottom = useCallback((): boolean => {
    if (!scrollContainerRef.current) return true;
    
    const element = scrollContainerRef.current;
    const { scrollHeight, scrollTop, clientHeight } = element;
    const scrollBottom = scrollTop + clientHeight;
    const distanceFromBottom = scrollHeight - scrollBottom;
    
    return distanceFromBottom <= 100; // Within 100px of bottom
  }, [scrollContainerRef]);

  // Calculate scroll velocity for intent detection
  const calculateScrollVelocity = useCallback((currentScrollTop: number): number => {
    const now = Date.now();
    const timeDelta = now - lastScrollTime.current;
    const scrollDelta = Math.abs(currentScrollTop - lastScrollTop.current);
    
    lastScrollTime.current = now;
    lastScrollTop.current = currentScrollTop;
    
    return timeDelta > 0 ? scrollDelta / timeDelta : 0;
  }, []);

  // Mark user as active and reset timeouts
  const markUserActive = useCallback(() => {
    const now = Date.now();
    setIsUserActive(true);
    setLastActivityTime(now);
    
    // Clear existing timeouts
    if (activityTimeoutRef.current) {
      clearTimeout(activityTimeoutRef.current);
    }
    if (idleTimeoutRef.current) {
      clearTimeout(idleTimeoutRef.current);
    }
    
    // Set activity debounce timeout
    activityTimeoutRef.current = window.setTimeout(() => {
      setIsUserActive(false);
      
      // Set idle timeout after activity stops
      idleTimeoutRef.current = window.setTimeout(() => {
        const nearBottom = checkIsNearBottom();
        if (nearBottom) {
          setState(UserActivityState.IDLE_AT_BOTTOM);
        } else {
          setState(UserActivityState.IDLE_ABOVE);
        }
      }, finalConfig.idleTimeout);
      
    }, finalConfig.activityDebounce);
  }, [finalConfig.activityDebounce, finalConfig.idleTimeout, checkIsNearBottom]);

  // Handle scroll events
  const handleScroll = useCallback((event: Event) => {
    const target = event.target as HTMLElement;
    if (!target) return;
    
    const velocity = calculateScrollVelocity(target.scrollTop);
    setScrollVelocity(velocity);
    
    const nearBottom = checkIsNearBottom();
    setIsNearBottom(nearBottom);
    
    // Only mark as active if scroll velocity indicates intentional scrolling
    if (velocity > finalConfig.scrollVelocityThreshold) {
      markUserActive();
      
      if (nearBottom) {
        setState(UserActivityState.FOLLOWING);
      } else {
        setState(UserActivityState.ACTIVELY_READING);
      }
    }
  }, [calculateScrollVelocity, checkIsNearBottom, finalConfig.scrollVelocityThreshold, markUserActive]);

  // Handle mouse activity
  const handleMouseActivity = useCallback((event: MouseEvent) => {
    // Only count significant mouse movements as activity
    const movement = Math.abs(event.movementX) + Math.abs(event.movementY);
    if (movement > 2) { // Minimum movement threshold
      markUserActive();
      
      const nearBottom = checkIsNearBottom();
      if (!nearBottom && state !== UserActivityState.ACTIVELY_READING) {
        setState(UserActivityState.ACTIVELY_READING);
      }
    }
  }, [markUserActive, checkIsNearBottom, state]);

  // Handle keyboard activity
  const handleKeyboardActivity = useCallback((event: KeyboardEvent) => {
    // Track navigation keys that indicate reading intent
    const navigationKeys = [
      'ArrowUp', 'ArrowDown', 'PageUp', 'PageDown', 
      'Home', 'End', ' ' // Space bar for scrolling
    ];
    
    if (navigationKeys.includes(event.key)) {
      markUserActive();
      
      const nearBottom = checkIsNearBottom();
      if (!nearBottom) {
        setState(UserActivityState.ACTIVELY_READING);
      }
    }
  }, [markUserActive, checkIsNearBottom]);

  // Handle wheel events (mouse wheel, trackpad)
  const handleWheel = useCallback((event: WheelEvent) => {
    // Any wheel activity indicates intentional scrolling
    markUserActive();
    
    const nearBottom = checkIsNearBottom();
    if (nearBottom && event.deltaY > 0) {
      // Scrolling down at bottom = following
      setState(UserActivityState.FOLLOWING);
    } else if (!nearBottom) {
      // Scrolling while not at bottom = actively reading
      setState(UserActivityState.ACTIVELY_READING);
    }
  }, [markUserActive, checkIsNearBottom]);

  // Set up event listeners
  useEffect(() => {
    const scrollContainer = scrollContainerRef.current;
    if (!scrollContainer) return;

    // Add event listeners
    scrollContainer.addEventListener('scroll', handleScroll, { passive: true });
    document.addEventListener('mousemove', handleMouseActivity, { passive: true });
    document.addEventListener('keydown', handleKeyboardActivity);
    scrollContainer.addEventListener('wheel', handleWheel, { passive: true });

    return () => {
      // Cleanup event listeners
      scrollContainer.removeEventListener('scroll', handleScroll);
      document.removeEventListener('mousemove', handleMouseActivity);
      document.removeEventListener('keydown', handleKeyboardActivity);
      scrollContainer.removeEventListener('wheel', handleWheel);
      
      // Cleanup timeouts
      if (activityTimeoutRef.current) {
        clearTimeout(activityTimeoutRef.current);
      }
      if (idleTimeoutRef.current) {
        clearTimeout(idleTimeoutRef.current);
      }
    };
  }, [scrollContainerRef, handleScroll, handleMouseActivity, handleKeyboardActivity, handleWheel]);

  // Determine if auto-scroll should happen
  const shouldAutoScroll = 
    state === UserActivityState.FOLLOWING || 
    state === UserActivityState.IDLE_AT_BOTTOM ||
    (state === UserActivityState.IDLE_ABOVE && isNearBottom);

  return {
    state,
    isUserActive,
    lastActivityTime,
    scrollVelocity,
    isNearBottom,
    shouldAutoScroll
  };
}
