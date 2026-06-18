/**
 * @vitest-environment jsdom
 */
import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import OnboardingGuard from './OnboardingGuard';
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

vi.mock('../../utils/analytics', () => ({
  trackOnboardingStarted: vi.fn(),
  trackOnboardingCompleted: vi.fn(),
  trackOnboardingProviderSelected: vi.fn(),
  trackTelemetryPreference: vi.fn(),
  setTelemetryEnabled: vi.fn(),
}));

vi.mock('react-router-dom', () => ({
  useNavigate: () => vi.fn(),
}));

vi.mock('../ConfigContext', () => ({
  useConfig: () => ({
    read: vi.fn().mockResolvedValue(null),
    upsert: vi.fn().mockResolvedValue(undefined),
    getProviders: vi.fn().mockResolvedValue([]),
  }),
}));

vi.mock('../ModelAndProviderContext', () => ({
  useModelAndProvider: () => ({
    getFallbackModelAndProvider: vi.fn().mockResolvedValue({ provider: '', model: '' }),
    refreshCurrentModelAndProvider: vi.fn().mockResolvedValue(undefined),
  }),
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

describe('OnboardingGuard', () => {
  beforeEach(() => {
    mockAppConfig({
      GOOSE_DEFAULT_PROVIDER: 'openai',
      GOOSE_DEFAULT_MODEL: 'deepseek-v4-flash',
      GOOSE_PREDEFINED_MODELS: PREDEFINED_MODELS,
    });
  });

  afterEach(() => {
    providersMock.mockClear();
    delete (window as unknown as Record<string, unknown>).appConfig;
  });

  it('shows the built-in TokenPlan onboarding copy for the security distro', async () => {
    render(
      <OnboardingGuard>
        <div>child content</div>
      </OnboardingGuard>,
      { wrapper: IntlTestWrapper }
    );

    await waitFor(() => {
      expect(screen.getByText(/built-in TokenPlan backend/i)).toBeInTheDocument();
    });

    expect(
      screen.getByText(
        'Your local AI agent. 收到 is ready with the built-in TokenPlan backend. Start with DeepSeek V4 Flash and switch models at any time.'
      )
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        'TokenPlan is already connected. Start with DeepSeek V4 Flash and switch models later.'
      )
    ).toBeInTheDocument();
    expect(screen.getByText('Use Built-in Models')).toBeInTheDocument();
    expect(screen.getByText('Advanced Provider Setup')).toBeInTheDocument();
  });

  it('shows a fallback preview warning when the packaged app was opened outside the supported launcher', async () => {
    mockAppConfig({
      GOOSE_DEFAULT_PROVIDER: 'openai',
      GOOSE_DEFAULT_MODEL: 'deepseek-v4-flash',
      GOOSE_PREDEFINED_MODELS: PREDEFINED_MODELS,
      SECURITY_PREVIEW_SESSION_MODE: 'packaged-preview-fallback',
    });

    render(
      <OnboardingGuard>
        <div>child content</div>
      </OnboardingGuard>,
      { wrapper: IntlTestWrapper }
    );

    await waitFor(() => {
      expect(
        screen.getByText('This local preview was opened outside the supported launcher.')
      ).toBeInTheDocument();
    });
  });
});
