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
 * Combines user activity detection with smart auto-scroll logic
 * to provide a non-disruptive chat experience
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
  const isLockingMessageRef = useRef<boolean>(false); // NEW: Track when locking message

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
    
    // NEVER auto-scroll if locked to a message OR currently locking
    if (activity.state === UserActivityState.LOCKED_TO_MESSAGE || isLockingMessageRef.current) {
      console.log('🔒 Skipping auto-scroll - locked to message or locking in progress:', activity.lockedMessageId);
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
    
    // Use smooth scrolling for better UX
    scrollMethods.scrollToBottom();
  }, [scrollMethods, clearTimeouts, activity.isUserActive, activity.state, activity.lockedMessageId]);

  // Execute graceful return to bottom (smoother animation)
  const executeGracefulReturn = useCallback(() => {
    if (!scrollContainerRef.current || !scrollMethods.scrollToPosition) {
      // Fallback to regular scroll if position method not available
      executeAutoScroll();
      return;
    }
    
    // NEVER auto-scroll if locked to a message OR currently locking
    if (activity.state === UserActivityState.LOCKED_TO_MESSAGE || isLockingMessageRef.current) {
      console.log('🔒 Skipping graceful return - locked to message or locking in progress:', activity.lockedMessageId);
      pendingAutoScrollRef.current = false;
      return;
    }
    
    // Don't execute if user is active
    if (activity.isUserActive) {
      pendingAutoScrollRef.current = false;
      return;
    }
    
    clearTimeouts();
    pendingAutoScrollRef.current = false;
    
    console.log('🎯 Executing graceful return to bottom');
    
    // Smooth scroll to bottom with custom timing
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
    
    // NEVER schedule auto-scroll if locked to a message OR currently locking
    if (activity.state === UserActivityState.LOCKED_TO_MESSAGE || isLockingMessageRef.current) {
      pendingAutoScrollRef.current = false;
      return;
    }
    
    // Reset user interrupted flag when user becomes idle
    if (!activity.isUserActive) {
      userInterruptedRef.current = false;
    }
    
    // Don't auto-scroll if user is active
    if (activity.isUserActive) {
      pendingAutoScrollRef.current = false;
      return;
    }
    
    if (!activity.shouldAutoScroll) {
      pendingAutoScrollRef.current = false;
      return;
    }
    
    pendingAutoScrollRef.current = true;
    
    const delay = (() => {
      switch (activity.state) {
        case UserActivityState.IDLE_AT_BOTTOM:
          // Quick scroll when idle at bottom
          console.log('⏱️ Scheduling auto-scroll for IDLE_AT_BOTTOM in', finalConfig.autoScrollDelay, 'ms');
          return finalConfig.autoScrollDelay;
          
        case UserActivityState.IDLE_ABOVE:
          // Graceful return to bottom after longer delay
          console.log('⏱️ Scheduling graceful return for IDLE_ABOVE in', finalConfig.gracefulReturnDelay, 'ms');
          return finalConfig.gracefulReturnDelay;
          
        default:
          // Don't auto-scroll when actively reading, following, or locked
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

  // Handle state changes to trigger immediate scheduling
  useEffect(() => {
    const currentState = activity.state;
    const previousState = lastStateRef.current;
    
    // Log state changes for debugging
    if (currentState !== previousState) {
      console.log('📊 State change:', previousState, '→', currentState, {
        isUserActive: activity.isUserActive,
        shouldAutoScroll: activity.shouldAutoScroll,
        isNearBottom: activity.isNearBottom,
        lockedMessageId: activity.lockedMessageId,
        isLocking: isLockingMessageRef.current
      });
    }
    
    lastStateRef.current = currentState;
    
    // Schedule auto-scroll when state changes (unless locked or locking)
    if (currentState !== UserActivityState.LOCKED_TO_MESSAGE && !isLockingMessageRef.current) {
      scheduleAutoScroll();
    }
  }, [activity.state, activity.isUserActive, activity.shouldAutoScroll, activity.lockedMessageId, scheduleAutoScroll]);

  // Handle content changes (new messages) - DO NOT AUTO-SCROLL WHEN LOCKED OR LOCKING
  const handleContentChange = useCallback(() => {
    if (!scrollContainerRef.current) return;
    
    const currentHeight = scrollContainerRef.current.scrollHeight;
    const hasNewContent = currentHeight > lastContentHeightRef.current;
    
    if (hasNewContent) {
      lastContentHeightRef.current = currentHeight;
      console.log('📝 New content detected, current state:', activity.state, 'isLocking:', isLockingMessageRef.current);
      
      // CRITICAL: Don't schedule auto-scroll if locked to a message OR currently locking
      if (activity.state === UserActivityState.LOCKED_TO_MESSAGE || isLockingMessageRef.current) {
        console.log('🔒 Skipping content change auto-scroll - locked to message or locking in progress');
        return;
      }
      
      // Always try to schedule auto-scroll when new content arrives
      // The scheduling logic will determine if it should actually execute
      scheduleAutoScroll();
    }
  }, [scrollContainerRef, scheduleAutoScroll, activity.state]);

  // Manual scroll to bottom (for external triggers) - UNLOCKS MESSAGE
  const scrollToBottomNow = useCallback(() => {
    console.log('🎯 Manual scroll to bottom requested');
    
    // Clear locking flag
    isLockingMessageRef.current = false;
    
    // Unlock from message if locked
    if (activity.state === UserActivityState.LOCKED_TO_MESSAGE) {
      activity.unlockFromMessage();
    }
    
    clearTimeouts();
    userInterruptedRef.current = false; // Reset interrupted flag
    executeAutoScroll();
  }, [clearTimeouts, executeAutoScroll, activity]);

  // Graceful scroll to bottom with visual feedback - UNLOCKS MESSAGE
  const gracefulScrollToBottom = useCallback(() => {
    console.log('🎯 Manual graceful scroll to bottom requested');
    
    // Clear locking flag
    isLockingMessageRef.current = false;
    
    // Unlock from message if locked
    if (activity.state === UserActivityState.LOCKED_TO_MESSAGE) {
      activity.unlockFromMessage();
    }
    
    clearTimeouts();
    userInterruptedRef.current = false; // Reset interrupted flag
    executeGracefulReturn();
  }, [clearTimeouts, executeGracefulReturn, activity]);

  // Lock to a specific message (exposed to parent components) - PREVENTS AUTO-SCROLL
  const lockToMessage = useCallback((messageId: string, element?: HTMLElement) => {
    console.log('🔒 Locking scroll to message (COMPLETELY DISABLE AUTO-SCROLL):', messageId);
    
    // Set locking flag to prevent ANY auto-scroll during the locking process
    isLockingMessageRef.current = true;
    
    // Clear any pending auto-scrolls immediately
    clearTimeouts();
    pendingAutoScrollRef.current = false;
    
    // Lock the message
    activity.lockToMessage(messageId, element);
    
    // Keep locking flag for a moment to ensure no interference
    setTimeout(() => {
      isLockingMessageRef.current = false;
      console.log('✅ Message locking complete, auto-scroll disabled while locked');
    }, 100);
  }, [clearTimeouts, activity]);

  // Unlock from message (exposed to parent components)
  const unlockFromMessage = useCallback(() => {
    console.log('🔓 Unlocking scroll from message');
    isLockingMessageRef.current = false;
    activity.unlockFromMessage();
    // Don't immediately schedule - let the state change trigger it
  }, [activity]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      clearTimeouts();
      isLockingMessageRef.current = false;
    };
  }, [clearTimeouts]);

  // Debug logging
  useEffect(() => {
    console.log('🔍 Intelligent Scroll Debug:', {
      userState: activity.state,
      isActive: activity.isUserActive,
      shouldAutoScroll: activity.shouldAutoScroll,
      isNearBottom: activity.isNearBottom,
      scrollVelocity: activity.scrollVelocity.toFixed(2),
      pendingAutoScroll: pendingAutoScrollRef.current,
      userInterrupted: userInterruptedRef.current,
      lockedMessageId: activity.lockedMessageId,
      isLocking: isLockingMessageRef.current
    });
  }, [activity]);

  return {
    // User activity data
    userActivity: activity,
    
    // Scroll control methods
    handleContentChange,
    scrollToBottomNow,
    gracefulScrollToBottom,
    
    // Message locking methods
    lockToMessage,
    unlockFromMessage,
    
    // State information
    isPendingAutoScroll: pendingAutoScrollRef.current,
    isLockedToMessage: activity.state === UserActivityState.LOCKED_TO_MESSAGE,
    lockedMessageId: activity.lockedMessageId,
    isLockingMessage: isLockingMessageRef.current, // NEW: Expose locking state
    
    // Configuration
    config: finalConfig
  };
}
