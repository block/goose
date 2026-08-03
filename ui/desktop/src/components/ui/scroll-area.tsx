import * as React from 'react';
import * as ScrollAreaPrimitive from '@radix-ui/react-scroll-area';

type ScrollBehavior = 'auto' | 'smooth';

import { cn } from '../../utils';

export interface ScrollAreaHandle {
  scrollToBottom: () => void;
  scrollToPosition: (options: { top: number; behavior?: ScrollBehavior }) => void;
  isAtBottom: () => boolean;
  isFollowing: boolean;
  viewportRef: React.RefObject<HTMLDivElement | null>;
}

interface ScrollAreaProps extends React.ComponentPropsWithoutRef<typeof ScrollAreaPrimitive.Root> {
  autoScroll?: boolean;
  onScrollChange?: (isAtBottom: boolean) => void;
  /* padding needs to be passed into the container inside ScrollArea to avoid pushing the scrollbar out */
  paddingX?: number;
  paddingY?: number;
  handleScroll?: (viewport: HTMLDivElement) => void;
}

const ScrollArea = React.forwardRef<ScrollAreaHandle, ScrollAreaProps>(
  (
    {
      className,
      children,
      autoScroll = false,
      onScrollChange,
      paddingX,
      paddingY,
      handleScroll: handleScrollProp,
      ...props
    },
    ref
  ) => {
    const rootRef = React.useRef<React.ElementRef<typeof ScrollAreaPrimitive.Root>>(null);
    const viewportRef = React.useRef<HTMLDivElement>(null);
    const viewportEndRef = React.useRef<HTMLDivElement>(null);
    const contentRef = React.useRef<HTMLDivElement>(null);
    const [isFollowing, setIsFollowing] = React.useState(true);
    const [isScrolled, setIsScrolled] = React.useState(false);
    const userScrolledUpRef = React.useRef(false);
    const lastScrollHeightRef = React.useRef(0);
    const isActivelyScrollingRef = React.useRef(false);
    const isProgrammaticScrollRef = React.useRef(false);
    const scrollTimeoutRef = React.useRef<number | null>(null);
    const scrollAnimationFrameRef = React.useRef<number | null>(null);

    const BOTTOM_SCROLL_THRESHOLD = 200;

    const isAtBottom = React.useCallback(() => {
      if (!viewportRef.current) return false;

      const viewport = viewportRef.current;
      const { scrollHeight, scrollTop, clientHeight } = viewport;
      const distanceFromBottom = scrollHeight - scrollTop - clientHeight;

      return distanceFromBottom <= BOTTOM_SCROLL_THRESHOLD;
    }, []);

    const finishProgrammaticScroll = React.useCallback(() => {
      const viewport = viewportRef.current;
      if (viewport) {
        // Snap to the real max scrollTop (scrollHeight alone overshoots and
        // can leave distanceFromBottom > 0 after layout settles).
        viewport.scrollTop = viewport.scrollHeight;
      }
      scrollAnimationFrameRef.current = null;
      setIsFollowing(true);
      userScrolledUpRef.current = false;
      onScrollChange?.(true);
      // Keep ignoring scroll events for a couple of frames so a trailing
      // scroll/layout event cannot immediately re-show the button.
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          isProgrammaticScrollRef.current = false;
        });
      });
    }, [onScrollChange]);

    const scrollToBottom = React.useCallback(() => {
      if (!viewportRef.current) return;

      const viewport = viewportRef.current;
      if (scrollAnimationFrameRef.current !== null) {
        cancelAnimationFrame(scrollAnimationFrameRef.current);
        scrollAnimationFrameRef.current = null;
      }

      const prefersReducedMotion =
        typeof window !== 'undefined' &&
        window.matchMedia('(prefers-reduced-motion: reduce)').matches;

      isProgrammaticScrollRef.current = true;
      userScrolledUpRef.current = false;
      setIsFollowing(true);
      // Hide the scroll-to-bottom control immediately on click — don't wait
      // for the animation to finish.
      onScrollChange?.(true);

      if (prefersReducedMotion) {
        viewport.scrollTop = viewport.scrollHeight;
        finishProgrammaticScroll();
        return;
      }

      const startScroll = viewport.scrollTop;
      const DURATION = 350;
      const startTime = performance.now();

      function easeOutCubic(t: number): number {
        return 1 - Math.pow(1 - t, 3);
      }

      function maxScrollTop() {
        return Math.max(0, viewport.scrollHeight - viewport.clientHeight);
      }

      function animate(currentTime: number) {
        const elapsed = currentTime - startTime;
        const progress = Math.min(elapsed / DURATION, 1);
        // Re-read max scroll each frame so content that grows mid-animation
        // still lands at the true bottom.
        const distance = maxScrollTop() - startScroll;
        viewport.scrollTop = startScroll + distance * easeOutCubic(progress);

        if (progress < 1) {
          scrollAnimationFrameRef.current = requestAnimationFrame(animate);
        } else {
          finishProgrammaticScroll();
        }
      }

      scrollAnimationFrameRef.current = requestAnimationFrame(animate);
    }, [finishProgrammaticScroll, onScrollChange]);

    const scrollToPosition = React.useCallback(
      ({ top, behavior = 'smooth' }: { top: number; behavior?: ScrollBehavior }) => {
        if (viewportRef.current) {
          viewportRef.current.scrollTo({
            top,
            behavior,
          });
        }
      },
      []
    );

    // Expose the scroll methods to parent components
    React.useImperativeHandle(
      ref,
      () => ({
        scrollToBottom,
        scrollToPosition,
        isAtBottom,
        isFollowing,
        viewportRef,
      }),
      [scrollToBottom, scrollToPosition, isAtBottom, isFollowing]
    );

    // track last scroll position to detect user-initiated scrolling
    const lastScrollTopRef = React.useRef(0);

    // Handle scroll events to update isFollowing state
    const handleScroll = React.useCallback(() => {
      if (!viewportRef.current) return;

      const viewport = viewportRef.current;
      const { scrollTop } = viewport;
      const currentIsAtBottom = isAtBottom();

      // Programmatic smooth-scroll must not look like the user scrolled away.
      if (isProgrammaticScrollRef.current) {
        lastScrollTopRef.current = scrollTop;
        setIsScrolled(scrollTop > 0);
        if (handleScrollProp) {
          handleScrollProp(viewport);
        }
        return;
      }

      // detect if this is a user-initiated scroll (position changed from last known position)
      const scrollDelta = Math.abs(scrollTop - lastScrollTopRef.current);
      if (scrollDelta > 0) {
        // Mark that user is actively scrolling immediately
        isActivelyScrollingRef.current = true;

        // clear any existing timeout and set a new one
        if (scrollTimeoutRef.current) {
          clearTimeout(scrollTimeoutRef.current);
        }

        // mark as not actively scrolling
        scrollTimeoutRef.current = window.setTimeout(() => {
          isActivelyScrollingRef.current = false;
        }, 100);
      }

      lastScrollTopRef.current = scrollTop;

      // Detect if user manually scrolled up from the bottom
      if (!currentIsAtBottom && isFollowing) {
        userScrolledUpRef.current = true;
        setIsFollowing(false);
        onScrollChange?.(false);
      } else if (currentIsAtBottom) {
        // Always sync follow + button when we are at the bottom — not only
        // when userScrolledUpRef is set (avoids a stuck scroll-to-bottom btn).
        userScrolledUpRef.current = false;
        if (!isFollowing) {
          setIsFollowing(true);
        }
        onScrollChange?.(true);
      }

      setIsScrolled(scrollTop > 0);

      if (handleScrollProp) {
        handleScrollProp(viewport);
      }
    }, [isAtBottom, isFollowing, onScrollChange, handleScrollProp]);

    // Auto-scroll when content changes and user is following
    React.useEffect(() => {
      if (!autoScroll || !viewportRef.current) return;

      const viewport = viewportRef.current;
      const currentScrollHeight = viewport.scrollHeight;

      // Only auto-scroll if:
      // 1. Content has actually grown (new content added)
      // 2. User was following (at the bottom)
      // 3. User hasn't manually scrolled up
      // 4. User is not actively scrolling
      if (
        currentScrollHeight > lastScrollHeightRef.current &&
        isFollowing &&
        !userScrolledUpRef.current &&
        !isActivelyScrollingRef.current
      ) {
        // Use requestAnimationFrame to ensure DOM has updated
        requestAnimationFrame(() => {
          if (viewportRef.current && !isActivelyScrollingRef.current) {
            viewportRef.current.scrollTo({
              top: viewportRef.current.scrollHeight,
              behavior: 'auto',
            });
          }
        });
      }

      lastScrollHeightRef.current = currentScrollHeight;
    }, [children, autoScroll, isFollowing]);

    // Keep pinned to the bottom when content grows from async media (images,
    // syntax highlighting) that resizes after paint without a React re-render,
    // so it isn't covered by the [children] effect above.
    React.useEffect(() => {
      if (!autoScroll) return;
      const viewport = viewportRef.current;
      const content = contentRef.current;
      if (!viewport || !content || typeof ResizeObserver === 'undefined') return;

      const observer = new ResizeObserver(() => {
        // Mirror the [children] effect's guards, including isActivelyScrolling, so
        // the re-pin doesn't fight a user scrolling up while content is still growing.
        if (isFollowing && !userScrolledUpRef.current && !isActivelyScrollingRef.current) {
          viewport.scrollTo({ top: viewport.scrollHeight, behavior: 'auto' });
        }
      });
      observer.observe(content);
      return () => observer.disconnect();
    }, [autoScroll, isFollowing]);

    // Add scroll event listener
    React.useEffect(() => {
      const viewport = viewportRef.current;
      if (!viewport) return;

      viewport.addEventListener('scroll', handleScroll, { passive: true });
      return () => {
        viewport.removeEventListener('scroll', handleScroll);
        if (scrollTimeoutRef.current) {
          clearTimeout(scrollTimeoutRef.current);
        }
        if (scrollAnimationFrameRef.current !== null) {
          cancelAnimationFrame(scrollAnimationFrameRef.current);
          scrollAnimationFrameRef.current = null;
        }
        isProgrammaticScrollRef.current = false;
      };
    }, [handleScroll]);

    return (
      <ScrollAreaPrimitive.Root
        ref={rootRef}
        className={cn('relative overflow-hidden', className)}
        data-scrolled={isScrolled}
        {...props}
      >
        <div className={cn('absolute top-0 left-0 right-0 z-10 transition-all duration-200')} />
        <ScrollAreaPrimitive.Viewport
          ref={viewportRef}
          className="h-full w-full rounded-[inherit] [&>div]:!block"
        >
          <div
            ref={contentRef}
            className={cn(paddingX ? `px-${paddingX}` : '', paddingY ? `py-${paddingY}` : '')}
          >
            {children}
            {autoScroll && <div ref={viewportEndRef} style={{ height: '1px' }} />}
          </div>
        </ScrollAreaPrimitive.Viewport>
        <ScrollBar />
        <ScrollAreaPrimitive.Corner />
      </ScrollAreaPrimitive.Root>
    );
  }
);
ScrollArea.displayName = ScrollAreaPrimitive.Root.displayName;

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
    <ScrollAreaPrimitive.ScrollAreaThumb className="relative flex-1 rounded-full bg-border-primary dark:bg-background-secondary" />
  </ScrollAreaPrimitive.ScrollAreaScrollbar>
));
ScrollBar.displayName = ScrollAreaPrimitive.ScrollAreaScrollbar.displayName;

export { ScrollArea, ScrollBar };
