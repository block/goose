import { useCallback, useEffect, useRef } from 'react';
import { useUserActivity, UserActivityState } from './useUserActivity';

interface IntelligentScrollConfig {
  // User activity detection config
  idleTimeout?: number;
  activityDebounce?: number;
  scrollVelocityThreshold?: number;
  messageLockTimeout?: number;
  
  // Auto-scroll behavior config
  autoScrollDelay?: number; // Delay before auto-scroll when conditions are met
  gracefulReturnDelay?: number; // Delay before graceful return to bottom
  smoothScrollDuration?: number; // Duration for smooth scroll animations
}

interface ScrollMethods {
  scrollToBottom: () => void;
  scrollToPosition: (options: { top: number; behavior?: ScrollBehavior }) => void;
}

const DEFAULT_CONFIG: Required<IntelligentScrollConfig> = {
  idleTimeout: 4000,
  activityDebounce: 100,
  scrollVelocityThreshold: 0.3,
  messageLockTimeout: 15000, // 15 seconds
  autoScrollDelay: 300,
  gracefulReturnDelay: 2000,
  smoothScrollDuration: 500
};

/**
 * Hook for intelligent chat scrolling that respects user intent
 * 
 * The core behavior is simple:
 * - When user clicks a message, prevent auto-scroll on new messages
 * - When user manually scrolls or unlocks, resume normal behavior
 */
export function useIntelligentScroll(
  scrollContainerRef: React.RefObject<HTMLElement | null>,
  scrollMethods: ScrollMethods,
  config: IntelligentScrollConfig = {}
) {
  const finalConfig = { ...DEFAULT_CONFIG, ...config };
  
  // Get user activity state
  const activity = useUserActivity(scrollContainerRef, {
    idleTimeout: finalConfig.idleTimeout,
    activityDebounce: finalConfig.activityDebounce,
    scrollVelocityThreshold: finalConfig.scrollVelocityThreshold,
    messageLockTimeout: finalConfig.messageLockTimeout
  });
  
  // Refs for managing timeouts and state
  const autoScrollTimeoutRef = useRef<number | null>(null);
  const gracefulReturnTimeoutRef = useRef<number | null>(null);
  const lastContentHeightRef = useRef<number>(0);
  const pendingAutoScrollRef = useRef<boolean>(false);
  const userInterruptedRef = useRef<boolean>(false);
  const lastStateRef = useRef<UserActivityState>(activity.state);

  // Clear all timeouts
  const clearTimeouts = useCallback(() => {
    if (autoScrollTimeoutRef.current) {
      clearTimeout(autoScrollTimeoutRef.current);
      autoScrollTimeoutRef.current = null;
    }
    if (gracefulReturnTimeoutRef.current) {
      clearTimeout(gracefulReturnTimeoutRef.current);
      gracefulReturnTimeoutRef.current = null;
    }
  }, []);

  // Execute auto-scroll with appropriate timing
  const executeAutoScroll = useCallback(() => {
    if (!scrollMethods.scrollToBottom) return;
    
    // CORE RULE: Never auto-scroll when locked to a message
    if (activity.state === UserActivityState.LOCKED_TO_MESSAGE) {
      console.log('🔒 BLOCKED: Auto-scroll prevented - locked to message:', activity.lockedMessageId);
      pendingAutoScrollRef.current = false;
      return;
    }
    
    // Don't auto-scroll if user is actively scrolling
    if (activity.isUserActive) {
      pendingAutoScrollRef.current = false;
      return;
    }
    
    clearTimeouts();
    pendingAutoScrollRef.current = false;
    
    console.log('🚀 Executing auto-scroll for state:', activity.state);
    scrollMethods.scrollToBottom();
  }, [scrollMethods, clearTimeouts, activity.isUserActive, activity.state, activity.lockedMessageId]);

  // Execute graceful return to bottom (smoother animation)
  const executeGracefulReturn = useCallback(() => {
    if (!scrollContainerRef.current || !scrollMethods.scrollToPosition) {
      executeAutoScroll();
      return;
    }
    
    // CORE RULE: Never auto-scroll when locked to a message
    if (activity.state === UserActivityState.LOCKED_TO_MESSAGE) {
      console.log('🔒 BLOCKED: Graceful return prevented - locked to message:', activity.lockedMessageId);
      pendingAutoScrollRef.current = false;
      return;
    }
    
    if (activity.isUserActive) {
      pendingAutoScrollRef.current = false;
      return;
    }
    
    clearTimeouts();
    pendingAutoScrollRef.current = false;
    
    console.log('🎯 Executing graceful return to bottom');
    
    const element = scrollContainerRef.current;
    const targetScrollTop = element.scrollHeight - element.clientHeight;
    
    scrollMethods.scrollToPosition({
      top: targetScrollTop,
      behavior: 'smooth'
    });
  }, [scrollContainerRef, scrollMethods, clearTimeouts, activity.isUserActive, activity.state, activity.lockedMessageId, executeAutoScroll]);

  // Schedule auto-scroll based on user activity state
  const scheduleAutoScroll = useCallback(() => {
    clearTimeouts();
    
    // CORE RULE: Never schedule when locked
    if (activity.state === UserActivityState.LOCKED_TO_MESSAGE) {
      console.log('🔒 BLOCKED: Auto-scroll scheduling prevented - locked to message');
      pendingAutoScrollRef.current = false;
      return;
    }
    
    // Reset user interrupted flag when user becomes idle
    if (!activity.isUserActive) {
      userInterruptedRef.current = false;
    }
    
    if (activity.isUserActive || !activity.shouldAutoScroll) {
      pendingAutoScrollRef.current = false;
      return;
    }
    
    pendingAutoScrollRef.current = true;
    
    const delay = (() => {
      switch (activity.state) {
        case UserActivityState.IDLE_AT_BOTTOM:
          return finalConfig.autoScrollDelay;
        case UserActivityState.IDLE_ABOVE:
          return finalConfig.gracefulReturnDelay;
        default:
          return -1;
      }
    })();
    
    if (delay >= 0) {
      const executeFunction = activity.state === UserActivityState.IDLE_ABOVE 
        ? executeGracefulReturn 
        : executeAutoScroll;
        
      autoScrollTimeoutRef.current = window.setTimeout(executeFunction, delay);
    }
  }, [activity.shouldAutoScroll, activity.state, activity.isUserActive, finalConfig, executeAutoScroll, executeGracefulReturn]);

  // Detect when user interrupts auto-scroll behavior
  useEffect(() => {
    if (activity.state === UserActivityState.ACTIVELY_READING && activity.isUserActive) {
      userInterruptedRef.current = true;
      clearTimeouts();
      pendingAutoScrollRef.current = false;
    }
  }, [activity.state, activity.isUserActive, clearTimeouts]);

  // Handle state changes
  useEffect(() => {
    const currentState = activity.state;
    const previousState = lastStateRef.current;
    
    if (currentState !== previousState) {
      console.log('📊 State change:', previousState, '→', currentState, {
        isUserActive: activity.isUserActive,
        shouldAutoScroll: activity.shouldAutoScroll,
        isNearBottom: activity.isNearBottom,
        lockedMessageId: activity.lockedMessageId
      });
    }
    
    lastStateRef.current = currentState;
    
    // Only schedule when not locked
    if (currentState !== UserActivityState.LOCKED_TO_MESSAGE) {
      scheduleAutoScroll();
    }
  }, [activity.state, activity.isUserActive, activity.shouldAutoScroll, activity.lockedMessageId, scheduleAutoScroll]);

  // CORE FUNCTION: Handle new content (messages) - this is where locking prevents auto-scroll
  const handleContentChange = useCallback(() => {
    if (!scrollContainerRef.current) return;
    
    const currentHeight = scrollContainerRef.current.scrollHeight;
    const hasNewContent = currentHeight > lastContentHeightRef.current;
    
    if (hasNewContent) {
      lastContentHeightRef.current = currentHeight;
      
      // CORE RULE: Don't auto-scroll when locked to a message
      if (activity.state === UserActivityState.LOCKED_TO_MESSAGE) {
        console.log('🔒 NEW MESSAGE BLOCKED: Auto-scroll prevented - locked to message:', activity.lockedMessageId);
        return;
      }
      
      console.log('📝 New content detected, scheduling auto-scroll for state:', activity.state);
      scheduleAutoScroll();
    }
  }, [scrollContainerRef, scheduleAutoScroll, activity.state, activity.lockedMessageId]);

  // Manual scroll to bottom - unlocks message
  const scrollToBottomNow = useCallback(() => {
    console.log('🎯 Manual scroll to bottom - unlocking message');
    if (activity.state === UserActivityState.LOCKED_TO_MESSAGE) {
      activity.unlockFromMessage();
    }
    clearTimeouts();
    userInterruptedRef.current = false;
    executeAutoScroll();
  }, [clearTimeouts, executeAutoScroll, activity]);

  // Graceful scroll to bottom - unlocks message
  const gracefulScrollToBottom = useCallback(() => {
    console.log('🎯 Manual graceful scroll - unlocking message');
    if (activity.state === UserActivityState.LOCKED_TO_MESSAGE) {
      activity.unlockFromMessage();
    }
    clearTimeouts();
    userInterruptedRef.current = false;
    executeGracefulReturn();
  }, [clearTimeouts, executeGracefulReturn, activity]);

  // Lock to message - this is the key function that prevents auto-scroll
  const lockToMessage = useCallback((messageId: string, element?: HTMLElement) => {
    console.log('🔒 LOCKING MESSAGE: Auto-scroll will be blocked for new messages:', messageId);
    clearTimeouts(); // Clear any pending scrolls
    activity.lockToMessage(messageId, element);
  }, [clearTimeouts, activity]);

  // Unlock from message
  const unlockFromMessage = useCallback(() => {
    console.log('🔓 UNLOCKING MESSAGE: Auto-scroll will resume');
    activity.unlockFromMessage();
  }, [activity]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      clearTimeouts();
    };
  }, [clearTimeouts]);

  return {
    // User activity data
    userActivity: activity,
    
    // Scroll control methods
    handleContentChange, // KEY: This prevents auto-scroll when locked
    scrollToBottomNow,
    gracefulScrollToBottom,
    
    // Message locking methods
    lockToMessage, // KEY: This enables the lock
    unlockFromMessage,
    
    // State information
    isPendingAutoScroll: pendingAutoScrollRef.current,
    isLockedToMessage: activity.state === UserActivityState.LOCKED_TO_MESSAGE,
    lockedMessageId: activity.lockedMessageId,
    
    // Configuration
    config: finalConfig
  };
}
