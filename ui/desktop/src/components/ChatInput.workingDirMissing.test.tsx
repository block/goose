import { act, render, type RenderOptions, screen, fireEvent } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { IntlTestWrapper } from '../i18n/test-utils';
import { ChatState } from '../types/chatState';
import type { UserInput } from '../types/message';
import ChatInput from './ChatInput';

vi.mock('./alerts', () => ({
  AlertType: { Error: 'error', Warning: 'warning', Info: 'info' },
  useAlerts: () => ({ alerts: [], addAlert: vi.fn(), clearAlerts: vi.fn() }),
}));

vi.mock('./ModelAndProviderContext', () => ({
  useModelAndProvider: () => ({
    getCurrentModelAndProvider: vi.fn().mockResolvedValue({ model: 'm', provider: 'p' }),
    currentModel: 'm',
    currentProvider: 'p',
  }),
}));

const audioRecorderCallbacks = vi.hoisted(
  () => ({}) as { onTranscription?: (text: string) => void }
);

vi.mock('../hooks/useAudioRecorder', () => ({
  useAudioRecorder: (options: { onTranscription: (text: string) => void }) => {
    audioRecorderCallbacks.onTranscription = options.onTranscription;
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

vi.mock('../hooks/useFileDrop', () => ({
  useFileDrop: () => ({
    droppedFiles: [],
    setDroppedFiles: vi.fn(),
    handleDrop: vi.fn(),
    handleDragOver: vi.fn(),
  }),
}));

vi.mock('../acp/providers', () => ({
  acpListProviderDetails: vi.fn().mockResolvedValue([]),
}));

vi.mock('../utils/canonical', () => ({
  fetchCanonicalModelInfo: vi.fn().mockResolvedValue(null),
}));

vi.mock('./settings/models/predefinedModelsUtils', () => ({
  getPredefinedModelsFromEnv: () => [],
}));

vi.mock('../utils/analytics', () => ({
  trackFileAttached: vi.fn(),
  trackVoiceDictation: vi.fn(),
  trackDiagnosticsOpened: vi.fn(),
}));

vi.mock('./MentionPopover', () => ({ default: () => null }));
vi.mock('./MessageQueue', () => ({
  MessageQueue: () => null,
}));
vi.mock('./bottom_menu/DirSwitcher', () => ({ DirSwitcher: () => null }));
vi.mock('./settings/models/bottom_bar/ModelsBottomBar', () => ({ default: () => null }));
vi.mock('./bottom_menu/BottomMenuExtensionSelection', () => ({
  BottomMenuExtensionSelection: () => null,
}));
vi.mock('./bottom_menu/CostTracker', () => ({ CostTracker: () => null }));
vi.mock('./bottom_menu/ContextWindowIndicator', () => ({ ContextWindowIndicator: () => null }));
vi.mock('./ui/Diagnostics', () => ({ DiagnosticsModal: () => null }));

class ResizeObserverStub {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
}
globalThis.ResizeObserver = ResizeObserverStub as unknown as typeof ResizeObserver;

const renderWithIntl = (ui: React.ReactElement, options?: RenderOptions) =>
  render(ui, { wrapper: IntlTestWrapper, ...options });

function baseProps(handleSubmit: (input: UserInput) => void) {
  return {
    sessionId: 'session-1',
    handleSubmit,
    chatState: ChatState.Idle,
    setView: vi.fn(),
    workingDir: '/tmp',
  };
}

describe('ChatInput submissionDisabled (working directory missing)', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('does not submit and preserves typed input when submission is disabled', () => {
    const handleSubmit = vi.fn<(input: UserInput) => void>();
    renderWithIntl(<ChatInput {...baseProps(handleSubmit)} submissionDisabled={true} />);

    const textarea = screen.getByTestId('chat-input') as HTMLTextAreaElement;
    fireEvent.change(textarea, { target: { value: 'blocked message' } });
    fireEvent.keyDown(textarea, { key: 'Enter' });

    expect(handleSubmit).not.toHaveBeenCalled();
    expect(textarea.value).toBe('blocked message');
    expect(screen.getByRole('button', { name: 'Send' })).toBeDisabled();
  });

  it('preserves dictated input when voice auto-submit fires while submission is disabled', () => {
    vi.useFakeTimers();
    try {
      const handleSubmit = vi.fn<(input: UserInput) => void>();
      renderWithIntl(<ChatInput {...baseProps(handleSubmit)} submissionDisabled={true} />);

      act(() => {
        audioRecorderCallbacks.onTranscription?.('deploy the fix submit');
      });
      act(() => {
        vi.runAllTimers();
      });

      expect(handleSubmit).not.toHaveBeenCalled();
      const textarea = screen.getByTestId('chat-input') as HTMLTextAreaElement;
      expect(textarea.value).toBe('deploy the fix');
    } finally {
      vi.useRealTimers();
    }
  });

  it('submits and clears input when submission is enabled', () => {
    const handleSubmit = vi.fn<(input: UserInput) => void>();
    renderWithIntl(<ChatInput {...baseProps(handleSubmit)} submissionDisabled={false} />);

    const textarea = screen.getByTestId('chat-input') as HTMLTextAreaElement;
    fireEvent.change(textarea, { target: { value: 'ready message' } });
    fireEvent.keyDown(textarea, { key: 'Enter' });

    expect(handleSubmit).toHaveBeenCalledWith({ msg: 'ready message', images: [] });
    expect(textarea.value).toBe('');
  });
});
