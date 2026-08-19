import * as React from 'react';
import * as ScrollAreaPrimitive from '@radix-ui/react-scroll-area';

type ScrollBehavior = 'auto' | 'smooth';

import { cn } from '../../utils';
import {
  BOTTOM_SCROLL_THRESHOLD,
  isViewportAtBottom,
  shouldUnfollowOnScroll,
} from '../../utils/scrollFollow';

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
    const lastScrollTopRef = React.useRef(0);
    const isActivelyScrollingRef = React.useRef(false);
    const isProgrammaticScrollRef = React.useRef(false);
    const scrollTimeoutRef = React.useRef<number | null>(null);
    const isFollowingRef = React.useRef(true);

    const isAtBottom = React.useCallback(() => {
      if (!viewportRef.current) return false;
      return isViewportAtBottom(viewportRef.current, BOTTOM_SCROLL_THRESHOLD);
    }, []);

    const pinToLiveEdge = React.useCallback(() => {
      const viewport = viewportRef.current;
      if (!viewport) {
        return;
      }
      isProgrammaticScrollRef.current = true;
      viewport.scrollTo({
        top: viewport.scrollHeight,
        behavior: 'auto',
      });
      lastScrollTopRef.current = viewport.scrollTop;
      lastScrollHeightRef.current = viewport.scrollHeight;
      window.setTimeout(() => {
        isProgrammaticScrollRef.current = false;
      }, 0);
    }, []);

    const scrollToBottom = React.useCallback(() => {
      pinToLiveEdge();
      isFollowingRef.current = true;
      setIsFollowing(true);
      userScrolledUpRef.current = false;
      onScrollChange?.(true);
    }, [onScrollChange, pinToLiveEdge]);

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

    const handleScroll = React.useCallback(() => {
      if (!viewportRef.current) return;

      const viewport = viewportRef.current;
      const { scrollTop } = viewport;
      const currentIsAtBottom = isAtBottom();
      const isProgrammatic = isProgrammaticScrollRef.current;

      if (isProgrammatic) {
        isProgrammaticScrollRef.current = false;
        lastScrollTopRef.current = scrollTop;
        setIsScrolled(scrollTop > 0);
        if (handleScrollProp) {
          handleScrollProp(viewport);
        }
        return;
      }

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

      if (
        shouldUnfollowOnScroll({
          isFollowing: isFollowingRef.current,
          isProgrammatic: false,
          isAtBottom: currentIsAtBottom,
        })
      ) {
        userScrolledUpRef.current = true;
        isFollowingRef.current = false;
        setIsFollowing(false);
        onScrollChange?.(false);
      } else if (currentIsAtBottom && userScrolledUpRef.current) {
        userScrolledUpRef.current = false;
        isFollowingRef.current = true;
        setIsFollowing(true);
        onScrollChange?.(true);
      }

      setIsScrolled(scrollTop > 0);

      if (handleScrollProp) {
        handleScrollProp(viewport);
      }
    }, [isAtBottom, onScrollChange, handleScrollProp]);

    React.useEffect(() => {
      if (!autoScroll || !viewportRef.current) return;

      const viewport = viewportRef.current;
      const currentScrollHeight = viewport.scrollHeight;

      if (
        currentScrollHeight > lastScrollHeightRef.current &&
        isFollowingRef.current &&
        !userScrolledUpRef.current
      ) {
        requestAnimationFrame(() => {
          if (viewportRef.current && isFollowingRef.current && !userScrolledUpRef.current) {
            pinToLiveEdge();
          }
        });
      }

      lastScrollHeightRef.current = currentScrollHeight;
    }, [autoScroll, children, pinToLiveEdge]);

    React.useEffect(() => {
      if (!autoScroll) return;
      const viewport = viewportRef.current;
      const content = contentRef.current;
      if (!viewport || !content || typeof ResizeObserver === 'undefined') return;

      const observer = new ResizeObserver(() => {
        if (isFollowingRef.current && !userScrolledUpRef.current) {
          pinToLiveEdge();
        }
      });
      observer.observe(content);
      return () => observer.disconnect();
    }, [autoScroll, pinToLiveEdge]);

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
          style={{ overflowAnchor: 'none' }}
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
