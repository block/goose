/**
 * @vitest-environment jsdom
 */
import { render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { MemoryRouter } from 'react-router-dom';

import ProviderSettings from './ProviderSettingsPage';
import { IntlTestWrapper } from '../../../i18n/test-utils';

const getProvidersMock = vi.fn().mockResolvedValue([]);

vi.mock('../../ConfigContext', () => ({
  useConfig: () => ({
    getProviders: getProvidersMock,
  }),
}));

vi.mock('./ProviderGrid', () => ({
  default: () => <div data-testid="provider-grid" />,
}));

function mockAppConfig(values: Record<string, unknown>) {
  (window as unknown as Record<string, unknown>).appConfig = {
    get: (key: string) => values[key],
    getAll: () => values,
  };
}

const PREDEFINED_MODELS = JSON.stringify([
  {
    id: 0,
    name: 'auto',
    provider: 'openai',
    alias: 'Auto',
    subtext: 'TokenPlan',
  },
  {
    id: 1,
    name: 'deepseek-v4-flash',
    provider: 'openai',
    alias: 'DeepSeek V4 Flash',
    subtext: '低延迟',
  },
]);

describe('ProviderSettingsPage', () => {
  afterEach(() => {
    getProvidersMock.mockClear();
    delete (window as unknown as Record<string, unknown>).appConfig;
  });

  it('keeps advanced provider setup as an override-only path in single-provider catalog mode', async () => {
    mockAppConfig({
      GOOSE_DEFAULT_PROVIDER: 'openai',
      GOOSE_DEFAULT_MODEL: 'deepseek-v4-flash',
      GOOSE_PREDEFINED_MODELS: PREDEFINED_MODELS,
    });

    render(
      <MemoryRouter>
        <ProviderSettings onClose={vi.fn()} isOnboarding />
      </MemoryRouter>,
      { wrapper: IntlTestWrapper }
    );

    await waitFor(() => {
      expect(screen.getByTestId('provider-selection-heading')).toHaveTextContent(
        'Other providers'
      );
    });

    expect(
      screen.getByText(
        'TokenPlan is already wired in as the built-in backend for this local version. Use this page only if you need to override it with another provider, host, or API key.'
      )
    ).toBeInTheDocument();
  });

  it('shows the fallback preview warning when provider settings are opened from an unsupported packaged launch', async () => {
    mockAppConfig({
      GOOSE_DEFAULT_PROVIDER: 'openai',
      GOOSE_DEFAULT_MODEL: 'deepseek-v4-flash',
      GOOSE_PREDEFINED_MODELS: PREDEFINED_MODELS,
      SECURITY_PREVIEW_SESSION_MODE: 'packaged-preview-fallback',
    });

    render(
      <MemoryRouter>
        <ProviderSettings onClose={vi.fn()} isOnboarding />
      </MemoryRouter>,
      { wrapper: IntlTestWrapper }
    );

    await waitFor(() => {
      expect(
        screen.getByText('This local preview was opened outside the supported launcher.')
      ).toBeInTheDocument();
    });
  });
});
