/**
 * @vitest-environment jsdom
 */
import React from 'react';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import ProviderSelector from './ProviderSelector';
import { IntlTestWrapper } from '../../i18n/test-utils';

const providersMock = vi.fn().mockResolvedValue({ data: [] });

vi.mock('../../api', () => ({
  providers: (...args: unknown[]) => providersMock(...args),
  createCustomProvider: vi.fn(),
}));

vi.mock('../ui/Select', () => ({
  Select: () => <div data-testid="provider-select" />,
}));

vi.mock('../ui/dialog', () => ({
  Dialog: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  DialogContent: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  DialogHeader: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  DialogTitle: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));

vi.mock('../settings/providers/modal/subcomponents/forms/CustomProviderForm', () => ({
  default: () => null,
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

describe('ProviderSelector', () => {
  afterEach(() => {
    providersMock.mockClear();
    delete (window as unknown as Record<string, unknown>).appConfig;
  });

  it('uses the built-in backend as the primary onboarding path for single-provider catalogs', async () => {
    mockAppConfig({
      GOOSE_DEFAULT_PROVIDER: 'openai',
      GOOSE_DEFAULT_MODEL: 'deepseek-v4-flash',
      GOOSE_PREDEFINED_MODELS: PREDEFINED_MODELS,
    });
    const onConfigured = vi.fn().mockResolvedValue(undefined);

    render(<ProviderSelector onConfigured={onConfigured} />, { wrapper: IntlTestWrapper });

    fireEvent.click(screen.getByText('Use Built-in Models'));

    await waitFor(() => {
      expect(onConfigured).toHaveBeenCalledWith('openai', 'deepseek-v4-flash');
    });

    expect(
      screen.getByText(
        'TokenPlan is already connected. Start with DeepSeek V4 Flash and switch models later.'
      )
    ).toBeInTheDocument();
    expect(screen.queryByText('Choose an option to get started.')).not.toBeInTheDocument();
  });
});
