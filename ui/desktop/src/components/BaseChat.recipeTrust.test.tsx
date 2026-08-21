import type { ReactNode } from 'react';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { IntlTestWrapper } from '../i18n/test-utils';
import type { ImageData, UserInput } from '../types/message';
import type { Session } from '../types/session';
import BaseChat from './BaseChat';

const mocks = vi.hoisted(() => ({
  session: {} as Session,
  submitMessage: vi.fn(),
  steerMessage: vi.fn(),
  updateMessage: vi.fn(),
  elicitationResponse: vi.fn(),
  hasAcceptedRecipeBefore: vi.fn(),
  recordRecipeHash: vi.fn(),
  autoSubmit: undefined as undefined | ((input: UserInput) => void),
}));

vi.mock('../hooks/useChatSession', () => ({
  useChatSession: () => ({
    session: mocks.session,
    messages: [],
    chatState: 'idle',
    progressMessage: undefined,
    updateSession: vi.fn(),
    handleSubmit: mocks.submitMessage,
    onSteerQueuedMessage: mocks.steerMessage,
    submitElicitationResponse: mocks.elicitationResponse,
    stopStreaming: vi.fn(),
    retrySessionLoad: vi.fn(),
    sessionLoadError: undefined,
    tokenState: undefined,
    notifications: [],
    pauseQueueOnStop: false,
    queueProcessingBlocked: false,
    onMessageUpdate: mocks.updateMessage,
  }),
}));

vi.mock('../hooks/useAutoSubmit', () => ({
  useAutoSubmit: ({ handleSubmit }: { handleSubmit: (input: UserInput) => void }) => {
    mocks.autoSubmit = handleSubmit;
    return { hasAutoSubmitted: false };
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
vi.mock('../hooks/use-mobile', () => ({ useIsMobile: () => false }));
vi.mock('../hooks/useNavigation', () => ({ useNavigation: () => vi.fn() }));
vi.mock('./Layout/NavigationContext', () => ({
  useNavigationContextSafe: () => ({ isNavExpanded: true }),
}));
vi.mock('../acp/sessions', () => ({
  acpDeleteSession: vi.fn(),
  acpUpdateWorkingDir: vi.fn(),
}));
vi.mock('../recipe', () => ({
  scanRecipe: vi.fn().mockResolvedValue({ has_security_warnings: false }),
}));
vi.mock('../acp/acpConnection', () => ({
  isAcpRecovering: false,
  subscribeToAcpRecovery: () => () => undefined,
}));

vi.mock('./ChatInput', () => ({
  default: ({
    handleSubmit,
    onSteerQueuedMessage,
    initialValue,
    recipeAccepted,
    queueProcessingBlocked,
  }: {
    handleSubmit: (input: UserInput) => void;
    onSteerQueuedMessage?: (input: UserInput) => Promise<boolean>;
    initialValue?: string;
    recipeAccepted?: boolean;
    queueProcessingBlocked?: boolean;
  }) => (
    <>
      <button
        type="button"
        data-testid="chat-input"
        data-initial-value={initialValue ?? ''}
        data-recipe-accepted={String(recipeAccepted)}
        data-queue-processing-blocked={String(queueProcessingBlocked)}
        onClick={() => handleSubmit({ msg: 'chat input', images: [] })}
      >
        chat input
      </button>
      <button
        type="button"
        onClick={() => void onSteerQueuedMessage?.({ msg: 'steer queued message', images: [] })}
      >
        steer queued message
      </button>
    </>
  ),
}));
vi.mock('./recipes/RecipeActivities', () => ({
  default: ({ append }: { append: (text: string) => void }) => (
    <button type="button" onClick={() => append('recipe activity')}>
      recipe activity
    </button>
  ),
}));
vi.mock('./ProgressiveMessageList', () => ({
  default: ({
    append,
    onMessageUpdate,
    submitElicitationResponse,
  }: {
    append: (text: string) => void;
    onMessageUpdate?: (
      messageId: string,
      newContent: string,
      editType: 'fork' | 'edit',
      retainedImages: ImageData[]
    ) => Promise<boolean>;
    submitElicitationResponse?: (
      elicitationId: string,
      userData: Record<string, unknown>
    ) => Promise<boolean>;
  }) => (
    <>
      <button type="button" onClick={() => append('progressive descendant')}>
        progressive descendant
      </button>
      <button type="button" onClick={() => append('mcp ui message')}>
        mcp ui message
      </button>
      <button
        type="button"
        onClick={() => onMessageUpdate?.('message-1', 'edited message', 'edit', [])}
      >
        edit message
      </button>
      <button
        type="button"
        onClick={() => void submitElicitationResponse?.('elicitation-1', { response: 'approved' })}
      >
        submit elicitation
      </button>
    </>
  ),
}));
vi.mock('./ui/RecipeWarningModal', () => ({
  RecipeWarningModal: ({ isOpen, onConfirm }: { isOpen: boolean; onConfirm: () => void }) => (
    <div data-testid="recipe-warning" data-open={String(isOpen)}>
      {isOpen && (
        <button type="button" onClick={onConfirm}>
          accept recipe
        </button>
      )}
    </div>
  ),
}));
vi.mock('./Layout/MainPanelLayout', () => ({
  MainPanelLayout: ({ children }: { children: ReactNode }) => <>{children}</>,
}));
vi.mock('./ChatInputCard', () => ({
  ChatInputCard: ({ children }: { children: ReactNode }) => <>{children}</>,
}));
vi.mock('./ui/scroll-area', () => ({
  ScrollArea: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));
vi.mock('./conversation/SearchView', () => ({
  SearchView: ({ children }: { children: ReactNode }) => <>{children}</>,
}));
vi.mock('./LoadingGoose', () => ({ default: () => null }));
vi.mock('./RecipeHeader', () => ({ RecipeHeader: () => null }));
vi.mock('./icons', () => ({ Goose: () => null }));
vi.mock('./GooseSidebar/EnvironmentBadge', () => ({ default: () => null }));
vi.mock('./SessionActionsHeader', () => ({ default: () => null }));

function makeSession(overrides: Partial<Session> = {}): Session {
  return {
    id: 'sess-1',
    name: 'Recipe session',
    message_count: 0,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    working_dir: '/tmp',
    extension_data: {},
    session_type: 'user',
    recipe: {
      title: 'Untrusted recipe',
      description: 'Runs a prompt',
      prompt: 'RUN_RECIPE',
    },
    ...overrides,
  } as Session;
}

function Wrapper({ children }: { children: ReactNode }) {
  return (
    <MemoryRouter initialEntries={['/pair?resumeSessionId=sess-1']}>
      <IntlTestWrapper>{children}</IntlTestWrapper>
    </MemoryRouter>
  );
}

function renderBaseChat(isActiveSession = true) {
  return render(
    <BaseChat
      setChat={vi.fn()}
      sessionId="sess-1"
      suppressEmptyState={false}
      isActiveSession={isActiveSession}
    />,
    { wrapper: Wrapper }
  );
}

function invokeAllSubmissionPaths() {
  fireEvent.click(screen.getByRole('button', { name: 'chat input' }));
  fireEvent.click(screen.getByRole('button', { name: 'recipe activity' }));
  fireEvent.click(screen.getByRole('button', { name: 'progressive descendant' }));
  fireEvent.click(screen.getByRole('button', { name: 'mcp ui message' }));
  fireEvent.click(screen.getByRole('button', { name: 'edit message' }));
  fireEvent.click(screen.getByRole('button', { name: 'submit elicitation' }));
  fireEvent.click(screen.getByRole('button', { name: 'steer queued message' }));
  act(() => mocks.autoSubmit?.({ msg: 'programmatic submit', images: [] }));
}

describe('BaseChat recipe trust gate', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.autoSubmit = undefined;
    mocks.session = makeSession();
    mocks.steerMessage.mockResolvedValue(true);
    mocks.updateMessage.mockResolvedValue(true);
    mocks.elicitationResponse.mockResolvedValue(true);
    Object.assign(window.electron, {
      hasAcceptedRecipeBefore: mocks.hasAcceptedRecipeBefore,
      recordRecipeHash: mocks.recordRecipeHash,
    });
  });

  it('blocks direct, descendant, and programmatic submissions while trust is pending or rejected', async () => {
    let resolveAcceptance: ((accepted: boolean) => void) | undefined;
    mocks.hasAcceptedRecipeBefore.mockReturnValue(
      new Promise<boolean>((resolve) => {
        resolveAcceptance = resolve;
      })
    );

    renderBaseChat();

    expect(screen.getByTestId('chat-input')).toHaveAttribute('data-initial-value', '');
    expect(screen.getByTestId('chat-input')).toHaveAttribute('data-recipe-accepted', 'false');
    expect(screen.getByTestId('chat-input')).toHaveAttribute(
      'data-queue-processing-blocked',
      'true'
    );
    expect(screen.getByTestId('recipe-warning')).toHaveAttribute('data-open', 'false');
    invokeAllSubmissionPaths();
    expect(mocks.submitMessage).not.toHaveBeenCalled();
    expect(mocks.steerMessage).not.toHaveBeenCalled();
    expect(mocks.updateMessage).not.toHaveBeenCalled();
    expect(mocks.elicitationResponse).not.toHaveBeenCalled();

    await act(async () => resolveAcceptance?.(false));
    await waitFor(() =>
      expect(screen.getByTestId('recipe-warning')).toHaveAttribute('data-open', 'true')
    );

    expect(screen.getByTestId('chat-input')).toHaveAttribute('data-initial-value', '');
    expect(screen.getByTestId('chat-input')).toHaveAttribute('data-recipe-accepted', 'false');
    expect(screen.getByTestId('chat-input')).toHaveAttribute(
      'data-queue-processing-blocked',
      'true'
    );
    invokeAllSubmissionPaths();
    expect(mocks.submitMessage).not.toHaveBeenCalled();
    expect(mocks.steerMessage).not.toHaveBeenCalled();
    expect(mocks.updateMessage).not.toHaveBeenCalled();
    expect(mocks.elicitationResponse).not.toHaveBeenCalled();
  });

  it('allows every submission path after trust is affirmatively accepted', async () => {
    mocks.hasAcceptedRecipeBefore.mockResolvedValue(true);
    renderBaseChat();

    await waitFor(() =>
      expect(screen.getByTestId('chat-input')).toHaveAttribute('data-recipe-accepted', 'true')
    );
    expect(screen.getByTestId('chat-input')).toHaveAttribute('data-initial-value', 'RUN_RECIPE');
    expect(screen.getByTestId('chat-input')).toHaveAttribute(
      'data-queue-processing-blocked',
      'false'
    );

    invokeAllSubmissionPaths();

    expect(mocks.submitMessage).toHaveBeenCalledTimes(5);
    expect(mocks.steerMessage).toHaveBeenCalledTimes(1);
    expect(mocks.updateMessage).toHaveBeenCalledTimes(1);
    expect(mocks.elicitationResponse).toHaveBeenCalledTimes(1);
  });

  it('returns to pending before a different recipe in the same session can submit', async () => {
    mocks.hasAcceptedRecipeBefore.mockResolvedValueOnce(true);
    const { rerender } = renderBaseChat();

    await waitFor(() =>
      expect(screen.getByTestId('chat-input')).toHaveAttribute('data-recipe-accepted', 'true')
    );

    mocks.submitMessage.mockClear();
    mocks.hasAcceptedRecipeBefore.mockReturnValue(new Promise<boolean>(() => undefined));
    mocks.session = makeSession({
      recipe: {
        title: 'Different recipe',
        description: 'Runs a different prompt',
        prompt: 'RUN_DIFFERENT_RECIPE',
      },
    });
    rerender(
      <BaseChat setChat={vi.fn()} sessionId="sess-1" suppressEmptyState={false} isActiveSession />
    );

    expect(screen.getByTestId('chat-input')).toHaveAttribute('data-recipe-accepted', 'false');
    expect(screen.getByTestId('chat-input')).toHaveAttribute(
      'data-queue-processing-blocked',
      'true'
    );
    expect(screen.getByTestId('chat-input')).toHaveAttribute('data-initial-value', '');
    invokeAllSubmissionPaths();
    expect(mocks.submitMessage).not.toHaveBeenCalled();
    expect(mocks.steerMessage).not.toHaveBeenCalled();
    expect(mocks.updateMessage).not.toHaveBeenCalled();
    expect(mocks.elicitationResponse).not.toHaveBeenCalled();
  });

  it('retains accepted trust for background work while the same recipe session is inactive', async () => {
    mocks.hasAcceptedRecipeBefore.mockResolvedValue(true);
    const { rerender } = renderBaseChat();

    await waitFor(() =>
      expect(screen.getByTestId('chat-input')).toHaveAttribute('data-recipe-accepted', 'true')
    );

    mocks.submitMessage.mockClear();
    mocks.steerMessage.mockClear();
    rerender(
      <BaseChat
        setChat={vi.fn()}
        sessionId="sess-1"
        suppressEmptyState={false}
        isActiveSession={false}
      />
    );

    expect(screen.getByTestId('chat-input')).toHaveAttribute('data-recipe-accepted', 'true');
    expect(screen.getByTestId('chat-input')).toHaveAttribute(
      'data-queue-processing-blocked',
      'false'
    );
    invokeAllSubmissionPaths();
    expect(mocks.submitMessage).toHaveBeenCalledTimes(5);
    expect(mocks.steerMessage).toHaveBeenCalledTimes(1);
    expect(mocks.updateMessage).toHaveBeenCalledTimes(1);
    expect(mocks.elicitationResponse).toHaveBeenCalledTimes(1);
  });

  it('fails closed on lookup failure and accepts only the current recipe when persistence fails', async () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => undefined);
    mocks.hasAcceptedRecipeBefore.mockRejectedValue(new Error('trust lookup failed'));
    mocks.recordRecipeHash.mockRejectedValue(new Error('trust persistence failed'));
    const { rerender } = renderBaseChat();

    await waitFor(() =>
      expect(screen.getByTestId('recipe-warning')).toHaveAttribute('data-open', 'true')
    );
    invokeAllSubmissionPaths();

    expect(mocks.submitMessage).not.toHaveBeenCalled();
    expect(mocks.steerMessage).not.toHaveBeenCalled();
    expect(mocks.updateMessage).not.toHaveBeenCalled();
    expect(mocks.elicitationResponse).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole('button', { name: 'accept recipe' }));
    await waitFor(() =>
      expect(screen.getByTestId('chat-input')).toHaveAttribute('data-recipe-accepted', 'true')
    );
    invokeAllSubmissionPaths();

    expect(mocks.recordRecipeHash).toHaveBeenCalledWith(mocks.session.recipe);
    expect(mocks.submitMessage).toHaveBeenCalledTimes(5);
    expect(mocks.steerMessage).toHaveBeenCalledTimes(1);
    expect(mocks.updateMessage).toHaveBeenCalledTimes(1);
    expect(mocks.elicitationResponse).toHaveBeenCalledTimes(1);
    expect(consoleError).toHaveBeenCalledWith('Failed to check recipe trust:', expect.any(Error));
    expect(consoleError).toHaveBeenCalledWith('Failed to persist recipe trust:', expect.any(Error));

    mocks.submitMessage.mockClear();
    mocks.steerMessage.mockClear();
    mocks.updateMessage.mockClear();
    mocks.elicitationResponse.mockClear();
    mocks.hasAcceptedRecipeBefore.mockReturnValue(new Promise<boolean>(() => undefined));
    mocks.session = makeSession({
      recipe: {
        title: 'Different recipe',
        description: 'Runs a different prompt',
        prompt: 'RUN_DIFFERENT_RECIPE',
      },
    });
    rerender(
      <BaseChat setChat={vi.fn()} sessionId="sess-1" suppressEmptyState={false} isActiveSession />
    );

    expect(screen.getByTestId('chat-input')).toHaveAttribute('data-recipe-accepted', 'false');
    invokeAllSubmissionPaths();
    expect(mocks.submitMessage).not.toHaveBeenCalled();
    expect(mocks.steerMessage).not.toHaveBeenCalled();
    expect(mocks.updateMessage).not.toHaveBeenCalled();
    expect(mocks.elicitationResponse).not.toHaveBeenCalled();
    consoleError.mockRestore();
  });

  it('keeps scheduled recipe submissions exempt from the interactive trust gate', () => {
    mocks.session = makeSession({ session_type: 'scheduled' });
    renderBaseChat();

    expect(mocks.hasAcceptedRecipeBefore).not.toHaveBeenCalled();
    expect(screen.getByTestId('chat-input')).toHaveAttribute('data-recipe-accepted', 'true');
    expect(screen.getByTestId('chat-input')).toHaveAttribute('data-initial-value', 'RUN_RECIPE');
    expect(screen.getByTestId('chat-input')).toHaveAttribute(
      'data-queue-processing-blocked',
      'false'
    );
    invokeAllSubmissionPaths();
    expect(mocks.submitMessage).toHaveBeenCalledTimes(5);
    expect(mocks.steerMessage).toHaveBeenCalledTimes(1);
    expect(mocks.updateMessage).toHaveBeenCalledTimes(1);
    expect(mocks.elicitationResponse).toHaveBeenCalledTimes(1);
  });

  it('keeps non-recipe submissions enabled', () => {
    mocks.session = makeSession({ recipe: null });
    renderBaseChat();

    expect(screen.getByTestId('chat-input')).toHaveAttribute('data-recipe-accepted', 'true');
    expect(screen.getByTestId('chat-input')).toHaveAttribute(
      'data-queue-processing-blocked',
      'false'
    );
    fireEvent.click(screen.getByRole('button', { name: 'chat input' }));
    fireEvent.click(screen.getByRole('button', { name: 'steer queued message' }));
    act(() => mocks.autoSubmit?.({ msg: 'programmatic submit', images: [] }));
    expect(mocks.submitMessage).toHaveBeenCalledTimes(2);
    expect(mocks.steerMessage).toHaveBeenCalledTimes(1);
  });
});
