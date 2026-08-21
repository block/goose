import type { ReactNode } from 'react';
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { IntlTestWrapper } from '../i18n/test-utils';
import type { UserInput } from '../types/message';
import type { Session } from '../types/session';
import BaseChat from './BaseChat';

const mocks = vi.hoisted(() => ({
  session: {} as Session,
  submitMessage: vi.fn(),
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
    onSteerQueuedMessage: vi.fn(),
    submitElicitationResponse: vi.fn(),
    stopStreaming: vi.fn(),
    retrySessionLoad: vi.fn(),
    sessionLoadError: undefined,
    tokenState: undefined,
    notifications: [],
    pauseQueueOnStop: false,
    queueProcessingBlocked: false,
    onMessageUpdate: vi.fn(),
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
    initialValue,
    recipeAccepted,
  }: {
    handleSubmit: (input: UserInput) => void;
    initialValue?: string;
    recipeAccepted?: boolean;
  }) => (
    <button
      type="button"
      data-testid="chat-input"
      data-initial-value={initialValue ?? ''}
      data-recipe-accepted={String(recipeAccepted)}
      onClick={() => handleSubmit({ msg: 'chat input', images: [] })}
    >
      chat input
    </button>
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
  default: ({ append }: { append: (text: string) => void }) => (
    <>
      <button type="button" onClick={() => append('progressive descendant')}>
        progressive descendant
      </button>
      <button type="button" onClick={() => append('mcp ui message')}>
        mcp ui message
      </button>
    </>
  ),
}));
vi.mock('./ui/RecipeWarningModal', () => ({
  RecipeWarningModal: ({ isOpen }: { isOpen: boolean }) => (
    <div data-testid="recipe-warning" data-open={String(isOpen)} />
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

function renderBaseChat() {
  return render(
    <BaseChat setChat={vi.fn()} sessionId="sess-1" suppressEmptyState={false} isActiveSession />,
    { wrapper: Wrapper }
  );
}

function invokeAllSubmissionPaths() {
  fireEvent.click(screen.getByRole('button', { name: 'chat input' }));
  fireEvent.click(screen.getByRole('button', { name: 'recipe activity' }));
  fireEvent.click(screen.getByRole('button', { name: 'progressive descendant' }));
  fireEvent.click(screen.getByRole('button', { name: 'mcp ui message' }));
  act(() => mocks.autoSubmit?.({ msg: 'programmatic submit', images: [] }));
}

describe('BaseChat recipe trust gate', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.autoSubmit = undefined;
    mocks.session = makeSession();
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
    expect(screen.getByTestId('recipe-warning')).toHaveAttribute('data-open', 'false');
    invokeAllSubmissionPaths();
    expect(mocks.submitMessage).not.toHaveBeenCalled();

    await act(async () => resolveAcceptance?.(false));
    await waitFor(() =>
      expect(screen.getByTestId('recipe-warning')).toHaveAttribute('data-open', 'true')
    );

    expect(screen.getByTestId('chat-input')).toHaveAttribute('data-initial-value', '');
    expect(screen.getByTestId('chat-input')).toHaveAttribute('data-recipe-accepted', 'false');
    invokeAllSubmissionPaths();
    expect(mocks.submitMessage).not.toHaveBeenCalled();
  });

  it('allows every submission path after trust is affirmatively accepted', async () => {
    mocks.hasAcceptedRecipeBefore.mockResolvedValue(true);
    renderBaseChat();

    await waitFor(() =>
      expect(screen.getByTestId('chat-input')).toHaveAttribute('data-recipe-accepted', 'true')
    );
    expect(screen.getByTestId('chat-input')).toHaveAttribute('data-initial-value', 'RUN_RECIPE');

    invokeAllSubmissionPaths();

    expect(mocks.submitMessage).toHaveBeenCalledTimes(5);
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
    expect(screen.getByTestId('chat-input')).toHaveAttribute('data-initial-value', '');
    invokeAllSubmissionPaths();
    expect(mocks.submitMessage).not.toHaveBeenCalled();
  });

  it('keeps scheduled recipe submissions exempt from the interactive trust gate', () => {
    mocks.session = makeSession({ session_type: 'scheduled' });
    renderBaseChat();

    expect(mocks.hasAcceptedRecipeBefore).not.toHaveBeenCalled();
    expect(screen.getByTestId('chat-input')).toHaveAttribute('data-recipe-accepted', 'true');
    expect(screen.getByTestId('chat-input')).toHaveAttribute('data-initial-value', 'RUN_RECIPE');
    invokeAllSubmissionPaths();
    expect(mocks.submitMessage).toHaveBeenCalledTimes(5);
  });

  it('keeps non-recipe submissions enabled', () => {
    mocks.session = makeSession({ recipe: null });
    renderBaseChat();

    expect(screen.getByTestId('chat-input')).toHaveAttribute('data-recipe-accepted', 'true');
    fireEvent.click(screen.getByRole('button', { name: 'chat input' }));
    act(() => mocks.autoSubmit?.({ msg: 'programmatic submit', images: [] }));
    expect(mocks.submitMessage).toHaveBeenCalledTimes(2);
  });
});
