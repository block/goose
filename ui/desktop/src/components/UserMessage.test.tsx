import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { Message } from '../api';
import { IntlTestWrapper } from '../i18n/test-utils';
import UserMessage from './UserMessage';

const renderWithIntl = (ui: React.ReactElement) => render(ui, { wrapper: IntlTestWrapper });

describe('UserMessage', () => {
  beforeEach(() => {
    window.electron.logInfo = vi.fn();
  });

  it('shows image removal controls without relying on hover', () => {
    const message: Message = {
      id: 'message-1',
      role: 'user',
      created: 1_700_000_000,
      content: [
        { type: 'text', text: 'hello' },
        { type: 'image', data: 'aW1hZ2U=', mimeType: 'image/png' },
      ],
      metadata: { userVisible: true, agentVisible: true },
    };

    renderWithIntl(<UserMessage message={message} onMessageUpdate={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: /edit message/i }));

    const removeImageButton = screen.getByRole('button', { name: /remove image from message/i });

    expect(removeImageButton).not.toHaveClass('opacity-0');
  });
});
