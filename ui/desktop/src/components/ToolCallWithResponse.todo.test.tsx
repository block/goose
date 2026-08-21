/**
 * @vitest-environment jsdom
 */
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { IntlTestWrapper } from '../i18n/test-utils';
import type { ToolRequestMessageContent } from '../types/message';
import ToolCallWithResponse from './ToolCallWithResponse';

const todoRequest: ToolRequestMessageContent = {
  type: 'toolRequest',
  id: 'todo-1',
  toolCall: {
    status: 'success',
    value: {
      name: 'todo__todo_write',
      arguments: { content: '- [ ] Persist this task' },
    },
  },
};

describe('ToolCallWithResponse todo rendering', () => {
  it('does not report a cancelled todo write as successful', () => {
    render(
      <ToolCallWithResponse
        isCancelledMessage
        toolRequest={todoRequest}
        isPendingApproval={false}
        isStreamingMessage={false}
      />,
      { wrapper: IntlTestWrapper }
    );

    expect(screen.getByText('Failed to update task list')).toBeVisible();
    expect(screen.queryByText(/Task list updated/)).not.toBeInTheDocument();
  });
});
