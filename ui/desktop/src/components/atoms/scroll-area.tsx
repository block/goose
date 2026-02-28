import * as React from 'react';

import { cn } from '@/utils';

type ScrollBehavior = 'auto' | 'smooth';

export interface ScrollAreaHandle {
  scrollToBottom: () => void;
  scrollToPosition: (options: { top: number; behavior?: ScrollBehavior }) => void;
  isAtBottom: () => boolean;
  isFollowing: boolean;
  viewportRef: React.RefObject<HTMLDivElement | null>;
}

interface ScrollAreaProps extends React.HTMLAttributes<HTMLDivElement> {
  autoScroll?: boolean;
  onScrollChange?: (isAtBottom: boolean) => void;
  paddingX?: number;
  paddingY?: number;
  handleScroll?: (viewport: HTMLDivElement) => void;
}

/**
 * Lightweight scroll area wrapper.
 *
 * We intentionally avoid Radix ScrollArea here because React 19 + callback ref
 * composition can trigger a ref/setState loop in certain render paths,
 * manifesting as "Maximum update depth exceeded".
 */
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
      const viewport = viewportRef.current;
      if (!viewport) return false;

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
      const viewport = viewportRef.current;
      if (!viewport) return;

      viewport.scrollTo({ top: viewport.scrollHeight, behavior: 'smooth' });
      setFollowing(true);
      userScrolledUpRef.current = false;
      onScrollChange?.(true);
    }, [onScrollChange, setFollowing]);

    const scrollToPosition = React.useCallback(
      ({ top, behavior = 'smooth' }: { top: number; behavior?: ScrollBehavior }) => {
        const viewport = viewportRef.current;
        if (!viewport) return;

        viewport.scrollTo({ top, behavior });
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

    const lastScrollTopRef = React.useRef(0);

    const onScrollChangeRef = React.useRef(onScrollChange);
    onScrollChangeRef.current = onScrollChange;

    const handleScrollPropRef = React.useRef(handleScrollProp);
    handleScrollPropRef.current = handleScrollProp;

    const handleScroll = React.useCallback(() => {
      const viewport = viewportRef.current;
      if (!viewport) return;

      const currentScrollTop = viewport.scrollTop;
      const hasUserScrolledUp = currentScrollTop < lastScrollTopRef.current;
      lastScrollTopRef.current = currentScrollTop;

      if (hasUserScrolledUp) {
        userScrolledUpRef.current = !isAtBottom();
      }

      const atBottom = isAtBottom();
      setFollowing(atBottom);
      setScrolled(!atBottom);
      onScrollChangeRef.current?.(atBottom);

      isActivelyScrollingRef.current = true;
      if (scrollTimeoutRef.current) {
        window.clearTimeout(scrollTimeoutRef.current);
      }
      scrollTimeoutRef.current = window.setTimeout(() => {
        isActivelyScrollingRef.current = false;
      }, 150);

      if (handleScrollPropRef.current) {
        handleScrollPropRef.current(viewport);
      }
    }, [isAtBottom, setFollowing, setScrolled]);

    React.useEffect(() => {
      const viewport = viewportRef.current;
      if (!viewport) return;

      viewport.addEventListener('scroll', handleScroll, { passive: true });
      return () => {
        viewport.removeEventListener('scroll', handleScroll);
        if (scrollTimeoutRef.current) {
          window.clearTimeout(scrollTimeoutRef.current);
        }
      };
    }, [handleScroll]);

    React.useEffect(() => {
      if (!autoScroll || !viewportRef.current) return;

      const viewport = viewportRef.current;
      const scrollContent = viewport.firstElementChild;
      if (!scrollContent) return;

      const observer = new ResizeObserver(() => {
        const currentScrollHeight = viewport.scrollHeight;
        if (
          currentScrollHeight > lastScrollHeightRef.current &&
          isFollowingRef.current &&
          !userScrolledUpRef.current &&
          !isActivelyScrollingRef.current
        ) {
          requestAnimationFrame(() => {
            const v = viewportRef.current;
            if (v && !isActivelyScrollingRef.current) {
              v.scrollTo({ top: v.scrollHeight, behavior: 'smooth' });
            }
          });
        }
        lastScrollHeightRef.current = currentScrollHeight;
      });

      observer.observe(scrollContent);
      return () => observer.disconnect();
    }, [autoScroll]);

    const paddingStyle: React.CSSProperties = {
      paddingLeft: paddingX ? `${paddingX * 0.25}rem` : undefined,
      paddingRight: paddingX ? `${paddingX * 0.25}rem` : undefined,
      paddingTop: paddingY ? `${paddingY * 0.25}rem` : undefined,
      paddingBottom: paddingY ? `${paddingY * 0.25}rem` : undefined,
    };

    return (
      <div className={cn('relative overflow-hidden', className)} data-scrolled={isScrolled} {...props}>
        <div ref={viewportRef} className="h-full w-full overflow-auto rounded-[inherit]">
          <div style={paddingStyle}>
            {children}
            {autoScroll && <div ref={viewportEndRef} style={{ height: '1px' }} />}
          </div>
        </div>
      </div>
    );
  }
);
ScrollArea.displayName = 'ScrollArea';

const ScrollBar = React.forwardRef<HTMLDivElement, React.HTMLAttributes<HTMLDivElement>>(
  (_props, _ref) => null
);
ScrollBar.displayName = 'ScrollBar';

export { ScrollArea, ScrollBar };
