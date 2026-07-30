import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { IntlTestWrapper } from '../i18n/test-utils';
import type {
  NotificationEvent,
  ToolRequestMessageContent,
  ToolResponseMessageContent,
} from '../types/message';
import ToolCallWithResponse from './ToolCallWithResponse';

const toolRequest: ToolRequestMessageContent = {
  type: 'toolRequest',
  id: 'tool-1',
  toolCall: {
    status: 'success',
    value: {
      name: 'developer__shell',
      arguments: {
        command: 'build',
      },
    },
  },
};

const liveOutputNotification: NotificationEvent = {
  type: 'Notification',
  request_id: 'tool-1',
  message: {
    method: 'goose/live_output',
    params: {
      sequence: 1,
      chunks: [
        {
          stream: 'stdout',
          output: 'starting\n',
        },
        {
          stream: 'stderr',
          output: 'checking\n',
        },
      ],
      truncated: false,
    },
  },
};

const toolResponse: ToolResponseMessageContent = {
  type: 'toolResponse',
  id: 'tool-1',
  toolResult: {
    status: 'success',
    value: {
      content: [
        {
          type: 'text',
          text: 'final result',
        },
      ],
      isError: false,
    },
  },
};

function renderToolCall(response?: ToolResponseMessageContent) {
  return render(
    <ToolCallWithResponse
      isCancelledMessage={false}
      toolRequest={toolRequest}
      toolResponse={response}
      notifications={[liveOutputNotification]}
      isStreamingMessage={!response}
      isPendingApproval={false}
    />,
    { wrapper: IntlTestWrapper }
  );
}

describe('ToolCallWithResponse live output', () => {
  beforeEach(() => {
    vi.mocked(window.electron.getSetting).mockResolvedValue('detailed');
  });

  it('renders raw live output while running and replaces it with the final result', async () => {
    const { rerender } = renderToolCall();

    expect(screen.getByText(/starting/)).toHaveTextContent('starting checking');
    expect(screen.queryByText(/stdout|stderr/)).not.toBeInTheDocument();

    rerender(
      <ToolCallWithResponse
        isCancelledMessage={false}
        toolRequest={toolRequest}
        toolResponse={toolResponse}
        notifications={[liveOutputNotification]}
        isStreamingMessage={false}
        isPendingApproval={false}
      />
    );

    expect(screen.queryByText(/starting/)).not.toBeInTheDocument();
    expect(await screen.findByText('final result')).toBeInTheDocument();
  });
});
