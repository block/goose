import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { act, renderHook } from '@testing-library/react';
import { useChatDraft, DRAFT_SAVE_DEBOUNCE_MS } from './useChatDraft';
import { SessionDraftStorage } from '../utils/sessionDraftStorage';

describe('useChatDraft', () => {
  beforeEach(() => {
    window.sessionStorage.clear();
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('reads the draft stored for its key', () => {
    SessionDraftStorage.set('hub', 'stored text');

    const { result } = renderHook(() => useChatDraft('hub'));

    expect(result.current.read()).toBe('stored text');
  });

  it('does not write before the debounce elapses', () => {
    const { result } = renderHook(() => useChatDraft('hub'));

    act(() => result.current.save('typing'));
    act(() => vi.advanceTimersByTime(DRAFT_SAVE_DEBOUNCE_MS - 1));

    expect(SessionDraftStorage.get('hub')).toBe('');
  });

  it('writes once the debounce elapses', () => {
    const { result } = renderHook(() => useChatDraft('hub'));

    act(() => result.current.save('typing'));
    act(() => vi.advanceTimersByTime(DRAFT_SAVE_DEBOUNCE_MS));

    expect(SessionDraftStorage.get('hub')).toBe('typing');
  });

  it('keeps only the last keystroke when typing continues', () => {
    const { result } = renderHook(() => useChatDraft('hub'));

    act(() => result.current.save('t'));
    act(() => vi.advanceTimersByTime(300));
    act(() => result.current.save('te'));
    act(() => vi.advanceTimersByTime(300));
    act(() => result.current.save('tex'));
    act(() => vi.advanceTimersByTime(DRAFT_SAVE_DEBOUNCE_MS));

    expect(SessionDraftStorage.get('hub')).toBe('tex');
  });

  // The bug in the implementation removed by #5366: sending within the debounce
  // window let the pending write land after the clear and put the sent text back.
  it('clear drops a write that is still pending', () => {
    const { result } = renderHook(() => useChatDraft('hub'));

    act(() => result.current.save('about to be sent'));
    act(() => result.current.clear());
    act(() => vi.advanceTimersByTime(DRAFT_SAVE_DEBOUNCE_MS * 2));

    expect(SessionDraftStorage.get('hub')).toBe('');
  });

  it('clear removes a draft that was already written', () => {
    const { result } = renderHook(() => useChatDraft('hub'));

    act(() => result.current.save('written'));
    act(() => vi.advanceTimersByTime(DRAFT_SAVE_DEBOUNCE_MS));
    act(() => result.current.clear());

    expect(SessionDraftStorage.get('hub')).toBe('');
  });

  // Submitting from Hub leaves the visible input alone until the navigation to the
  // new session completes, so the clear and the unmount arrive back to back.
  it('does not resurrect a cleared draft when the unmount flush follows', () => {
    const { result, unmount } = renderHook(() => useChatDraft('hub'));

    act(() => result.current.save('sent from hub'));
    act(() => result.current.clear());
    unmount();

    expect(SessionDraftStorage.get('hub')).toBe('');
  });

  it('flushes a pending write on unmount, so leaving right after typing keeps the draft', () => {
    const { result, unmount } = renderHook(() => useChatDraft('hub'));

    act(() => result.current.save('typed then navigated away'));
    unmount();

    expect(SessionDraftStorage.get('hub')).toBe('typed then navigated away');
  });

  it('flushes the pending write under its own key when the key changes', () => {
    const { result, rerender } = renderHook(({ key }) => useChatDraft(key), {
      initialProps: { key: 'hub' },
    });

    act(() => result.current.save('hub text'));
    rerender({ key: 'sess-1' });

    expect(SessionDraftStorage.get('hub')).toBe('hub text');
    expect(SessionDraftStorage.get('sess-1')).toBe('');
  });

  it('saving an empty string removes the draft', () => {
    const { result } = renderHook(() => useChatDraft('hub'));

    act(() => result.current.save('something'));
    act(() => vi.advanceTimersByTime(DRAFT_SAVE_DEBOUNCE_MS));
    act(() => result.current.save(''));
    act(() => vi.advanceTimersByTime(DRAFT_SAVE_DEBOUNCE_MS));

    expect(SessionDraftStorage.get('hub')).toBe('');
  });

  it('writes each chat under its own key', () => {
    const hub = renderHook(() => useChatDraft('hub'));
    const session = renderHook(() => useChatDraft('sess-1'));

    act(() => hub.result.current.save('hub text'));
    act(() => session.result.current.save('session text'));
    act(() => vi.advanceTimersByTime(DRAFT_SAVE_DEBOUNCE_MS));

    expect(SessionDraftStorage.get('hub')).toBe('hub text');
    expect(SessionDraftStorage.get('sess-1')).toBe('session text');
  });
});
