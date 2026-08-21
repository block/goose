import { act, fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router';
import { describe, expect, it, vi } from 'vitest';
import { IntlTestWrapper } from '../i18n/test-utils';
import { ChatState } from '../types/chatState';
import ChatInput from './ChatInput';

const mocks = vi.hoisted(() => ({
  onTranscription: undefined as undefined | ((text: string) => void),
}));

vi.stubGlobal(
  'ResizeObserver',
  class {
    observe() {}
    unobserve() {}
    disconnect() {}
  }
);

vi.mock('./ModelAndProviderContext', () => ({
  useModelAndProvider: () => ({
    currentModel: null,
    currentProvider: null,
    getCurrentModelAndProvider: vi.fn().mockResolvedValue(null),
  }),
}));
vi.mock('../hooks/useAudioRecorder', () => ({
  useAudioRecorder: ({ onTranscription }: { onTranscription: (text: string) => void }) => {
    mocks.onTranscription = onTranscription;
    return {
      isEnabled: false,
      dictationProvider: null,
      isRecording: false,
      isTranscribing: false,
      startRecording: vi.fn(),
      stopRecording: vi.fn(),
    };
  },
}));
vi.mock('./bottom_menu/BottomMenuExtensionSelection', () => ({
  BottomMenuExtensionSelection: () => null,
}));
vi.mock('./bottom_menu/DirSwitcher', () => ({ DirSwitcher: () => null }));
vi.mock('./GitBranchIndicator', () => ({ GitBranchIndicator: () => null }));
vi.mock('./settings/models/bottom_bar/ModelsBottomBar', () => ({ default: () => null }));
vi.mock('./bottom_menu/CostTracker', () => ({ CostTracker: () => null }));
vi.mock('./bottom_menu/ContextWindowIndicator', () => ({ ContextWindowIndicator: () => null }));
vi.mock('../utils/conversionUtils', () => ({
  compressImageDataUrl: (dataUrl: string) => Promise.resolve(dataUrl),
}));

describe('ChatInput initial value updates', () => {
  it('preserves an edited draft when the initial value resolves', () => {
    const { rerender } = render(
      <MemoryRouter>
        <IntlTestWrapper>
          <ChatInput
            sessionId="sess-1"
            handleSubmit={vi.fn()}
            chatState={ChatState.Idle}
            setView={vi.fn()}
            initialValue=""
          />
        </IntlTestWrapper>
      </MemoryRouter>
    );

    fireEvent.change(screen.getByTestId('chat-input'), { target: { value: 'Keep my draft' } });

    rerender(
      <MemoryRouter>
        <IntlTestWrapper>
          <ChatInput
            sessionId="sess-1"
            handleSubmit={vi.fn()}
            chatState={ChatState.Idle}
            setView={vi.fn()}
            initialValue="RUN_RECIPE"
          />
        </IntlTestWrapper>
      </MemoryRouter>
    );

    expect(screen.getByTestId('chat-input')).toHaveValue('Keep my draft');
  });

  it('preserves a pasted image when the initial value resolves', async () => {
    const { rerender } = render(
      <MemoryRouter>
        <IntlTestWrapper>
          <ChatInput
            sessionId="sess-1"
            handleSubmit={vi.fn()}
            chatState={ChatState.Idle}
            setView={vi.fn()}
            initialValue=""
          />
        </IntlTestWrapper>
      </MemoryRouter>
    );

    fireEvent.paste(screen.getByTestId('chat-input'), {
      clipboardData: {
        files: [new File(['image'], 'draft.png', { type: 'image/png' })],
        getData: () => '',
      },
    });
    expect(await screen.findByRole('button', { name: 'Remove image' })).toBeInTheDocument();

    rerender(
      <MemoryRouter>
        <IntlTestWrapper>
          <ChatInput
            sessionId="sess-1"
            handleSubmit={vi.fn()}
            chatState={ChatState.Idle}
            setView={vi.fn()}
            initialValue="RUN_RECIPE"
          />
        </IntlTestWrapper>
      </MemoryRouter>
    );

    expect(screen.getByRole('button', { name: 'Remove image' })).toBeInTheDocument();
  });

  it('preserves dictated text when auto-submit is blocked', () => {
    vi.useFakeTimers();
    const handleSubmit = vi.fn();
    render(
      <MemoryRouter>
        <IntlTestWrapper>
          <ChatInput
            sessionId="sess-1"
            handleSubmit={handleSubmit}
            chatState={ChatState.Idle}
            setView={vi.fn()}
            queueProcessingBlocked
          />
        </IntlTestWrapper>
      </MemoryRouter>
    );

    act(() => mocks.onTranscription?.('Keep this message submit'));
    act(() => vi.advanceTimersByTime(100));

    expect(screen.getByTestId('chat-input')).toHaveValue('Keep this message');
    expect(handleSubmit).not.toHaveBeenCalled();
    vi.useRealTimers();
  });
});
