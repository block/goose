import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Message } from '../types/message';
import { IntlTestWrapper } from '../i18n/test-utils';
import ProgressiveMessageList from './ProgressiveMessageList';

const gooseMessageRenderCounts = new Map<string, number>();

vi.mock('./GooseMessage', async () => {
  const React = await import('react');
  const MockGooseMessage = ({ message }: { message: Message }) => {
    const id = message.id ?? 'unknown';
    gooseMessageRenderCounts.set(id, (gooseMessageRenderCounts.get(id) ?? 0) + 1);
    return <div data-testid={`assistant-${id}`}>{id}</div>;
  };
  return { default: React.memo(MockGooseMessage) };
});

vi.mock('./UserMessage', () => ({
  default: ({ message }: { message: Message }) => (
    <div data-testid={`user-${message.id}`}>{message.id}</div>
  ),
}));

function visibleMessage(id: string, role: Message['role'] = 'user'): Message {
  return {
    id,
    role,
    created: 1,
    content: [{ type: 'text', text: id }],
    metadata: { userVisible: true, agentVisible: true },
  };
}

function renderList(messages: Message[], visibleWindow = 2) {
  return render(
    <ProgressiveMessageList
      messages={messages}
      chat={{ sessionId: 'session-1' }}
      isUserMessage={(message) => message.role === 'user'}
      visibleWindow={visibleWindow}
    />,
    { wrapper: IntlTestWrapper }
  );
}

describe('ProgressiveMessageList', () => {
  beforeEach(() => {
    gooseMessageRenderCounts.clear();
  });

  it('renders only the latest window of a long transcript', () => {
    renderList(
      [
        visibleMessage('m1'),
        visibleMessage('m2', 'assistant'),
        visibleMessage('m3'),
        visibleMessage('m4', 'assistant'),
        visibleMessage('m5'),
      ],
      2
    );

    expect(screen.queryByTestId('user-m1')).not.toBeInTheDocument();
    expect(screen.queryByTestId('assistant-m2')).not.toBeInTheDocument();
    expect(screen.queryByTestId('user-m3')).not.toBeInTheDocument();
    expect(screen.getByTestId('assistant-m4')).toBeInTheDocument();
    expect(screen.getByTestId('user-m5')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /3 hidden/i })).toBeInTheDocument();
  });

  it('pages earlier messages in without mounting the full history', async () => {
    const user = userEvent.setup();
    renderList(
      Array.from({ length: 5 }, (_, index) =>
        visibleMessage(`m${index + 1}`, index % 2 === 0 ? 'user' : 'assistant')
      ),
      2
    );

    await user.click(screen.getByRole('button', { name: /3 hidden/i }));

    expect(screen.queryByTestId('user-m1')).not.toBeInTheDocument();
    expect(screen.getByTestId('assistant-m2')).toBeInTheDocument();
    expect(screen.getByTestId('user-m5')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /1 hidden/i })).toBeInTheDocument();
    expect(document.querySelector('[data-transcript-anchor="m4"]')).toBeInTheDocument();
  });

  it('keeps following the live edge until the user expands history', () => {
    const { rerender } = renderList(
      [visibleMessage('m1'), visibleMessage('m2', 'assistant'), visibleMessage('m3')],
      2
    );

    expect(screen.queryByTestId('user-m1')).not.toBeInTheDocument();
    expect(screen.getByTestId('assistant-m2')).toBeInTheDocument();
    expect(screen.getByTestId('user-m3')).toBeInTheDocument();

    rerender(
      <ProgressiveMessageList
        messages={[
          visibleMessage('m1'),
          visibleMessage('m2', 'assistant'),
          visibleMessage('m3'),
          visibleMessage('m4', 'assistant'),
        ]}
        chat={{ sessionId: 'session-1' }}
        isUserMessage={(message) => message.role === 'user'}
        visibleWindow={2}
      />
    );

    expect(screen.queryByTestId('user-m1')).not.toBeInTheDocument();
    expect(screen.queryByTestId('assistant-m2')).not.toBeInTheDocument();
    expect(screen.getByTestId('user-m3')).toBeInTheDocument();
    expect(screen.getByTestId('assistant-m4')).toBeInTheDocument();
  });

  it('keeps an expanded earlier page visible when a new message arrives', async () => {
    const user = userEvent.setup();
    const { rerender } = renderList(
      Array.from({ length: 5 }, (_, index) =>
        visibleMessage(`m${index + 1}`, index % 2 === 0 ? 'user' : 'assistant')
      ),
      2
    );

    await user.click(screen.getByRole('button', { name: /3 hidden/i }));
    expect(screen.getByTestId('assistant-m2')).toBeInTheDocument();

    rerender(
      <ProgressiveMessageList
        messages={Array.from({ length: 6 }, (_, index) =>
          visibleMessage(`m${index + 1}`, index % 2 === 0 ? 'user' : 'assistant')
        )}
        chat={{ sessionId: 'session-1' }}
        isUserMessage={(message) => message.role === 'user'}
        visibleWindow={2}
      />
    );

    expect(screen.getByTestId('assistant-m2')).toBeInTheDocument();
    expect(screen.getByTestId('assistant-m6')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /1 hidden/i })).toBeInTheDocument();
  });

  it('does not remount earlier assistant rows when only the last message streams', () => {
    const first = visibleMessage('m1');
    const earlierAssistant = visibleMessage('m2', 'assistant');
    const streaming = visibleMessage('m3', 'assistant');

    const { rerender } = renderList([first, earlierAssistant, streaming], 3);
    expect(gooseMessageRenderCounts.get('m2')).toBe(1);
    expect(gooseMessageRenderCounts.get('m3')).toBe(1);

    rerender(
      <ProgressiveMessageList
        messages={[first, earlierAssistant, visibleMessage('m3', 'assistant')]}
        chat={{ sessionId: 'session-1' }}
        isUserMessage={(message) => message.role === 'user'}
        visibleWindow={3}
      />
    );

    expect(gooseMessageRenderCounts.get('m2')).toBe(1);
    expect(gooseMessageRenderCounts.get('m3')).toBe(2);
  });

  it('keeps a 241-message session inside the default 80-message window', () => {
    const messages = Array.from({ length: 241 }, (_, index) =>
      visibleMessage(`m${index + 1}`, index % 2 === 0 ? 'user' : 'assistant')
    );

    render(
      <ProgressiveMessageList
        messages={messages}
        chat={{ sessionId: 'session-1' }}
        isUserMessage={(message) => message.role === 'user'}
      />,
      { wrapper: IntlTestWrapper }
    );

    expect(screen.queryByTestId('user-m1')).not.toBeInTheDocument();
    expect(screen.queryByTestId('assistant-m161')).not.toBeInTheDocument();
    expect(screen.getByTestId('assistant-m162')).toBeInTheDocument();
    expect(screen.getByTestId('user-m241')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /161 hidden/i })).toBeInTheDocument();
    expect(screen.getAllByTestId(/^(user|assistant)-m\d+$/)).toHaveLength(80);
  });

  it('does not remount earlier rows while a long session streams', () => {
    const messages = Array.from({ length: 80 }, (_, index) =>
      visibleMessage(`m${index + 1}`, index % 2 === 0 ? 'user' : 'assistant')
    );
    const earlierAssistant = messages[77];
    const streaming = messages[79];

    const { rerender } = renderList(messages, 80);
    expect(gooseMessageRenderCounts.get(earlierAssistant.id ?? '')).toBe(1);
    expect(gooseMessageRenderCounts.get(streaming.id ?? '')).toBe(1);

    const nextMessages = messages.slice(0, -1).concat([visibleMessage('m80', 'assistant')]);
    rerender(
      <ProgressiveMessageList
        messages={nextMessages}
        chat={{ sessionId: 'session-1' }}
        isUserMessage={(message) => message.role === 'user'}
        visibleWindow={80}
      />
    );

    expect(gooseMessageRenderCounts.get(earlierAssistant.id ?? '')).toBe(1);
    expect(gooseMessageRenderCounts.get('m80')).toBe(2);
  });

  it('expands the full transcript when search is triggered', async () => {
    const user = userEvent.setup();
    renderList(
      [
        visibleMessage('m1'),
        visibleMessage('m2', 'assistant'),
        visibleMessage('m3'),
        visibleMessage('m4', 'assistant'),
      ],
      2
    );

    await user.keyboard('{Meta>}f{/Meta}');

    expect(screen.getByTestId('user-m1')).toBeInTheDocument();
    expect(screen.getByTestId('assistant-m4')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /hidden/i })).not.toBeInTheDocument();
  });
});
