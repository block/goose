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

  it('saving an empty string keeps the draft present but empty', () => {
    const { result } = renderHook(() => useChatDraft('hub'));

    act(() => result.current.save('something'));
    act(() => vi.advanceTimersByTime(DRAFT_SAVE_DEBOUNCE_MS));
    act(() => result.current.save(''));
    act(() => vi.advanceTimersByTime(DRAFT_SAVE_DEBOUNCE_MS));

    expect(result.current.read()).toBe('');
    expect(result.current.has()).toBe(true);
  });

  // An effect can run while a write is still waiting out the debounce, so a read
  // that only consulted storage would report the draft as absent and overwrite it.
  it('reads the pending text before the debounce has fired', () => {
    const { result } = renderHook(() => useChatDraft('hub'));

    act(() => result.current.save('still pending'));

    expect(result.current.has()).toBe(true);
    expect(result.current.read()).toBe('still pending');
    expect(SessionDraftStorage.get('hub')).toBe('');
  });

  it('keeps a pending write to itself, so another chat still reads as absent', () => {
    const { result } = renderHook(() => useChatDraft('hub'));
    const other = renderHook(() => useChatDraft('sess-1'));

    act(() => result.current.save('hub pending'));

    expect(other.result.current.has()).toBe(false);
    expect(other.result.current.read()).toBe('');
  });

  it('clear makes the draft absent again, including a pending one', () => {
    const { result } = renderHook(() => useChatDraft('hub'));

    act(() => result.current.save('pending'));
    act(() => result.current.clear());

    expect(result.current.has()).toBe(false);
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
