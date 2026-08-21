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

const messageWithImage: Message = {
  ...message,
  content: [...message.content, { type: 'image', data: 'base64-image', mimeType: 'image/png' }],
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

    render(<UserMessage message={messageWithImage} onMessageUpdate={onMessageUpdate} />, {
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
    const textarea = screen.getByRole('textbox', { name: 'Edit message content' });
    const cancelButton = screen.getByRole('button', { name: 'Cancel editing' });
    const removeImageButton = screen.getByRole('button', { name: 'Remove image from message' });
    expect(textarea).toBeDisabled();
    expect(cancelButton).toBeDisabled();
    expect(removeImageButton).toBeDisabled();
    fireEvent.change(textarea, { target: { value: 'Discard this later change' } });
    fireEvent.keyDown(textarea, { key: 'Escape' });
    expect(textarea).toHaveValue('Keep this edited draft');
    await act(async () => resolveUpdate?.(false));
    await waitFor(() => expect(saveButton).toBeEnabled());
    expect(textarea).toBeEnabled();
    expect(cancelButton).toBeEnabled();
    expect(removeImageButton).toBeEnabled();
    expect(textarea).toHaveValue('Keep this edited draft');
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
