import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { describe, expect, it } from 'vitest';
import type { Message } from '@/api';
import { ReasoningDetailProvider } from '@/contexts/ReasoningDetailContext';
import GooseMessage from '../GooseMessage';

const JSONL_CARD_SPEC = [
  '{"op":"add","path":"/root","value":"card"}',
  '{"op":"add","path":"/elements/card","value":{"type":"Card","props":{"title":"Total Components","description":"Across all domains"},"children":["value"]}}',
  '{"op":"add","path":"/elements/value","value":{"type":"Heading","props":{"text":"339","level":"h2"},"children":[]}}',
].join('\n');

function assistantMessage(content: Message['content']): Message {
  return {
    role: 'assistant',
    content,
    created: Date.now() / 1000,
    id: 'assistant-1',
    metadata: { agentVisible: true, userVisible: true },
  };
}

describe('GooseMessage: jsonRenderSpec', () => {
  it('renders JsonRenderBlock output from structured jsonRenderSpec content', () => {
    const message = assistantMessage([{ type: 'jsonRenderSpec', spec: JSONL_CARD_SPEC }]);

    render(
      <MemoryRouter>
        <ReasoningDetailProvider>
          <GooseMessage
            sessionId="s1"
            message={message}
            messages={[message]}
            toolCallNotifications={new Map()}
            append={() => {}}
            isStreaming={false}
          />
        </ReasoningDetailProvider>
      </MemoryRouter>
    );

    expect(screen.getByText('Total Components')).toBeInTheDocument();
    expect(screen.getByText('Across all domains')).toBeInTheDocument();
    expect(screen.getByText('339')).toBeInTheDocument();
  });
});
