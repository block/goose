import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { IntlTestWrapper } from '../i18n/test-utils';
import type { Message } from '../types/message';
import UserMessage from './UserMessage';

const message: Message = {
  id: 'message-1',
  role: 'user',
  content: [{ type: 'text', text: 'Original message' }],
  created: 0,
  metadata: { agentVisible: true, userVisible: true },
};

describe('UserMessage editing', () => {
  it('keeps the editor and draft open when an update is rejected', async () => {
    Object.assign(window.electron, { logInfo: vi.fn() });
    let resolveUpdate: ((updated: boolean) => void) | undefined;
    const onMessageUpdate = vi.fn().mockReturnValue(
      new Promise<boolean>((resolve) => {
        resolveUpdate = resolve;
      })
    );

    render(<UserMessage message={message} onMessageUpdate={onMessageUpdate} />, {
      wrapper: IntlTestWrapper,
    });

    fireEvent.click(screen.getByRole('button', { name: /Edit message:/ }));
    fireEvent.change(screen.getByRole('textbox', { name: 'Edit message content' }), {
      target: { value: 'Keep this edited draft' },
    });
    const saveButton = screen.getByRole('button', { name: 'Edit message in place' });
    fireEvent.click(saveButton);
    fireEvent.click(saveButton);

    expect(onMessageUpdate).toHaveBeenCalledTimes(1);
    expect(saveButton).toBeDisabled();
    await act(async () => resolveUpdate?.(false));
    await waitFor(() => expect(saveButton).toBeEnabled());
    expect(screen.getByRole('textbox', { name: 'Edit message content' })).toHaveValue(
      'Keep this edited draft'
    );
  });

  it('closes the editor when an update succeeds', async () => {
    Object.assign(window.electron, { logInfo: vi.fn() });
    const onMessageUpdate = vi.fn().mockResolvedValue(true);
    render(<UserMessage message={message} onMessageUpdate={onMessageUpdate} />, {
      wrapper: IntlTestWrapper,
    });

    fireEvent.click(screen.getByRole('button', { name: /Edit message:/ }));
    fireEvent.change(screen.getByRole('textbox', { name: 'Edit message content' }), {
      target: { value: 'Apply this edited draft' },
    });
    fireEvent.click(screen.getByRole('button', { name: 'Edit message in place' }));

    await waitFor(() =>
      expect(
        screen.queryByRole('textbox', { name: 'Edit message content' })
      ).not.toBeInTheDocument()
    );
  });
});
