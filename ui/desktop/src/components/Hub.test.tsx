/**
 * @vitest-environment jsdom
 */
import { act, render } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import Hub from './Hub';
import { IntlTestWrapper } from '../i18n/test-utils';
import { createSession } from '../sessions';
import { UserInput } from '../types/message';

type ChatInputCapture = {
  draftRef?: { current: string };
  handleSubmit: (input: UserInput) => void;
};

const captured = vi.hoisted(() => ({ chatInput: null as ChatInputCapture | null }));

vi.mock('./ChatInput', () => ({
  default: (props: ChatInputCapture) => {
    captured.chatInput = props;
    return <div data-testid="chat-input" />;
  },
}));

vi.mock('./LoadingGoose', () => ({ default: () => <div /> }));

vi.mock('./ConfigContext', () => ({
  useConfig: () => ({ extensionsList: [] }),
}));

vi.mock('../sessions', () => ({ createSession: vi.fn() }));

vi.mock('../utils/workingDir', () => ({ getInitialWorkingDir: () => '/tmp/goose' }));

vi.mock('../utils/nextChatExtensions', () => ({
  createNextChatExtensionDraft: () => ({}),
  selectNextChatExtensions: () => [],
}));

vi.mock('../acp/errors', () => ({ formatAcpError: (error: unknown) => String(error) }));

vi.mock('../toasts', () => ({ toastError: vi.fn() }));

const DRAFT = 'a half-written thought';
const TYPED_WHILE_STARTING = 'and one more thought';

function renderHub(draftRef: { current: string }) {
  return render(
    <IntlTestWrapper>
      <Hub setView={vi.fn()} draftRef={draftRef} />
    </IntlTestWrapper>
  );
}

describe('Hub', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    captured.chatInput = null;
  });

  it('hands the draft to the input', () => {
    const draftRef = { current: DRAFT };
    renderHub(draftRef);

    expect(captured.chatInput?.draftRef).toBe(draftRef);
  });

  it('puts the text back in the draft when the chat fails to start', async () => {
    vi.mocked(createSession).mockRejectedValue(new Error('no agent'));
    // Submitting drops the draft, and the input keeps showing the text.
    const draftRef = { current: '' };
    renderHub(draftRef);

    await act(async () => {
      captured.chatInput?.handleSubmit({ msg: DRAFT, images: [] });
    });

    expect(draftRef.current).toBe(DRAFT);
  });

  it('keeps text typed while the session was starting', async () => {
    let failCreate!: (error: Error) => void;
    vi.mocked(createSession).mockImplementation(
      () =>
        new Promise<Awaited<ReturnType<typeof createSession>>>((_resolve, reject) => {
          failCreate = reject;
        })
    );
    const draftRef = { current: '' };
    renderHub(draftRef);

    await act(async () => {
      captured.chatInput?.handleSubmit({ msg: DRAFT, images: [] });
    });
    // The input stays editable while the session is being created.
    draftRef.current = TYPED_WHILE_STARTING;

    await act(async () => {
      failCreate(new Error('no agent'));
    });

    expect(draftRef.current).toBe(TYPED_WHILE_STARTING);
  });

  it('leaves the draft cleared when the chat starts', async () => {
    vi.mocked(createSession).mockResolvedValue({ id: 'session-1' } as Awaited<
      ReturnType<typeof createSession>
    >);
    const draftRef = { current: '' };
    renderHub(draftRef);

    await act(async () => {
      captured.chatInput?.handleSubmit({ msg: DRAFT, images: [] });
    });

    expect(draftRef.current).toBe('');
  });
});
