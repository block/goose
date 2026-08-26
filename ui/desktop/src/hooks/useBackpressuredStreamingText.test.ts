import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  STREAMING_RENDER_COOLDOWN_MS,
  useBackpressuredStreamingText,
} from './useBackpressuredStreamingText';

describe('useBackpressuredStreamingText', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('coalesces updates received during the cooldown', () => {
    const { result, rerender } = renderHook(
      ({ content }) => useBackpressuredStreamingText(content, true),
      { initialProps: { content: 'first' } }
    );

    rerender({ content: 'first second' });
    rerender({ content: 'first second third' });

    expect(result.current).toBe('first');

    act(() => vi.advanceTimersByTime(STREAMING_RENDER_COOLDOWN_MS));

    expect(result.current).toBe('first second third');
  });

  it('publishes immediately when the previous cooldown has elapsed', () => {
    const { result, rerender } = renderHook(
      ({ content }) => useBackpressuredStreamingText(content, true),
      { initialProps: { content: 'first' } }
    );

    act(() => vi.advanceTimersByTime(STREAMING_RENDER_COOLDOWN_MS));
    rerender({ content: 'first second' });

    expect(result.current).toBe('first second');
  });

  it('uses a longer cooldown when requested', () => {
    const longerCooldownMs = 250;
    const { result, rerender } = renderHook(
      ({ content }) => useBackpressuredStreamingText(content, true, longerCooldownMs),
      { initialProps: { content: 'first' } }
    );

    rerender({ content: 'first second' });
    act(() => vi.advanceTimersByTime(STREAMING_RENDER_COOLDOWN_MS));
    expect(result.current).toBe('first');

    act(() => vi.advanceTimersByTime(longerCooldownMs - STREAMING_RENDER_COOLDOWN_MS));
    expect(result.current).toBe('first second');
  });

  it('returns the latest content immediately when backpressure is disabled', () => {
    const { result, rerender } = renderHook(
      ({ content, enabled }) => useBackpressuredStreamingText(content, enabled),
      { initialProps: { content: 'first', enabled: true } }
    );

    rerender({ content: 'first second', enabled: true });
    expect(result.current).toBe('first');

    rerender({ content: 'first second', enabled: false });
    expect(result.current).toBe('first second');
    expect(vi.getTimerCount()).toBe(0);
  });

  it('cancels the cooldown on unmount', () => {
    const { unmount } = renderHook(() => useBackpressuredStreamingText('first', true));

    expect(vi.getTimerCount()).toBe(1);
    unmount();
    expect(vi.getTimerCount()).toBe(0);
  });
});
