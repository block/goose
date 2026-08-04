import { render, act } from '@testing-library/react';
import { createRef } from 'react';
import { describe, it, expect, beforeAll, vi } from 'vitest';

import { ScrollArea, ScrollAreaHandle } from './scroll-area';

function mockGeometry(el: HTMLElement, scrollHeight: number, clientHeight: number) {
  Object.defineProperty(el, 'scrollHeight', { configurable: true, get: () => scrollHeight });
  Object.defineProperty(el, 'clientHeight', { configurable: true, get: () => clientHeight });
}

function scrollTo(el: HTMLElement, top: number) {
  el.scrollTop = top;
  act(() => {
    el.dispatchEvent(new Event('scroll'));
  });
}

describe('ScrollArea auto-follow', () => {
  beforeAll(() => {
    // jsdom does not implement Element.prototype.scrollTo
    if (!Element.prototype.scrollTo) {
      Element.prototype.scrollTo = vi.fn();
    }
  });

  function setup() {
    const ref = createRef<ScrollAreaHandle>();
    const { container } = render(
      <ScrollArea autoScroll ref={ref}>
        <div style={{ height: 1000 }}>content</div>
      </ScrollArea>
    );
    const viewport = container.querySelector('[data-radix-scroll-area-viewport]') as HTMLElement;
    // scrollHeight 1000, clientHeight 500 -> bottom is scrollTop === 500
    mockGeometry(viewport, 1000, 500);
    return { ref, viewport };
  }

  it('stops following when the user scrolls up within the near-bottom threshold', () => {
    const { ref, viewport } = setup();

    // Anchor at the bottom.
    scrollTo(viewport, 500);
    expect(ref.current?.isFollowing).toBe(true);

    // Scroll up 100px: distanceFromBottom (100) is still within the 200px
    // threshold, so isAtBottom() stays true, but the user clearly scrolled up.
    scrollTo(viewport, 400);
    expect(ref.current?.isFollowing).toBe(false);
  });

  it('resumes following once the user returns to the bottom', () => {
    const { ref, viewport } = setup();

    scrollTo(viewport, 500);
    scrollTo(viewport, 400);
    expect(ref.current?.isFollowing).toBe(false);

    // Back to the bottom.
    scrollTo(viewport, 500);
    expect(ref.current?.isFollowing).toBe(true);
  });

  it('stays unfollowed across consecutive upward scroll events within the threshold', () => {
    const { ref, viewport } = setup();

    scrollTo(viewport, 500);
    // A slow scroll-up arrives as several small events, all still within the
    // 200px band. Following must stay off for the whole gesture.
    scrollTo(viewport, 480);
    expect(ref.current?.isFollowing).toBe(false);
    scrollTo(viewport, 460);
    expect(ref.current?.isFollowing).toBe(false);
    scrollTo(viewport, 440);
    expect(ref.current?.isFollowing).toBe(false);
  });
});
