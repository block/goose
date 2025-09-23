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
  messageLockTimeout: 30000, // 30 seconds
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

  // Clear all timeouts - CRITICAL for preventing unwanted scrolls
  const clearTimeouts = useCallback(() => {
    if (autoScrollTimeoutRef.current) {
      clearTimeout(autoScrollTimeoutRef.current);
      autoScrollTimeoutRef.current = null;
      console.log('🚫 Cleared auto-scroll timeout');
    }
    if (gracefulReturnTimeoutRef.current) {
      clearTimeout(gracefulReturnTimeoutRef.current);
      gracefulReturnTimeoutRef.current = null;
      console.log('🚫 Cleared graceful return timeout');
    }
    pendingAutoScrollRef.current = false;
  }, []);

  // Execute auto-scroll with appropriate timing
  const executeAutoScroll = useCallback(() => {
    if (!scrollMethods.scrollToBottom) return;
    
    // CRITICAL: Never auto-scroll when locked to a message
    if (activity.state === UserActivityState.LOCKED_TO_MESSAGE) {
      console.log('🔒 BLOCKED: Auto-scroll execution prevented - locked to message:', activity.lockedMessageId);
      pendingAutoScrollRef.current = false;
      return;
    }
    
    // Don't auto-scroll if user is actively scrolling
    if (activity.isUserActive) {
      console.log('🔒 BLOCKED: Auto-scroll execution prevented - user is active');
      pendingAutoScrollRef.current = false;
      return;
    }
    
    clearTimeouts();
    pendingAutoScrollRef.current = false;
    
    console.log('🚀 EXECUTING: Auto-scroll for state:', activity.state);
    scrollMethods.scrollToBottom();
  }, [scrollMethods, clearTimeouts, activity.isUserActive, activity.state, activity.lockedMessageId]);

  // Execute graceful return to bottom (smoother animation)
  const executeGracefulReturn = useCallback(() => {
    if (!scrollContainerRef.current || !scrollMethods.scrollToPosition) {
      executeAutoScroll();
      return;
    }
    
    // CRITICAL: Never auto-scroll when locked to a message
    if (activity.state === UserActivityState.LOCKED_TO_MESSAGE) {
      console.log('🔒 BLOCKED: Graceful return prevented - locked to message:', activity.lockedMessageId);
      pendingAutoScrollRef.current = false;
      return;
    }
    
    if (activity.isUserActive) {
      console.log('🔒 BLOCKED: Graceful return prevented - user is active');
      pendingAutoScrollRef.current = false;
      return;
    }
    
    clearTimeouts();
    pendingAutoScrollRef.current = false;
    
    console.log('🎯 EXECUTING: Graceful return to bottom');
    
    const element = scrollContainerRef.current;
    const targetScrollTop = element.scrollHeight - element.clientHeight;
    
    scrollMethods.scrollToPosition({
      top: targetScrollTop,
      behavior: 'smooth'
    });
  }, [scrollContainerRef, scrollMethods, clearTimeouts, activity.isUserActive, activity.state, activity.lockedMessageId, executeAutoScroll]);

  // Schedule auto-scroll based on user activity state
  const scheduleAutoScroll = useCallback(() => {
    // CRITICAL: Always clear existing timeouts first
    clearTimeouts();
    
    // CRITICAL: Never schedule when locked
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
      console.log('🔒 BLOCKED: Auto-scroll scheduling prevented - user active or shouldAutoScroll false');
      pendingAutoScrollRef.current = false;
      return;
    }
    
    pendingAutoScrollRef.current = true;
    
    const delay = (() => {
      switch (activity.state) {
        case UserActivityState.IDLE_AT_BOTTOM:
          console.log('⏱️ SCHEDULING: Auto-scroll for IDLE_AT_BOTTOM in', finalConfig.autoScrollDelay, 'ms');
          return finalConfig.autoScrollDelay;
        case UserActivityState.IDLE_ABOVE:
          console.log('⏱️ SCHEDULING: Graceful return for IDLE_ABOVE in', finalConfig.gracefulReturnDelay, 'ms');
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
      console.log('⏱️ SCHEDULED: Auto-scroll timeout set for', delay, 'ms');
    }
  }, [activity.shouldAutoScroll, activity.state, activity.isUserActive, finalConfig, executeAutoScroll, executeGracefulReturn, clearTimeouts]);

  // Detect when user interrupts auto-scroll behavior
  useEffect(() => {
    if (activity.state === UserActivityState.ACTIVELY_READING && activity.isUserActive) {
      userInterruptedRef.current = true;
      clearTimeouts();
      pendingAutoScrollRef.current = false;
      console.log('🚫 User interrupted - clearing all timeouts');
    }
  }, [activity.state, activity.isUserActive, clearTimeouts]);

  // Handle state changes
  useEffect(() => {
    const currentState = activity.state;
    const previousState = lastStateRef.current;
    
    if (currentState !== previousState) {
      console.log('📊 STATE CHANGE:', previousState, '→', currentState, {
        isUserActive: activity.isUserActive,
        shouldAutoScroll: activity.shouldAutoScroll,
        isNearBottom: activity.isNearBottom,
        lockedMessageId: activity.lockedMessageId
      });
      
      // CRITICAL: Clear timeouts on any state change
      if (currentState === UserActivityState.LOCKED_TO_MESSAGE) {
        console.log('🔒 ENTERING LOCKED STATE - clearing all timeouts');
        clearTimeouts();
      }
    }
    
    lastStateRef.current = currentState;
    
    // Only schedule when not locked
    if (currentState !== UserActivityState.LOCKED_TO_MESSAGE) {
      scheduleAutoScroll();
    }
  }, [activity.state, activity.isUserActive, activity.shouldAutoScroll, activity.lockedMessageId, scheduleAutoScroll, clearTimeouts]);

  // CORE FUNCTION: Handle new content (messages) - this is where locking prevents auto-scroll
  const handleContentChange = useCallback(() => {
    if (!scrollContainerRef.current) return;
    
    const currentHeight = scrollContainerRef.current.scrollHeight;
    const hasNewContent = currentHeight > lastContentHeightRef.current;
    
    if (hasNewContent) {
      lastContentHeightRef.current = currentHeight;
      
      // CRITICAL: Don't auto-scroll when locked to a message
      if (activity.state === UserActivityState.LOCKED_TO_MESSAGE) {
        console.log('🔒 NEW MESSAGE BLOCKED: Auto-scroll prevented - locked to message:', activity.lockedMessageId);
        // Also clear any existing timeouts that might have been set before lock
        clearTimeouts();
        return;
      }
      
      console.log('📝 NEW CONTENT: Scheduling auto-scroll for state:', activity.state);
      scheduleAutoScroll();
    }
  }, [scrollContainerRef, scheduleAutoScroll, activity.state, activity.lockedMessageId, clearTimeouts]);

  // Manual scroll to bottom - unlocks message
  const scrollToBottomNow = useCallback(() => {
    console.log('🎯 MANUAL: Scroll to bottom - unlocking message');
    if (activity.state === UserActivityState.LOCKED_TO_MESSAGE) {
      activity.unlockFromMessage();
    }
    clearTimeouts();
    userInterruptedRef.current = false;
    executeAutoScroll();
  }, [clearTimeouts, executeAutoScroll, activity]);

  // Graceful scroll to bottom - unlocks message
  const gracefulScrollToBottom = useCallback(() => {
    console.log('🎯 MANUAL: Graceful scroll - unlocking message');
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
    // CRITICAL: Clear all timeouts immediately when locking
    clearTimeouts();
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

  // Debug logging for locked state
  useEffect(() => {
    if (activity.state === UserActivityState.LOCKED_TO_MESSAGE) {
      console.log('🔒 LOCKED STATE ACTIVE:', {
        messageId: activity.lockedMessageId,
        pendingAutoScroll: pendingAutoScrollRef.current,
        hasAutoScrollTimeout: autoScrollTimeoutRef.current !== null,
        hasGracefulTimeout: gracefulReturnTimeoutRef.current !== null
      });
    }
  }, [activity.state, activity.lockedMessageId]);

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
