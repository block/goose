import * as React from 'react';
import * as ScrollAreaPrimitive from '@radix-ui/react-scroll-area';
import { useIntelligentScroll, UserActivityState } from '../../hooks/scrolling';
import { MessageLockIndicator } from './MessageLockIndicator';
import { cn } from '../../utils';

type ScrollBehavior = 'auto' | 'smooth';

export interface ScrollAreaHandle {
  scrollToBottom: () => void;
  scrollToPosition: (options: { top: number; behavior?: ScrollBehavior }) => void;
  getUserActivityState: () => UserActivityState;
  isUserActive: () => boolean;
  lockToMessage: (messageId: string, element?: HTMLElement) => void;
  unlockFromMessage: () => void;
  isLockedToMessage: () => boolean;
  getLockedMessageId: () => string | undefined;
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
  };
  // Callback when content changes (for triggering intelligent scroll)
  onContentChange?: () => void;
  // Callback when message is clicked (for message locking)
  onMessageClick?: (messageId: string, element: HTMLElement) => void;
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
    ...props 
  }, ref) => {
    const rootRef = React.useRef<React.ElementRef<typeof ScrollAreaPrimitive.Root>>(null);
    const viewportRef = React.useRef<HTMLDivElement>(null);
    const viewportEndRef = React.useRef<HTMLDivElement>(null);
    
    // Legacy state for backward compatibility
    const [isFollowing, setIsFollowing] = React.useState(true);
    const [isScrolled, setIsScrolled] = React.useState(false);

    // Scroll methods that can be used by intelligent scroll system
    const scrollMethods = React.useMemo(() => ({
      scrollToBottom: () => {
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
      }
    }), []);

    // Initialize intelligent scrolling if enabled
    const intelligentScrollData = useIntelligentScroll(
      intelligentScroll ? viewportRef : { current: null },
      scrollMethods,
      scrollConfig
    );

    // Handle message clicks for locking - PREVENT DEFAULT SCROLL BEHAVIOR
    const handleMessageClick = React.useCallback((event: MouseEvent) => {
      if (!intelligentScroll) return;
      
      const target = event.target as HTMLElement;
      if (!target) return;
      
      // Find the closest message element (look for data-message-id attribute)
      let messageElement = target.closest('[data-message-id]') as HTMLElement;
      if (!messageElement) {
        // Fallback: look for common message container classes
        messageElement = target.closest('.message, [role="article"], .chat-message, .goose-message, .user-message') as HTMLElement;
      }
      
      if (messageElement) {
        // PREVENT any default scrolling behavior
        event.preventDefault();
        event.stopPropagation();
        
        const messageId = messageElement.getAttribute('data-message-id') || 
                         messageElement.id || 
                         `message-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
        
        console.log('🖱️ Message clicked - locking without scroll:', messageId, messageElement);
        
        // Add visual highlight to the clicked message
        messageElement.style.backgroundColor = 'rgba(59, 130, 246, 0.1)';
        messageElement.style.borderLeft = '3px solid rgb(59, 130, 246)';
        messageElement.style.transition = 'all 0.2s ease';
        
        // Lock to this message WITHOUT scrolling
        intelligentScrollData.lockToMessage(messageId, messageElement);
        
        // Call external handler
        onMessageClick?.(messageId, messageElement);
      }
    }, [intelligentScroll, intelligentScrollData, onMessageClick]);

    // Remove highlight when unlocked
    React.useEffect(() => {
      const lockedElement = intelligentScrollData.userActivity.lockedElement;
      
      if (intelligentScrollData.userActivity.state !== UserActivityState.LOCKED_TO_MESSAGE && lockedElement) {
        // Remove highlight
        lockedElement.style.backgroundColor = '';
        lockedElement.style.borderLeft = '';
        lockedElement.style.transition = '';
      }
    }, [intelligentScrollData.userActivity.state, intelligentScrollData.userActivity.lockedElement]);

    // Set up click event listener for message locking
    React.useEffect(() => {
      if (!intelligentScroll) return;
      
      const viewport = viewportRef.current;
      if (!viewport) return;

      // Use capture phase to prevent other handlers from interfering
      viewport.addEventListener('click', handleMessageClick, { capture: true });
      
      return () => {
        viewport.removeEventListener('click', handleMessageClick, { capture: true });
      };
    }, [intelligentScroll, handleMessageClick]);

    // Legacy scroll handler for backward compatibility
    const handleLegacyScroll = React.useCallback(() => {
      if (!viewportRef.current) return;

      const viewport = viewportRef.current;
      const { scrollHeight, scrollTop, clientHeight } = viewport;

      const scrollBottom = scrollTop + clientHeight;
      const isAtBottom = scrollHeight - scrollBottom <= 10;

      setIsFollowing(isAtBottom);
      setIsScrolled(scrollTop > 0);
    }, []);

    // Track previous scroll height to detect content changes
    const prevScrollHeightRef = React.useRef<number>(0);

    // Handle content changes
    React.useEffect(() => {
      if (!viewportRef.current) return;

      const viewport = viewportRef.current;
      const currentScrollHeight = viewport.scrollHeight;

      // Detect content changes
      if (currentScrollHeight > prevScrollHeightRef.current) {
        prevScrollHeightRef.current = currentScrollHeight;
        
        if (intelligentScroll) {
          // Use intelligent scroll system
          intelligentScrollData.handleContentChange();
        } else if (autoScroll && isFollowing) {
          // Use legacy auto-scroll behavior
          scrollMethods.scrollToBottom();
        }
        
        // Call external content change handler
        onContentChange?.();
      }
    }, [children, autoScroll, intelligentScroll, isFollowing, intelligentScrollData, scrollMethods, onContentChange]);

    // Set up scroll event listener
    React.useEffect(() => {
      const viewport = viewportRef.current;
      if (!viewport) return;

      // Always set up legacy scroll handler for backward compatibility
      viewport.addEventListener('scroll', handleLegacyScroll);
      
      return () => viewport.removeEventListener('scroll', handleLegacyScroll);
    }, [handleLegacyScroll]);

    // Expose methods to parent components
    React.useImperativeHandle(
      ref,
      () => ({
        scrollToBottom: scrollMethods.scrollToBottom,
        scrollToPosition: scrollMethods.scrollToPosition,
        getUserActivityState: () => intelligentScrollData.userActivity.state,
        isUserActive: () => intelligentScrollData.userActivity.isUserActive,
        lockToMessage: intelligentScrollData.lockToMessage,
        unlockFromMessage: intelligentScrollData.unlockFromMessage,
        isLockedToMessage: () => intelligentScrollData.isLockedToMessage,
        getLockedMessageId: () => intelligentScrollData.lockedMessageId,
      }),
      [scrollMethods, intelligentScrollData]
    );

    // Find the locked message element to position the indicator
    const lockedElement = intelligentScrollData.userActivity.lockedElement;
    const isLocked = intelligentScrollData.userActivity.state === UserActivityState.LOCKED_TO_MESSAGE;

    return (
      <ScrollAreaPrimitive.Root
        ref={rootRef}
        className={cn('relative overflow-hidden', className)}
        data-scrolled={isScrolled}
        data-intelligent-scroll={intelligentScroll}
        data-user-state={intelligentScroll ? intelligentScrollData.userActivity.state : undefined}
        data-locked-message={intelligentScroll ? intelligentScrollData.lockedMessageId : undefined}
        {...props}
      >
        {/* Visual indicator for intelligent scroll state (optional, for debugging) */}
        {intelligentScroll && process.env.NODE_ENV === 'development' && (
          <div className="absolute top-2 right-2 z-50 text-xs bg-black/50 text-white px-2 py-1 rounded">
            {intelligentScrollData.userActivity.state}
            {intelligentScrollData.userActivity.isUserActive && ' (active)'}
            {intelligentScrollData.isLockedToMessage && (
              <div className="text-yellow-300">
                🔒 {intelligentScrollData.lockedMessageId?.slice(-8)}
              </div>
            )}
          </div>
        )}
        
        <div className={cn('absolute top-0 left-0 right-0 z-10 transition-all duration-200')} />
        <ScrollAreaPrimitive.Viewport
          ref={viewportRef}
          className="h-full w-full rounded-[inherit] [&>div]:!block"
        >
          <div className={cn(paddingX ? `px-${paddingX}` : '', paddingY ? `py-${paddingY}` : '')}>
            {/* Render children with potential lock indicator injection */}
            {React.Children.map(children, (child, index) => {
              // If this is the locked element, add the indicator after it
              if (React.isValidElement(child) && isLocked && lockedElement) {
                const childElement = child as React.ReactElement;
                
                // Check if this child corresponds to the locked element
                // This is a simplified check - in a real implementation you'd want more robust matching
                const shouldShowIndicator = index === React.Children.count(children) - 1; // Show after last child for now
                
                return (
                  <React.Fragment key={index}>
                    {child}
                    {shouldShowIndicator && intelligentScrollData.lockedMessageId && (
                      <MessageLockIndicator
                        messageId={intelligentScrollData.lockedMessageId}
                        onUnlock={intelligentScrollData.unlockFromMessage}
                        onScrollToBottom={scrollMethods.scrollToBottom}
                      />
                    )}
                  </React.Fragment>
                );
              }
              
              return child;
            })}
            
            {/* Show lock indicator at bottom if locked but couldn't inject it above */}
            {isLocked && intelligentScrollData.lockedMessageId && (
              <MessageLockIndicator
                messageId={intelligentScrollData.lockedMessageId}
                onUnlock={intelligentScrollData.unlockFromMessage}
                onScrollToBottom={scrollMethods.scrollToBottom}
                className="mb-4"
              />
            )}
            
            {(autoScroll || intelligentScroll) && <div ref={viewportEndRef} style={{ height: '1px' }} />}
          </div>
        </ScrollAreaPrimitive.Viewport>
        <ScrollBar />
        <ScrollAreaPrimitive.Corner />
      </ScrollAreaPrimitive.Root>
    );
  }
);
ScrollAreaEnhanced.displayName = 'ScrollAreaEnhanced';

const ScrollBar = React.forwardRef<
  React.ElementRef<typeof ScrollAreaPrimitive.ScrollAreaScrollbar>,
  React.ComponentPropsWithoutRef<typeof ScrollAreaPrimitive.ScrollAreaScrollbar>
>(({ className, orientation = 'vertical', ...props }, ref) => (
  <ScrollAreaPrimitive.ScrollAreaScrollbar
    ref={ref}
    orientation={orientation}
    className={cn(
      'flex touch-none select-none transition-colors',
      orientation === 'vertical' && 'h-full w-2.5 border-l border-l-transparent p-[1px]',
      orientation === 'horizontal' && 'h-2.5 flex-col border-t border-t-transparent p-[1px]',
      className
    )}
    {...props}
  >
    <ScrollAreaPrimitive.ScrollAreaThumb className="relative flex-1 rounded-full bg-border dark:bg-border-dark" />
  </ScrollAreaPrimitive.ScrollAreaScrollbar>
));
ScrollBar.displayName = ScrollAreaPrimitive.ScrollAreaScrollbar.displayName;

export { ScrollAreaEnhanced, ScrollBar };
