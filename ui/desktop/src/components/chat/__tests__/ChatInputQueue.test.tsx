import React from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import ChatInput from '../ChatInput';
import { ChatState } from '../../../types/chatState';

vi.mock('../../ui/atoms/Tooltip', () => ({
  Tooltip: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  TooltipTrigger: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  TooltipContent: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock('lodash/debounce', () => {
  return {
    default: (fn: (...args: unknown[]) => void) => {
      const wrapped = (...args: unknown[]) => fn(...args);
      // @ts-expect-error test helper
      wrapped.cancel = vi.fn();
      return wrapped;
    },
  };
});

vi.mock('../../../api', () => ({
  getSession: vi.fn(() => Promise.resolve({ data: { working_dir: '/tmp' } })),
}));

vi.mock('../../../contexts/ConfigContext', () => ({
  useConfig: () => ({
    getProviders: vi.fn(() => Promise.resolve([])),
    read: vi.fn(() => Promise.resolve(null)),
  }),
}));

vi.mock('../../../contexts/ModelAndProviderContext', () => ({
  useModelAndProvider: () => ({
    getCurrentModelAndProvider: vi.fn(() => Promise.resolve({ model: 'gpt-4o', provider: 'openai' })),
  }),
}));

vi.mock('../../../hooks/useAudioRecorder', () => ({
  useAudioRecorder: () => ({
    isEnabled: false,
    dictationProvider: null,
    isRecording: false,
    isTranscribing: false,
    startRecording: vi.fn(),
    stopRecording: vi.fn(),
  }),
}));

vi.mock('../../../hooks/useFileDrop', () => ({
  useFileDrop: () => ({
    droppedFiles: [],
    setDroppedFiles: vi.fn(),
    handleDrop: vi.fn(),
    handleDragOver: vi.fn(),
  }),
}));

vi.mock('../../../utils/localMessageStorage', () => ({
  LocalMessageStorage: {
    addMessage: vi.fn(),
    getRecentMessages: vi.fn(() => []),
  },
}));

vi.mock('../../../utils/workingDir', () => ({
  getInitialWorkingDir: vi.fn(() => '/tmp'),
}));

vi.mock('../../alerts', () => ({
  AlertType: { Info: 'info', Warning: 'warning' },
  useAlerts: () => ({
    alerts: [],
    addAlert: vi.fn(),
    clearAlerts: vi.fn(),
  }),
}));

// Render-only dependencies — keep ChatInput tests focused on queue logic.
vi.mock('../../bottom_menu/BottomMenuAgentSelection', () => ({ BottomMenuAgentSelection: () => null }));
vi.mock('../../bottom_menu/BottomMenuExtensionSelection', () => ({
  BottomMenuExtensionSelection: () => null,
}));
vi.mock('../../bottom_menu/BottomMenuModeSelection', () => ({ BottomMenuModeSelection: () => null }));
vi.mock('../../bottom_menu/CostTracker', () => ({ CostTracker: () => null }));
vi.mock('../../bottom_menu/DirSwitcher', () => ({ DirSwitcher: () => null }));
vi.mock('../../recipes/CreateEditRecipeModal', () => ({ default: () => null }));
vi.mock('../../recipes/CreateRecipeFromSessionModal', () => ({ default: () => null }));
vi.mock('../../settings/models/bottom_bar/ModelsBottomBar', () => ({ default: () => null }));
vi.mock('../../ui/molecules/Diagnostics', () => ({ DiagnosticsModal: () => null }));
vi.mock('./MentionPopover', () => ({ default: () => null }));

describe('ChatInput — interruption queue behavior', () => {
  it('sends the interruption message first, then keeps the remaining queue paused', async () => {
    const handleSubmit = vi.fn();
    const onStop = vi.fn();

    const { rerender } = render(
      <ChatInput
        sessionId="s1"
        handleSubmit={handleSubmit}
        chatState={ChatState.Streaming}
        onStop={onStop}
        setView={vi.fn()}
        toolCount={0}
        messages={[]}
      />
    );

    const textarea = screen.getByTestId('chat-input');

    // Queue a normal message while streaming.
    fireEvent.change(textarea, { target: { value: 'hello' } });
    fireEvent.keyDown(textarea, { key: 'Enter' });

    // Queue an interruption; it should be prioritized.
    fireEvent.change(textarea, { target: { value: 'stop' } });
    fireEvent.keyDown(textarea, { key: 'Enter' });

    expect(onStop).toHaveBeenCalledTimes(1);
    expect(handleSubmit).toHaveBeenCalledTimes(0);

    // Transition out of loading: interruption message should be sent.
    rerender(
      <ChatInput
        sessionId="s1"
        handleSubmit={handleSubmit}
        chatState={ChatState.Idle}
        onStop={onStop}
        setView={vi.fn()}
        toolCount={0}
        messages={[]}
      />
    );

    // Effects are async; wait for the queued interruption to flush.
    await vi.waitFor(() => {
      expect(handleSubmit).toHaveBeenCalledTimes(1);
    });

    expect(handleSubmit).toHaveBeenLastCalledWith({ msg: 'stop', images: [] });

    // Simulate another loading->idle cycle. Since the queue remains paused after interruption,
    // the remaining queued message should not auto-send.
    rerender(
      <ChatInput
        sessionId="s1"
        handleSubmit={handleSubmit}
        chatState={ChatState.Streaming}
        onStop={onStop}
        setView={vi.fn()}
        toolCount={0}
        messages={[]}
      />
    );
    rerender(
      <ChatInput
        sessionId="s1"
        handleSubmit={handleSubmit}
        chatState={ChatState.Idle}
        onStop={onStop}
        setView={vi.fn()}
        toolCount={0}
        messages={[]}
      />
    );

    // Still only the interruption has been dispatched.
    expect(handleSubmit).toHaveBeenCalledTimes(1);
  });
});
