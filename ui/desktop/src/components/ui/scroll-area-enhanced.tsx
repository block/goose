import * as React from 'react';
import * as ScrollAreaPrimitive from '@radix-ui/react-scroll-area';
import { useIntelligentScroll, UserActivityState } from '../../hooks/scrolling';
import { MessageLockIndicator } from './MessageLockIndicator';
import { cn } from '../../utils';

// type ScrollBehavior = 'auto' | 'smooth'; // Use native ScrollBehavior

export interface ScrollAreaHandle {
  scrollToBottom: () => void;
  scrollToPosition: (options: { top: number; behavior?: ScrollBehavior }) => void;
  getUserActivityState: () => UserActivityState;
  isUserActive: () => boolean;
  lockToMessage: (messageId: string, element?: HTMLElement) => void;
  unlockFromMessage: () => void;
  isLockedToMessage: () => boolean;
  getLockedMessageId: () => string | undefined;
  // NEW: Stream following information
  isFollowingStream: () => boolean;
}

interface ScrollAreaEnhancedProps extends React.ComponentPropsWithoutRef<typeof ScrollAreaPrimitive.Root> {
  autoScroll?: boolean;
  intelligentScroll?: boolean; // Enable intelligent scrolling behavior
  /* padding needs to be passed into the container inside ScrollArea to avoid pushing the scrollbar out */
  paddingX?: number;
  paddingY?: number;
  // Intelligent scroll configuration
  scrollConfig?: {
    idleTimeout?: number;
    activityDebounce?: number;
    scrollVelocityThreshold?: number;
    messageLockTimeout?: number;
    autoScrollDelay?: number;
    gracefulReturnDelay?: number;
    // NEW: Stream following config
    streamFollowingEnabled?: boolean;
    streamFollowingThreshold?: number;
  };
  // Callback when content changes (for triggering intelligent scroll)
  onContentChange?: () => void;
  // Callback when message is clicked (for message locking)
  onMessageClick?: (messageId: string, element: HTMLElement) => void;
  // NEW: Stream state for intelligent scrolling
  isStreamingMessage?: boolean;
}

const ScrollAreaEnhanced = React.forwardRef<ScrollAreaHandle, ScrollAreaEnhancedProps>(
  ({ 
    className, 
    children, 
    autoScroll = false, 
    intelligentScroll = false,
    paddingX, 
    paddingY, 
    scrollConfig = {},
    onContentChange,
    onMessageClick,
    isStreamingMessage = false, // NEW: Stream state
    ...props 
  }, ref) => {
    const rootRef = React.useRef<React.ElementRef<typeof ScrollAreaPrimitive.Root>>(null);
    const viewportRef = React.useRef<HTMLDivElement>(null);
    const viewportEndRef = React.useRef<HTMLDivElement>(null);
    const lockIndicatorRef = React.useRef<HTMLDivElement>(null);
    
    // Legacy state for backward compatibility
    const [isFollowing, setIsFollowing] = React.useState(true);
    // const [isScrolled, setIsScrolled] = React.useState(false); // Unused

    // Initialize intelligent scrolling if enabled - NOW WITH STREAM STATE
    const intelligentScrollData = useIntelligentScroll(
      intelligentScroll ? viewportRef : { current: null },
      // Pass scroll methods that include unlock behavior
      {
        scrollToBottom: () => {
          // UNLOCK when going to bottom - user explicitly navigating away
          if (intelligentScrollData?.isLockedToMessage) {
            console.log('🎯 Go to Bottom: Unlocking message before scroll');
            intelligentScrollData.unlockFromMessage();
          }
          
          if (viewportEndRef.current) {
            viewportEndRef.current.scrollIntoView({
              behavior: 'smooth',
              block: 'end',
              inline: 'nearest',
            });
            setIsFollowing(true);
          }
        },
        scrollToPosition: ({ top, behavior = 'smooth' }: { top: number; behavior?: ScrollBehavior }) => {
          if (viewportRef.current) {
            viewportRef.current.scrollTo({
              top,
              behavior,
            });
          }
        },
      },
      scrollConfig,
      isStreamingMessage // NEW: Pass streaming state to intelligent scroll hook
    );

    // Handle content changes for intelligent scrolling
    React.useEffect(() => {
      if (intelligentScroll && intelligentScrollData) {
        intelligentScrollData.handleContentChange();
      }
      onContentChange?.();
    }, [children, intelligentScroll, intelligentScrollData, onContentChange]);

    // Legacy auto-scroll behavior (when intelligent scrolling is disabled)
    React.useEffect(() => {
      if (!autoScroll || intelligentScroll) return;

      const timer = setTimeout(() => {
        if (isFollowing && viewportEndRef.current) {
          viewportEndRef.current.scrollIntoView({
            behavior: 'smooth',
            block: 'end',
            inline: 'nearest',
          });
        }
      }, 100);

      return () => clearTimeout(timer);
    }, [children, autoScroll, isFollowing, intelligentScroll]);

    // Handle scroll events for legacy behavior
    const handleScroll = React.useCallback((event: React.UIEvent<HTMLDivElement>) => {
      if (intelligentScroll) return; // Skip legacy behavior when intelligent scrolling is enabled

      const target = event.target as HTMLDivElement;
      const { scrollTop, scrollHeight, clientHeight } = target;
      const isAtBottom = scrollHeight - scrollTop <= clientHeight + 10;
      // const hasScrolled = scrollTop > 0; // Unused

      setIsFollowing(isAtBottom);
      // setIsScrolled(hasScrolled); // Unused
    }, [intelligentScroll]);

    // Enhanced message click handler with intelligent scrolling
    const handleMessageClick = React.useCallback((event: React.MouseEvent<HTMLDivElement>) => {
      if (!intelligentScroll || !intelligentScrollData) {
        onMessageClick?.('unknown', event.currentTarget);
        return;
      }

      // CRITICAL: Prevent any scroll behavior when clicking messages
      event.preventDefault();
      event.stopPropagation();
      // event.stopImmediatePropagation(); // Not available in React

      // Find the message element and ID
      const messageElement = event.currentTarget.closest('[data-message-id]') as HTMLElement;
      if (!messageElement) {
        console.warn('🖱️ Message click: Could not find message element with data-message-id');
        return;
      }

      const messageId = messageElement.getAttribute('data-message-id');
      if (!messageId) {
        console.warn('🖱️ Message click: Could not find message ID');
        return;
      }

      console.log('🖱️ Message clicked:', messageId);
      
      // Add visual highlight to the clicked message
      messageElement.style.backgroundColor = "rgba(59, 130, 246, 0.1)";
      messageElement.style.borderLeft = "3px solid rgb(59, 130, 246)";
      messageElement.style.transition = "all 0.2s ease";      
      // Lock to this message to prevent auto-scroll
      intelligentScrollData.lockToMessage(messageId, messageElement);
      
      // Call the callback
      onMessageClick?.(messageId, messageElement);
    }, [intelligentScroll, intelligentScrollData, onMessageClick]);

    // Expose methods via ref
    React.useImperativeHandle(ref, () => ({
      scrollToBottom: () => {
        if (intelligentScroll && intelligentScrollData) {
          intelligentScrollData.scrollToBottomNow();
        } else if (viewportEndRef.current) {
          viewportEndRef.current.scrollIntoView({
            behavior: 'smooth',
            block: 'end',
            inline: 'nearest',
          });
          setIsFollowing(true);
        }
      },
      scrollToPosition: ({ top, behavior = 'smooth' }: { top: number; behavior?: ScrollBehavior }) => {
        if (viewportRef.current) {
          viewportRef.current.scrollTo({
            top,
            behavior,
          });
        }
      },
      getUserActivityState: () => {
        return intelligentScrollData?.userActivity?.state || UserActivityState.IDLE_AT_BOTTOM;
      },
      isUserActive: () => {
        return intelligentScrollData?.userActivity?.isUserActive || false;
      },
      lockToMessage: (messageId: string, element?: HTMLElement) => {
        if (intelligentScroll && intelligentScrollData) {
          intelligentScrollData.lockToMessage(messageId, element);
        }
      },
      unlockFromMessage: () => {
        if (intelligentScroll && intelligentScrollData) {
          intelligentScrollData.unlockFromMessage();
        }
      },
      isLockedToMessage: () => {
        return intelligentScrollData?.isLockedToMessage || false;
      },
      getLockedMessageId: () => {
        return intelligentScrollData?.lockedMessageId;
      },
      // NEW: Stream following information
      isFollowingStream: () => {
        return intelligentScrollData?.isFollowingStream || false;
      },
    }), [intelligentScroll, intelligentScrollData]);

    return (
      <ScrollAreaPrimitive.Root
        ref={rootRef}
        className={cn('relative overflow-hidden', className)}
        {...props}
      >
        <ScrollAreaPrimitive.Viewport
          ref={viewportRef}
          className="h-full w-full rounded-[inherit]"
          onScroll={handleScroll}
          onClick={intelligentScroll ? handleMessageClick : undefined}
        >
          <div 
            className={cn(
              'min-h-full',
              paddingX !== undefined && `px-${paddingX}`,
              paddingY !== undefined && `py-${paddingY}`
            )}
            style={{
              paddingLeft: paddingX !== undefined ? `${paddingX * 0.25}rem` : undefined,
              paddingRight: paddingX !== undefined ? `${paddingX * 0.25}rem` : undefined,
              paddingTop: paddingY !== undefined ? `${paddingY * 0.25}rem` : undefined,
              paddingBottom: paddingY !== undefined ? `${paddingY * 0.25}rem` : undefined,
            }}
          >
            {children}
            {/* Invisible element at the end for scroll-to-bottom functionality */}
            <div ref={viewportEndRef} className="h-0" />
          </div>
        </ScrollAreaPrimitive.Viewport>
        
        {/* Message Lock Indicator */}
        {intelligentScroll && intelligentScrollData?.isLockedToMessage && (
          <MessageLockIndicator
            ref={lockIndicatorRef}
            messageId={intelligentScrollData.lockedMessageId!}
            onUnlock={intelligentScrollData.unlockFromMessage}
            isStreamingMessage={isStreamingMessage} // NEW: Pass streaming state
            isFollowingStream={intelligentScrollData.isFollowingStream} // NEW: Pass following state
          />
        )}
        
        <ScrollAreaPrimitive.Scrollbar
          orientation="vertical"
          className="flex touch-none select-none transition-colors"
        >
          <ScrollAreaPrimitive.Thumb className="relative flex-1 rounded-full bg-border" />
        </ScrollAreaPrimitive.Scrollbar>
        <ScrollAreaPrimitive.Corner />
      </ScrollAreaPrimitive.Root>
    );
  }
);

ScrollAreaEnhanced.displayName = 'ScrollAreaEnhanced';

export { ScrollAreaEnhanced };
