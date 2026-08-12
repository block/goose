/**
 * E2E-2 — desktop OnboardingGuard gates chat until avocado OAuth completes.
 * covers AC-6, AC-UI
 */
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter } from 'react-router';
import OnboardingGuard from '../OnboardingGuard';

vi.mock('../../../i18n', async () => {
  const actual = await vi.importActual<typeof import('../../../i18n')>('../../../i18n');
  return {
    ...actual,
    useIntl: () => ({
      formatMessage: (
        msg: { defaultMessage?: string; id?: string },
        values?: Record<string, string>
      ) => {
        let text = msg.defaultMessage ?? msg.id ?? '';
        if (values) {
          for (const [k, v] of Object.entries(values)) {
            text = text.replace(`{${k}}`, v);
          }
        }
        return text;
      },
    }),
  };
});

vi.mock('../../ConfigContext', () => ({
  useConfig: () => ({ upsert: vi.fn() }),
}));

vi.mock('../../ModelAndProviderContext', () => ({
  useModelAndProvider: () => ({
    getFallbackModelAndProvider: vi.fn().mockResolvedValue(undefined),
    refreshCurrentModelAndProvider: vi.fn().mockResolvedValue(undefined),
  }),
}));

vi.mock('../../../utils/analytics', () => ({
  trackOnboardingStarted: vi.fn(),
  trackOnboardingCompleted: vi.fn(),
  trackOnboardingProviderSelected: vi.fn(),
  trackTelemetryPreference: vi.fn(),
  setTelemetryEnabled: vi.fn(),
}));

const acpReadDefaults = vi.fn();
const acpListProviderDetails = vi.fn();
const acpSaveDefaults = vi.fn();
const acpAuthenticateProvider = vi.fn();

vi.mock('../../../acp/providers', () => ({
  acpReadDefaults: (...args: unknown[]) => acpReadDefaults(...args),
  acpListProviderDetails: (...args: unknown[]) => acpListProviderDetails(...args),
  acpSaveDefaults: (...args: unknown[]) => acpSaveDefaults(...args),
  acpAuthenticateProvider: (...args: unknown[]) => acpAuthenticateProvider(...args),
}));

vi.mock('../../../contexts/FeaturesContext', () => ({
  useFeatures: () => ({ localInference: false }),
}));

function avocadoProvider(configured: boolean) {
  return {
    name: 'avocado',
    is_configured: configured,
    provider_type: 'Preferred',
    metadata: {
      display_name: 'Avocado LLM API',
      description: 'AVCD OpenAI-compatible LLM gateway',
      default_model: 'anthropic/claude-sonnet-4.6',
      known_models: [],
      config_keys: [
        {
          name: 'AVOCADO_API_KEY',
          required: true,
          secret: true,
          oauth_flow: true,
          device_code_flow: false,
          primary: true,
        },
      ],
    },
  };
}

describe('E2E: OnboardingGuard OAuth - Goal: block chat until avocado Sign in', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.appConfig = {
      get: (key: string) => {
        if (key === 'GOOSE_DEFAULT_PROVIDER') return 'avocado';
        if (key === 'GOOSE_DEFAULT_MODEL') return 'anthropic/claude-sonnet-4.6';
        return undefined;
      },
    } as typeof window.appConfig;

    // Unconfigured: defaults may be empty OR baked but is_configured false.
    acpReadDefaults.mockResolvedValue({ providerId: null, modelId: null });
    acpListProviderDetails.mockResolvedValue([avocadoProvider(false)]);
    acpSaveDefaults.mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('GivenAvocadoUnconfigured_WhenOnboarding_ThenSignInNotChat', async () => {
    // covers AC-6
    render(
      <MemoryRouter>
        <OnboardingGuard>
          <div data-testid="app-children">chat hub</div>
        </OnboardingGuard>
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(screen.queryByTestId('app-children')).toBeNull();
    });

    await waitFor(() => {
      expect(
        screen.getByRole('button', { name: /sign in with avocado/i })
      ).toBeTruthy();
    });
  });

  it('GivenAvocadoConfigured_WhenOnboarding_ThenChildrenRendered', async () => {
    // covers AC-6
    acpReadDefaults.mockResolvedValue({
      providerId: 'avocado',
      modelId: 'anthropic/claude-sonnet-4.6',
    });
    acpListProviderDetails.mockResolvedValue([avocadoProvider(true)]);

    render(
      <MemoryRouter>
        <OnboardingGuard>
          <div data-testid="app-children">chat hub</div>
        </OnboardingGuard>
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(screen.getByTestId('app-children')).toBeTruthy();
    });
  });

  it('GivenAuthenticateForbidden_WhenOnboarding_ThenAccessDenied', async () => {
    // covers AC-UI / AC-4 UX
    const { default: userEvent } = await import('@testing-library/user-event');
    acpAuthenticateProvider.mockRejectedValue(
      new Error('Missing required role: agent-access')
    );

    render(
      <MemoryRouter>
        <OnboardingGuard>
          <div data-testid="app-children">chat hub</div>
        </OnboardingGuard>
      </MemoryRouter>
    );

    const button = await screen.findByRole('button', {
      name: /sign in with avocado/i,
    });
    await userEvent.click(button);

    await waitFor(() => {
      expect(screen.queryByTestId('app-children')).toBeNull();
      expect(screen.getByTestId('onboarding-access-denied')).toBeTruthy();
      expect(screen.getByText(/access denied/i)).toBeTruthy();
    });
  });

  it('GivenBundledDefaultsButUnconfigured_WhenOnboarding_ThenDoesNotSkipSignIn', async () => {
    // covers AC-6 — baked GOOSE_DEFAULT_PROVIDER must not skip Sign in
    acpReadDefaults.mockResolvedValue({ providerId: null, modelId: null });
    acpListProviderDetails.mockResolvedValue([avocadoProvider(false)]);

    render(
      <MemoryRouter>
        <OnboardingGuard>
          <div data-testid="app-children">chat hub</div>
        </OnboardingGuard>
      </MemoryRouter>
    );

    await waitFor(() => {
      expect(screen.queryByTestId('app-children')).toBeNull();
      expect(
        screen.getByRole('button', { name: /sign in with avocado/i })
      ).toBeTruthy();
    });
  });
});
