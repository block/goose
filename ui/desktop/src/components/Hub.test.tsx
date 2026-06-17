/**
 * @vitest-environment jsdom
 */
import React from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';

import Hub from './Hub';
import { IntlTestWrapper } from '../i18n/test-utils';

vi.mock('./ConfigContext', () => ({
  useConfig: () => ({
    extensionsList: [],
  }),
}));

vi.mock('./ChatInput', () => ({
  default: () => <div data-testid="chat-input" />,
}));

vi.mock('./ChatInputCard', () => ({
  ChatInputCard: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));

vi.mock('./LoadingGoose', () => ({
  default: () => null,
}));

vi.mock('../sessions', () => ({
  createSession: vi.fn(),
}));

function mockAppConfig(values: Record<string, unknown>) {
  (window as unknown as Record<string, unknown>).appConfig = {
    get: (key: string) => values[key],
    getAll: () => values,
  };
}

describe('Hub', () => {
  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).appConfig;
  });

  it('shows the packaged fallback preview warning on the empty-chat landing screen', () => {
    mockAppConfig({
      SECURITY_PREVIEW_SESSION_MODE: 'packaged-preview-fallback',
    });

    render(<Hub setView={vi.fn()} />, { wrapper: IntlTestWrapper });

    expect(
      screen.getByText('This local preview was opened outside the supported launcher.')
    ).toBeInTheDocument();
    expect(screen.getByText('./scripts/start-security-packaged-preview.sh')).toBeInTheDocument();
  });
});
