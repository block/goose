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
  messageLockTimeout: 60000 // 60 seconds - longer since we want more manual control
};

/**
 * Hook for detecting user activity and intent in chat scrolling context
 * 
 * Key behavior: Lock allows scrolling around message area but unlocks when user moves away
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
  const lockedMessageBoundsRef = useRef<{ top: number; bottom: number; height: number } | null>(null);

  // Calculate if user is near bottom of scroll container
  const checkIsNearBottom = useCallback((): boolean => {
    if (!scrollContainerRef.current) return true;
    
    const element = scrollContainerRef.current;
    const { scrollHeight, scrollTop, clientHeight } = element;
    const scrollBottom = scrollTop + clientHeight;
    const distanceFromBottom = scrollHeight - scrollBottom;
    
    return distanceFromBottom <= 100; // Within 100px of bottom
  }, [scrollContainerRef]);

  // Get message boundaries relative to scroll container
  const getMessageBounds = useCallback((): { top: number; bottom: number; height: number } | null => {
    if (!lockedElement || !scrollContainerRef.current) return null;
    
    const scrollContainer = scrollContainerRef.current;
    const messageRect = lockedElement.getBoundingClientRect();
    const containerRect = scrollContainer.getBoundingClientRect();
    
    // Calculate message position relative to the scroll container's content
    const messageTopInContainer = messageRect.top - containerRect.top + scrollContainer.scrollTop;
    const messageBottomInContainer = messageRect.bottom - containerRect.top + scrollContainer.scrollTop;
    const messageHeight = messageBottomInContainer - messageTopInContainer;
    
    return {
      top: messageTopInContainer,
      bottom: messageBottomInContainer,
      height: messageHeight
    };
  }, [lockedElement, scrollContainerRef]);

  // Smart unlock check - allows scrolling around message area
  const shouldUnlockBasedOnScroll = useCallback((): boolean => {
    if (!scrollContainerRef.current || !lockedMessageBoundsRef.current) {
      console.log('❌ No scroll container or message bounds');
      return false;
    }
    
    const currentScrollTop = scrollContainerRef.current.scrollTop;
    const viewportHeight = scrollContainerRef.current.clientHeight;
    const currentViewportBottom = currentScrollTop + viewportHeight;
    
    const bounds = lockedMessageBoundsRef.current;
    
    // Create a "reading zone" around the message
    // This allows scrolling to read the full message and some context
    const readingZoneBuffer = Math.max(viewportHeight * 0.5, 300); // 50% of viewport or 300px, whichever is larger
    const readingZoneTop = bounds.top - readingZoneBuffer;
    const readingZoneBottom = bounds.bottom + readingZoneBuffer;
    
    // Check if current viewport is completely outside the reading zone
    const viewportCompletelyAbove = currentViewportBottom < readingZoneTop;
    const viewportCompletelyBelow = currentScrollTop > readingZoneBottom;
    
    const shouldUnlock = viewportCompletelyAbove || viewportCompletelyBelow;
    
    console.log('🔍 SMART UNLOCK CHECK:', {
      currentScrollTop,
      viewportHeight,
      currentViewportBottom,
      messageTop: bounds.top,
      messageBottom: bounds.bottom,
      messageHeight: bounds.height,
      readingZoneTop,
      readingZoneBottom,
      readingZoneBuffer,
      viewportCompletelyAbove,
      viewportCompletelyBelow,
      shouldUnlock,
      status: shouldUnlock ? '🔓 UNLOCK' : '🔒 STAY LOCKED'
    });
    
    return shouldUnlock;
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

  // Lock scroll position to a specific message
  const lockToMessage = useCallback((messageId: string, element?: HTMLElement) => {
    console.log('🔒 Locking to message:', messageId);
    
    setLockedMessageId(messageId);
    setLockedElement(element);
    setState(UserActivityState.LOCKED_TO_MESSAGE);
    
    // Store the message bounds for smart unlocking
    if (element) {
      const bounds = getMessageBounds();
      if (bounds) {
        lockedMessageBoundsRef.current = bounds;
        console.log('📍 Stored message bounds:', bounds);
      }
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
    
  }, [finalConfig.messageLockTimeout, getMessageBounds]);

  // Unlock from message and return to normal behavior
  const unlockFromMessage = useCallback(() => {
    console.log('🔓 Unlocking from message');
    
    setLockedMessageId(undefined);
    setLockedElement(undefined);
    lockedMessageBoundsRef.current = null;
    
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

  // Handle scroll events - SMART: Check if scrolled outside message reading zone
  const handleScroll = useCallback((event: Event) => {
    const target = event.target as HTMLElement;
    if (!target) return;
    
    const velocity = calculateScrollVelocity(target.scrollTop);
    setScrollVelocity(velocity);
    
    const nearBottom = checkIsNearBottom();
    setIsNearBottom(nearBottom);
    
    // KEY LOGIC: If locked to message, use smart unlock detection
    if (state === UserActivityState.LOCKED_TO_MESSAGE) {
      console.log('🔒 Locked state - checking smart unlock condition...');
      
      if (shouldUnlockBasedOnScroll()) {
        console.log('✅ SMART UNLOCK: User scrolled outside message reading zone');
        unlockFromMessage();
        return;
      } else {
        console.log('🔒 STAYING LOCKED: Still within message reading zone');
      }
      return; // Stay locked, don't change state
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
    
  }, [calculateScrollVelocity, checkIsNearBottom, finalConfig.scrollVelocityThreshold, markUserActive, state, shouldUnlockBasedOnScroll, unlockFromMessage]);

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
      console.log('⌨️ Escape key pressed - unlocking');
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
        console.log('⌨️ Keyboard navigation while locked, will check reading zone');
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
    
    // If locked to message, don't unlock on wheel - let scroll handler check reading zone
    if (state === UserActivityState.LOCKED_TO_MESSAGE) {
      console.log('🖱️ Wheel scroll while locked, will check reading zone');
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

  // Debug logging every 2 seconds when locked
  useEffect(() => {
    if (state === UserActivityState.LOCKED_TO_MESSAGE) {
      const interval = setInterval(() => {
        if (scrollContainerRef.current && lockedMessageBoundsRef.current) {
          const currentScroll = scrollContainerRef.current.scrollTop;
          const bounds = lockedMessageBoundsRef.current;
          const viewportHeight = scrollContainerRef.current.clientHeight;
          const readingZoneBuffer = Math.max(viewportHeight * 0.5, 300);
          
          console.log('🔒 LOCK STATUS:', {
            locked: true,
            messageId: lockedMessageId,
            currentScroll,
            messageTop: bounds.top,
            messageBottom: bounds.bottom,
            messageHeight: bounds.height,
            readingZoneBuffer,
            readingZoneTop: bounds.top - readingZoneBuffer,
            readingZoneBottom: bounds.bottom + readingZoneBuffer,
            withinReadingZone: true // If we're here, we're still locked
          });
        }
      }, 2000);
      
      return () => clearInterval(interval);
    }
  }, [state, lockedMessageId, scrollContainerRef]);

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
