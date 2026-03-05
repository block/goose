import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { WorkBlockIndicator } from '@/components/organisms/messages/WorkBlockIndicator';
import { ReasoningDetailProvider } from '@/contexts/ReasoningDetailContext';

function wrap(ui: React.ReactNode) {
  return <ReasoningDetailProvider>{ui}</ReasoningDetailProvider>;
}

describe('WorkBlockIndicator one-liner', () => {
  it('prefers goose/activity events over assistant json-render text', () => {
    const messages = [
      {
        id: 'm1',
        role: 'assistant',
        content: [
          {
            type: 'text',
            text: '```json-render\n{"op":"add","path":"/root","value":"main"}\n```',
          },
        ],
        created_at: new Date().toISOString(),
      },
    ] as unknown as import('@/api').Message[];

    const activityEvents = new Map<string, unknown[]>([
      [
        'a1',
        [
          {
            type: 'Notification',
            request_id: 'a1',
            message: {
              method: 'goose/activity',
              params: { phase: 'render', text: 'Generating chart…' },
            },
          },
        ],
      ],
    ]);

    render(
      wrap(
        <WorkBlockIndicator
          messages={messages}
          blockId="b1"
          isStreaming={false}
          sessionId="s1"
          showAgentBadge={false}
          activityEvents={activityEvents}
        />
      )
    );

    expect(screen.getByText(/render: Generating chart/i)).toBeInTheDocument();
    expect(screen.queryByText(/json-render/i)).not.toBeInTheDocument();
  });
});
