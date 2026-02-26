import { render } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { useTextAnimator } from '../use-text-animator';

const splitTypeInstance = {
  split: vi.fn(),
  revert: vi.fn(),
  lines: [],
  chars: [],
};
vi.mock('split-type', () => {
  class SplitTypeMock {
    lines = splitTypeInstance.lines;
    chars = splitTypeInstance.chars;
    split = splitTypeInstance.split;
    revert = splitTypeInstance.revert;
    constructor(_el: Element, _options?: unknown) {}
  }
  return { default: SplitTypeMock };
});
describe('useTextAnimator', () => {
  it('disconnects ResizeObserver and reverts SplitType on unmount', () => {
    Object.defineProperty(window, 'matchMedia', {
      writable: true,
      value: vi.fn(() => ({
        matches: false,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
        addListener: vi.fn(),
        removeListener: vi.fn(),
        dispatchEvent: vi.fn(),
      })),
    });

    const disconnect = vi.fn();
    const observe = vi.fn();

    (globalThis as { ResizeObserver?: unknown }).ResizeObserver = class {
      observe = observe;
      disconnect = disconnect;
      unobserve = vi.fn();
      takeRecords = vi.fn(() => []);
      constructor(_cb: ResizeObserverCallback) {}
    };

    function Test() {
      const ref = useTextAnimator({ text: 'hello' });
      return <div ref={ref}>hello</div>;
    }

    const { unmount } = render(<Test />);
    unmount();

    expect(disconnect).toHaveBeenCalledTimes(1);
    expect(splitTypeInstance.revert).toHaveBeenCalledTimes(1);
  });
});
