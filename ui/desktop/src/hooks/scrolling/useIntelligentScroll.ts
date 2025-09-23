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
  scrollVelocityThreshold: 0.3, // Lowered for better detection
  autoScrollDelay: 300, // Slightly longer delay
  gracefulReturnDelay: 2000, // Longer delay for graceful return
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
    
    // CRITICAL FIX: Don't auto-scroll if user is actively scrolling or recently interrupted
    if (activity.isUserActive || userInterruptedRef.current) {
      pendingAutoScrollRef.current = false;
      return;
    }
    
    clearTimeouts();
    pendingAutoScrollRef.current = false;
    
    // Use smooth scrolling for better UX
    scrollMethods.scrollToBottom();
  }, [scrollMethods, clearTimeouts, activity.isUserActive]);

  // Schedule auto-scroll based on user activity state
  const scheduleAutoScroll = useCallback(() => {
    clearTimeouts();
    
    // CRITICAL FIX: Reset user interrupted flag when user becomes idle
    if (!activity.isUserActive) {
      userInterruptedRef.current = false;
    }
    
    // Don't auto-scroll if user is active or has recently interrupted
    if (activity.isUserActive || userInterruptedRef.current) {
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
          return finalConfig.autoScrollDelay;
          
        case UserActivityState.IDLE_ABOVE:
          // Graceful return to bottom after longer delay
          return finalConfig.gracefulReturnDelay;
          
        default:
          // Don't auto-scroll when actively reading or following (let user control)
          return -1;
      }
    })();
    
    if (delay >= 0) {
      autoScrollTimeoutRef.current = window.setTimeout(executeAutoScroll, delay);
    }
  }, [activity.shouldAutoScroll, activity.state, activity.isUserActive, finalConfig, executeAutoScroll]);

  // Detect when user interrupts auto-scroll behavior
  useEffect(() => {
    if (activity.state === UserActivityState.ACTIVELY_READING && activity.isUserActive) {
      userInterruptedRef.current = true;
      clearTimeouts();
      pendingAutoScrollRef.current = false;
    }
  }, [activity.state, activity.isUserActive, clearTimeouts]);

  // Handle content changes (new messages)
  const handleContentChange = useCallback(() => {
    if (!scrollContainerRef.current) return;
    
    const currentHeight = scrollContainerRef.current.scrollHeight;
    const hasNewContent = currentHeight > lastContentHeightRef.current;
    
    if (hasNewContent) {
      lastContentHeightRef.current = currentHeight;
      
      // Only schedule auto-scroll if user hasn't interrupted and isn't actively scrolling
      if (!userInterruptedRef.current && !activity.isUserActive) {
        scheduleAutoScroll();
      }
    }
  }, [scrollContainerRef, scheduleAutoScroll, activity.isUserActive]);

  // Manual scroll to bottom (for external triggers)
  const scrollToBottomNow = useCallback(() => {
    clearTimeouts();
    userInterruptedRef.current = false; // Reset interrupted flag
    executeAutoScroll();
  }, [clearTimeouts, executeAutoScroll]);

  // Graceful scroll to bottom with visual feedback
  const gracefulScrollToBottom = useCallback(() => {
    if (!scrollContainerRef.current || !scrollMethods.scrollToPosition) return;
    
    clearTimeouts();
    userInterruptedRef.current = false; // Reset interrupted flag
    
    // Smooth scroll to bottom with custom timing
    const element = scrollContainerRef.current;
    const targetScrollTop = element.scrollHeight - element.clientHeight;
    
    scrollMethods.scrollToPosition({
      top: targetScrollTop,
      behavior: 'smooth'
    });
  }, [scrollContainerRef, scrollMethods, clearTimeouts]);

  // Effect to handle activity state changes
  useEffect(() => {
    scheduleAutoScroll();
  }, [scheduleAutoScroll]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      clearTimeouts();
    };
  }, [clearTimeouts]);

  // Debug logging (can be removed in production)
  useEffect(() => {
    console.log('Intelligent Scroll State:', {
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
