import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router';
import { describe, expect, it, vi } from 'vitest';
import { IntlTestWrapper } from '../i18n/test-utils';
import { ChatState } from '../types/chatState';
import ChatInput from './ChatInput';

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
  useAudioRecorder: () => ({
    isEnabled: false,
    dictationProvider: null,
    isRecording: false,
    isTranscribing: false,
    startRecording: vi.fn(),
    stopRecording: vi.fn(),
  }),
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
});
