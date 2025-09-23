import { useCallback, useEffect, useRef, useState } from 'react';

export enum UserActivityState {
  ACTIVELY_READING = 'actively_reading',
  IDLE_AT_BOTTOM = 'idle_at_bottom', 
  IDLE_ABOVE = 'idle_above',
  FOLLOWING = 'following',
  LOCKED_TO_MESSAGE = 'locked_to_message' // User clicked on a specific message
}

interface UserActivityConfig {
  idleTimeout?: number; // ms to wait before considering user idle
  activityDebounce?: number; // ms to debounce activity detection
  scrollVelocityThreshold?: number; // px/ms threshold for intentional scrolling
  messageLockTimeout?: number; // ms to wait before unlocking from clicked message
}

interface UserActivityData {
  state: UserActivityState;
  isUserActive: boolean;
  lastActivityTime: number;
  scrollVelocity: number;
  isNearBottom: boolean;
  shouldAutoScroll: boolean;
  lockedMessageId?: string; // ID of the message user clicked on
  lockedElement?: HTMLElement; // Reference to the locked element
  lockToMessage: (messageId: string, element?: HTMLElement) => void;
  unlockFromMessage: () => void;
}

const DEFAULT_CONFIG: Required<UserActivityConfig> = {
  idleTimeout: 4000, // 4 seconds
  activityDebounce: 100, // 100ms
  scrollVelocityThreshold: 0.3,
  messageLockTimeout: 30000 // 30 seconds before auto-unlock (longer since we want manual unlock)
};

/**
 * Hook for detecting user activity and intent in chat scrolling context
 * 
 * Key behavior: Lock only clears when user scrolls past the locked message
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
  const [lockedMessageId, setLockedMessageId] = useState<string | undefined>();
  const [lockedElement, setLockedElement] = useState<HTMLElement | undefined>();
  
  // Refs for tracking scroll behavior
  const lastScrollTime = useRef(Date.now());
  const lastScrollTop = useRef(0);
  const activityTimeoutRef = useRef<number | null>(null);
  const idleTimeoutRef = useRef<number | null>(null);
  const messageLockTimeoutRef = useRef<number | null>(null);
  const isScrollingRef = useRef(false);
  const lockedMessagePositionRef = useRef<number | null>(null);

  // Calculate if user is near bottom of scroll container
  const checkIsNearBottom = useCallback((): boolean => {
    if (!scrollContainerRef.current) return true;
    
    const element = scrollContainerRef.current;
    const { scrollHeight, scrollTop, clientHeight } = element;
    const scrollBottom = scrollTop + clientHeight;
    const distanceFromBottom = scrollHeight - scrollBottom;
    
    return distanceFromBottom <= 100; // Within 100px of bottom
  }, [scrollContainerRef]);

  // Calculate the position of the locked message relative to viewport
  const getLockedMessagePosition = useCallback((): number | null => {
    if (!lockedElement || !scrollContainerRef.current) return null;
    
    const scrollContainer = scrollContainerRef.current;
    const containerRect = scrollContainer.getBoundingClientRect();
    const messageRect = lockedElement.getBoundingClientRect();
    
    // Calculate message position relative to scroll container
    const messageTopRelativeToContainer = messageRect.top - containerRect.top + scrollContainer.scrollTop;
    
    return messageTopRelativeToContainer;
  }, [lockedElement, scrollContainerRef]);

  // Check if user has scrolled past the locked message
  const hasScrolledPastLockedMessage = useCallback((): boolean => {
    if (!scrollContainerRef.current || !lockedElement) return false;
    
    const scrollContainer = scrollContainerRef.current;
    const currentScrollTop = scrollContainer.scrollTop;
    const viewportHeight = scrollContainer.clientHeight;
    const currentViewportBottom = currentScrollTop + viewportHeight;
    
    const messagePosition = getLockedMessagePosition();
    if (messagePosition === null) return false;
    
    // Consider "scrolled past" if the message is completely above the current viewport
    // Add some buffer (50px) to avoid premature unlocking
    const hasScrolledPast = currentScrollTop > messagePosition + 50;
    
    console.log('🔍 Scroll past check:', {
      currentScrollTop,
      messagePosition,
      hasScrolledPast,
      buffer: 50
    });
    
    return hasScrolledPast;
  }, [scrollContainerRef, lockedElement, getLockedMessagePosition]);

  // Calculate scroll velocity for intent detection
  const calculateScrollVelocity = useCallback((currentScrollTop: number): number => {
    const now = Date.now();
    const timeDelta = now - lastScrollTime.current;
    const scrollDelta = Math.abs(currentScrollTop - lastScrollTop.current);
    
    lastScrollTime.current = now;
    lastScrollTop.current = currentScrollTop;
    
    return timeDelta > 0 ? scrollDelta / timeDelta : 0;
  }, []);

  // Lock scroll position to a specific message
  const lockToMessage = useCallback((messageId: string, element?: HTMLElement) => {
    console.log('🔒 Locking to message:', messageId);
    
    setLockedMessageId(messageId);
    setLockedElement(element);
    setState(UserActivityState.LOCKED_TO_MESSAGE);
    
    // Store the locked message position
    if (element) {
      const position = getLockedMessagePosition();
      lockedMessagePositionRef.current = position;
      console.log('📍 Stored locked message position:', position);
    }
    
    // Clear existing timeouts
    if (activityTimeoutRef.current) {
      clearTimeout(activityTimeoutRef.current);
    }
    if (idleTimeoutRef.current) {
      clearTimeout(idleTimeoutRef.current);
    }
    if (messageLockTimeoutRef.current) {
      clearTimeout(messageLockTimeoutRef.current);
    }
    
    // Set timeout to auto-unlock after specified time (as fallback)
    messageLockTimeoutRef.current = window.setTimeout(() => {
      console.log('⏰ Auto-unlocking from message after timeout');
      unlockFromMessage();
    }, finalConfig.messageLockTimeout);
    
  }, [finalConfig.messageLockTimeout, getLockedMessagePosition]);

  // Unlock from message and return to normal behavior
  const unlockFromMessage = useCallback(() => {
    console.log('🔓 Unlocking from message');
    
    setLockedMessageId(undefined);
    setLockedElement(undefined);
    lockedMessagePositionRef.current = null;
    
    // Clear message lock timeout
    if (messageLockTimeoutRef.current) {
      clearTimeout(messageLockTimeoutRef.current);
      messageLockTimeoutRef.current = null;
    }
    
    // Determine new state based on current position
    const nearBottom = checkIsNearBottom();
    if (nearBottom) {
      setState(UserActivityState.FOLLOWING);
    } else {
      setState(UserActivityState.ACTIVELY_READING);
    }
  }, [checkIsNearBottom]);

  // Mark user as active and reset timeouts
  const markUserActive = useCallback(() => {
    const now = Date.now();
    setIsUserActive(true);
    setLastActivityTime(now);
    
    // Don't reset timeouts if locked to a message
    if (state === UserActivityState.LOCKED_TO_MESSAGE) {
      return;
    }
    
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
      
      // Don't set idle timeout if locked to message
      if (state === UserActivityState.LOCKED_TO_MESSAGE) {
        return;
      }
      
      // Set idle timeout after activity stops
      idleTimeoutRef.current = window.setTimeout(() => {
        const nearBottom = checkIsNearBottom();
        const newState = nearBottom ? UserActivityState.IDLE_AT_BOTTOM : UserActivityState.IDLE_ABOVE;
        console.log('⏰ User became idle, setting state to:', newState, 'nearBottom:', nearBottom);
        setState(newState);
      }, finalConfig.idleTimeout);
      
    }, finalConfig.activityDebounce);
  }, [finalConfig.activityDebounce, finalConfig.idleTimeout, checkIsNearBottom, state]);

  // Handle scroll events - KEY: Check if scrolled past locked message
  const handleScroll = useCallback((event: Event) => {
    const target = event.target as HTMLElement;
    if (!target) return;
    
    const velocity = calculateScrollVelocity(target.scrollTop);
    setScrollVelocity(velocity);
    
    const nearBottom = checkIsNearBottom();
    setIsNearBottom(nearBottom);
    
    // KEY LOGIC: If locked to message, check if user has scrolled past it
    if (state === UserActivityState.LOCKED_TO_MESSAGE) {
      if (hasScrolledPastLockedMessage()) {
        console.log('📜 User scrolled past locked message, unlocking');
        unlockFromMessage();
        return;
      } else {
        console.log('🔒 Still within locked message area, maintaining lock');
        return; // Stay locked, don't change state
      }
    }
    
    // Normal scroll handling for non-locked states
    if (velocity > 0) {
      markUserActive();
      isScrollingRef.current = true;
      
      if (!nearBottom) {
        setState(UserActivityState.ACTIVELY_READING);
      } else if (nearBottom && velocity < finalConfig.scrollVelocityThreshold) {
        setState(UserActivityState.FOLLOWING);
      } else {
        setState(UserActivityState.ACTIVELY_READING);
      }
    }
    
    // Reset scrolling flag after a short delay
    setTimeout(() => {
      isScrollingRef.current = false;
    }, 50);
    
  }, [calculateScrollVelocity, checkIsNearBottom, finalConfig.scrollVelocityThreshold, markUserActive, state, hasScrolledPastLockedMessage, unlockFromMessage]);

  // Handle mouse activity
  const handleMouseActivity = useCallback((event: MouseEvent) => {
    // Only count significant mouse movements as activity
    const movement = Math.abs(event.movementX) + Math.abs(event.movementY);
    if (movement > 2) { // Minimum movement threshold
      markUserActive();
      
      // Don't change state based on mouse movement if locked to message
      if (state === UserActivityState.LOCKED_TO_MESSAGE) {
        return;
      }
      
      // Don't change state based on mouse movement alone unless we're not scrolling
      if (!isScrollingRef.current) {
        const nearBottom = checkIsNearBottom();
        if (!nearBottom && state !== UserActivityState.ACTIVELY_READING) {
          setState(UserActivityState.ACTIVELY_READING);
        }
      }
    }
  }, [markUserActive, checkIsNearBottom, state]);

  // Handle keyboard activity
  const handleKeyboardActivity = useCallback((event: KeyboardEvent) => {
    // Escape key unlocks from message
    if (event.key === 'Escape' && state === UserActivityState.LOCKED_TO_MESSAGE) {
      unlockFromMessage();
      return;
    }
    
    // Track navigation keys that indicate reading intent
    const navigationKeys = [
      'ArrowUp', 'ArrowDown', 'PageUp', 'PageDown', 
      'Home', 'End', ' ' // Space bar for scrolling
    ];
    
    if (navigationKeys.includes(event.key)) {
      markUserActive();
      
      // If locked to message, don't unlock on keyboard - let scroll handler check position
      if (state === UserActivityState.LOCKED_TO_MESSAGE) {
        console.log('⌨️ Keyboard navigation while locked, checking scroll position');
        return;
      }
      
      // Keyboard navigation always indicates intentional reading
      const nearBottom = checkIsNearBottom();
      if (!nearBottom || ['ArrowUp', 'PageUp', 'Home'].includes(event.key)) {
        setState(UserActivityState.ACTIVELY_READING);
      }
    }
  }, [markUserActive, checkIsNearBottom, state, unlockFromMessage]);

  // Handle wheel events (mouse wheel, trackpad)
  const handleWheel = useCallback((event: WheelEvent) => {
    // Any wheel activity indicates intentional scrolling
    markUserActive();
    isScrollingRef.current = true;
    
    // If locked to message, don't unlock on wheel - let scroll handler check position
    if (state === UserActivityState.LOCKED_TO_MESSAGE) {
      console.log('🖱️ Wheel scroll while locked, checking scroll position');
      return;
    }
    
    const nearBottom = checkIsNearBottom();
    
    // Respect wheel direction
    if (event.deltaY < 0) {
      // Scrolling up = always actively reading
      setState(UserActivityState.ACTIVELY_READING);
    } else if (event.deltaY > 0 && nearBottom) {
      // Scrolling down at bottom = following
      setState(UserActivityState.FOLLOWING);
    } else {
      // Scrolling down while not at bottom = still actively reading
      setState(UserActivityState.ACTIVELY_READING);
    }
    
    // Reset scrolling flag
    setTimeout(() => {
      isScrollingRef.current = false;
    }, 50);
    
  }, [markUserActive, checkIsNearBottom, state]);

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
      if (messageLockTimeoutRef.current) {
        clearTimeout(messageLockTimeoutRef.current);
      }
    };
  }, [scrollContainerRef, handleScroll, handleMouseActivity, handleKeyboardActivity, handleWheel]);

  // Determine if auto-scroll should happen
  // NEVER auto-scroll when locked to a message
  const shouldAutoScroll = 
    state !== UserActivityState.LOCKED_TO_MESSAGE && (
      state === UserActivityState.IDLE_AT_BOTTOM ||
      state === UserActivityState.IDLE_ABOVE ||
      (state === UserActivityState.FOLLOWING && !isUserActive)
    );

  // Debug logging for shouldAutoScroll
  useEffect(() => {
    console.log('🎯 shouldAutoScroll calculation:', {
      state,
      isUserActive,
      shouldAutoScroll,
      lockedMessageId,
      lockedPosition: lockedMessagePositionRef.current,
      conditions: {
        notLocked: state !== UserActivityState.LOCKED_TO_MESSAGE,
        idleAtBottom: state === UserActivityState.IDLE_AT_BOTTOM,
        idleAbove: state === UserActivityState.IDLE_ABOVE,
        followingAndNotActive: state === UserActivityState.FOLLOWING && !isUserActive
      }
    });
  }, [state, isUserActive, shouldAutoScroll, lockedMessageId]);

  return {
    state,
    isUserActive,
    lastActivityTime,
    scrollVelocity,
    isNearBottom,
    shouldAutoScroll,
    lockedMessageId,
    lockedElement,
    lockToMessage,
    unlockFromMessage
  };
}
