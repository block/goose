import * as ScrollAreaPrimitive from '@radix-ui/react-scroll-area';
import * as React from 'react';

type ScrollBehavior = 'auto' | 'smooth';

import { cn } from '../../../utils';

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
    const [isFollowing, setIsFollowing] = React.useState(true);
    const isFollowingRef = React.useRef(true);
    const [isScrolled, setIsScrolled] = React.useState(false);
    const isScrolledRef = React.useRef(false);
    const userScrolledUpRef = React.useRef(false);
    const lastScrollHeightRef = React.useRef(0);
    const isActivelyScrollingRef = React.useRef(false);
    const scrollTimeoutRef = React.useRef<number | null>(null);

    const BOTTOM_SCROLL_THRESHOLD = 200;

    const isAtBottom = React.useCallback(() => {
      if (!viewportRef.current) return false;

      const viewport = viewportRef.current;
      const { scrollHeight, scrollTop, clientHeight } = viewport;
      const distanceFromBottom = scrollHeight - scrollTop - clientHeight;

      return distanceFromBottom <= BOTTOM_SCROLL_THRESHOLD;
    }, []);

    const setFollowing = React.useCallback((value: boolean) => {
      if (isFollowingRef.current !== value) {
        isFollowingRef.current = value;
        setIsFollowing(value);
      }
    }, []);

    const setScrolled = React.useCallback((value: boolean) => {
      if (isScrolledRef.current !== value) {
        isScrolledRef.current = value;
        setIsScrolled(value);
      }
    }, []);

    const scrollToBottom = React.useCallback(() => {
      if (viewportRef.current) {
        viewportRef.current.scrollTo({
          top: viewportRef.current.scrollHeight,
          behavior: 'smooth',
        });
        setFollowing(true);
        userScrolledUpRef.current = false;
        onScrollChange?.(true);
      }
    }, [onScrollChange, setFollowing]);

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

    // Stable refs for callbacks used inside handleScroll to avoid re-creating it
    const onScrollChangeRef = React.useRef(onScrollChange);
    onScrollChangeRef.current = onScrollChange;
    const handleScrollPropRef = React.useRef(handleScrollProp);
    handleScrollPropRef.current = handleScrollProp;

    // Handle scroll events — stable callback, reads refs not state
    const handleScroll = React.useCallback(() => {
      if (!viewportRef.current) return;

      const viewport = viewportRef.current;
      const { scrollTop } = viewport;
      const currentIsAtBottom = isAtBottom();

      const scrollDelta = Math.abs(scrollTop - lastScrollTopRef.current);
      if (scrollDelta > 0) {
        isActivelyScrollingRef.current = true;
        if (scrollTimeoutRef.current) {
          clearTimeout(scrollTimeoutRef.current);
        }
        scrollTimeoutRef.current = window.setTimeout(() => {
          isActivelyScrollingRef.current = false;
        }, 100);
      }

      lastScrollTopRef.current = scrollTop;

      if (!currentIsAtBottom && isFollowingRef.current) {
        userScrolledUpRef.current = true;
        setFollowing(false);
        onScrollChangeRef.current?.(false);
      } else if (currentIsAtBottom && userScrolledUpRef.current) {
        userScrolledUpRef.current = false;
        setFollowing(true);
        onScrollChangeRef.current?.(true);
      }

      setScrolled(scrollTop > 0);

      if (handleScrollPropRef.current) {
        handleScrollPropRef.current(viewport);
      }
    }, [isAtBottom, setFollowing, setScrolled]);

    // Auto-scroll when content changes and user is following
    React.useEffect(() => {
      if (!autoScroll || !viewportRef.current) return;

      const viewport = viewportRef.current;
      const currentScrollHeight = viewport.scrollHeight;

      if (
        currentScrollHeight > lastScrollHeightRef.current &&
        isFollowingRef.current &&
        !userScrolledUpRef.current &&
        !isActivelyScrollingRef.current
      ) {
        requestAnimationFrame(() => {
          if (viewportRef.current && !isActivelyScrollingRef.current) {
            viewportRef.current.scrollTo({
              top: viewportRef.current.scrollHeight,
              behavior: 'smooth',
            });
          }
        });
      }

      lastScrollHeightRef.current = currentScrollHeight;
    }, [autoScroll]);

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
          <div className={cn(paddingX ? `px-${paddingX}` : '', paddingY ? `py-${paddingY}` : '')}>
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
    <ScrollAreaPrimitive.ScrollAreaThumb className="relative flex-1 rounded-full bg-border-default dark:bg-background-muted" />
  </ScrollAreaPrimitive.ScrollAreaScrollbar>
));
ScrollBar.displayName = ScrollAreaPrimitive.ScrollAreaScrollbar.displayName;

export { ScrollArea, ScrollBar };
