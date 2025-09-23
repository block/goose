/**
 * Intelligent Chat Scrolling System
 * 
 * This module provides hooks and utilities for creating a non-disruptive
 * chat scrolling experience that respects user intent and activity.
 * 
 * Key Features:
 * - User activity detection (scroll, mouse, keyboard, wheel)
 * - Intelligent auto-scroll based on user state
 * - Graceful return to bottom after idle periods
 * - Message locking when user clicks on specific messages
 * - Smooth animations and transitions
 * - Configurable timing and thresholds
 * 
 * Usage:
 * ```tsx
 * const { userActivity, handleContentChange, lockToMessage } = 
 *   useIntelligentScroll(scrollRef, scrollMethods);
 * ```
 */

export { useUserActivity, UserActivityState } from './useUserActivity';
export { useIntelligentScroll } from './useIntelligentScroll';

export type { UserActivityData } from './useUserActivity';
