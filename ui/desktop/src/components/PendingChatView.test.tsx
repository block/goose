import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import PendingChatView from './PendingChatView';
import { IntlTestWrapper } from '../i18n/test-utils';

describe('PendingChatView', () => {
  it('renders the submitted message and loading indicator', () => {
    render(
      <IntlTestWrapper>
        <PendingChatView initialMessage={{ msg: 'hello from hub', images: [] }} />
      </IntlTestWrapper>
    );

    expect(screen.getByTestId('pending-chat')).toBeInTheDocument();
    expect(screen.getByText('hello from hub')).toBeInTheDocument();
    expect(screen.getByTestId('loading-indicator')).toBeInTheDocument();
  });
});
