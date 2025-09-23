import { useCallback, useEffect, useRef } from 'react';
import { useUserActivity, UserActivityState } from './useUserActivity';

interface IntelligentScrollConfig {
  // User activity detection config
  idleTimeout?: number;
  activityDebounce?: number;
  scrollVelocityThreshold?: number;
  
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
  autoScrollDelay: 300,
  gracefulReturnDelay: 2000, // 2 seconds for graceful return
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
    scrollVelocityThreshold: finalConfig.scrollVelocityThreshold
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
  }, [scrollMethods, clearTimeouts, activity.isUserActive, activity.state]);

  // Execute graceful return to bottom (smoother animation)
  const executeGracefulReturn = useCallback(() => {
    if (!scrollContainerRef.current || !scrollMethods.scrollToPosition) {
      // Fallback to regular scroll if position method not available
      executeAutoScroll();
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
  }, [scrollContainerRef, scrollMethods, clearTimeouts, activity.isUserActive, executeAutoScroll]);

  // Schedule auto-scroll based on user activity state
  const scheduleAutoScroll = useCallback(() => {
    clearTimeouts();
    
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
          // Don't auto-scroll when actively reading or following (let user control)
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
        isNearBottom: activity.isNearBottom
      });
    }
    
    lastStateRef.current = currentState;
    
    // Schedule auto-scroll when state changes
    scheduleAutoScroll();
  }, [activity.state, activity.isUserActive, activity.shouldAutoScroll, scheduleAutoScroll]);

  // Handle content changes (new messages)
  const handleContentChange = useCallback(() => {
    if (!scrollContainerRef.current) return;
    
    const currentHeight = scrollContainerRef.current.scrollHeight;
    const hasNewContent = currentHeight > lastContentHeightRef.current;
    
    if (hasNewContent) {
      lastContentHeightRef.current = currentHeight;
      console.log('📝 New content detected, current state:', activity.state);
      
      // Always try to schedule auto-scroll when new content arrives
      // The scheduling logic will determine if it should actually execute
      scheduleAutoScroll();
    }
  }, [scrollContainerRef, scheduleAutoScroll, activity.state]);

  // Manual scroll to bottom (for external triggers)
  const scrollToBottomNow = useCallback(() => {
    console.log('🎯 Manual scroll to bottom requested');
    clearTimeouts();
    userInterruptedRef.current = false; // Reset interrupted flag
    executeAutoScroll();
  }, [clearTimeouts, executeAutoScroll]);

  // Graceful scroll to bottom with visual feedback
  const gracefulScrollToBottom = useCallback(() => {
    console.log('🎯 Manual graceful scroll to bottom requested');
    clearTimeouts();
    userInterruptedRef.current = false; // Reset interrupted flag
    executeGracefulReturn();
  }, [clearTimeouts, executeGracefulReturn]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      clearTimeouts();
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
      userInterrupted: userInterruptedRef.current
    });
  }, [activity]);

  return {
    // User activity data
    userActivity: activity,
    
    // Scroll control methods
    handleContentChange,
    scrollToBottomNow,
    gracefulScrollToBottom,
    
    // State information
    isPendingAutoScroll: pendingAutoScrollRef.current,
    
    // Configuration
    config: finalConfig
  };
}
