/**
 * @vitest-environment jsdom
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook } from '@testing-library/react';
import { MemoryRouter } from 'react-router';
import type { PropsWithChildren } from 'react';
import { useAutoSubmit } from './useAutoSubmit';
import { ChatState } from '../types/chatState';
import type { UserInput } from '../types/message';
import type { Session } from '../types/session';

function makeSession(overrides: Partial<Session> = {}): Session {
  return {
    id: 'sess-1',
    name: 'untitled',
    message_count: 0,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    working_dir: '/tmp',
    extension_data: { active: [], installed: [] },
    ...overrides,
  } as Session;
}

const initialMessage: UserInput = {
  msg: 'Run the recipe',
  images: [],
};

describe('useAutoSubmit', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('does not auto-submit while recipe acceptance is unresolved', () => {
    const handleSubmit = vi.fn();
    const dispatchEventSpy = vi.spyOn(window, 'dispatchEvent');

    const wrapper = ({ children }: PropsWithChildren) => (
      <MemoryRouter initialEntries={['/pair?resumeSessionId=sess-1']}>{children}</MemoryRouter>
    );

    renderHook(
      () =>
        useAutoSubmit({
          sessionId: 'sess-1',
          session: makeSession(),
          messages: [],
          chatState: ChatState.Idle,
          initialMessage,
          canAutoSubmit: false,
          handleSubmit,
        }),
      { wrapper }
    );

    expect(handleSubmit).not.toHaveBeenCalled();
    expect(dispatchEventSpy).not.toHaveBeenCalled();
  });

  it('keeps the initial message while blocked and submits it once unblocked', async () => {
    const handleSubmit = vi.fn().mockResolvedValue(true);
    const dispatchEventSpy = vi.spyOn(window, 'dispatchEvent');

    const wrapper = ({ children }: PropsWithChildren) => (
      <MemoryRouter initialEntries={['/pair?resumeSessionId=sess-1']}>{children}</MemoryRouter>
    );

    const { rerender } = renderHook(
      ({ canAutoSubmit }) =>
        useAutoSubmit({
          sessionId: 'sess-1',
          session: makeSession(),
          messages: [],
          chatState: ChatState.Idle,
          initialMessage,
          canAutoSubmit,
          handleSubmit,
        }),
      {
        initialProps: { canAutoSubmit: false },
        wrapper,
      }
    );

    expect(handleSubmit).not.toHaveBeenCalled();
    expect(dispatchEventSpy).not.toHaveBeenCalled();

    rerender({ canAutoSubmit: true });

    await vi.waitFor(() => {
      expect(handleSubmit).toHaveBeenCalledTimes(1);
    });
    expect(handleSubmit).toHaveBeenCalledWith(initialMessage);
    expect(dispatchEventSpy).toHaveBeenCalledTimes(1);
  });

  it('retries Hub initial message when handleSubmit skips (returns false)', async () => {
    const handleSubmit = vi.fn().mockResolvedValueOnce(false).mockResolvedValueOnce(true);
    const dispatchEventSpy = vi.spyOn(window, 'dispatchEvent');

    const wrapper = ({ children }: PropsWithChildren) => (
      <MemoryRouter initialEntries={['/pair?resumeSessionId=sess-1']}>{children}</MemoryRouter>
    );

    const { rerender } = renderHook(
      ({ chatState }) =>
        useAutoSubmit({
          sessionId: 'sess-1',
          session: makeSession(),
          messages: [],
          chatState,
          initialMessage,
          canAutoSubmit: true,
          handleSubmit,
        }),
      {
        initialProps: { chatState: ChatState.Idle },
        wrapper,
      }
    );

    await vi.waitFor(() => {
      expect(handleSubmit).toHaveBeenCalledTimes(1);
    });
    expect(dispatchEventSpy).not.toHaveBeenCalled();

    // Simulate recovery: leave Idle then return so the effect re-runs
    rerender({ chatState: ChatState.LoadingConversation });
    rerender({ chatState: ChatState.Idle });

    await vi.waitFor(() => {
      expect(handleSubmit).toHaveBeenCalledTimes(2);
    });
    expect(dispatchEventSpy).toHaveBeenCalledTimes(1);
  });
});
