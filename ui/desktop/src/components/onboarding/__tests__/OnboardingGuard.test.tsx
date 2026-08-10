import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { IntlTestWrapper } from '../../../i18n/test-utils';
import OnboardingGuard from '../OnboardingGuard';

const mocks = vi.hoisted(() => ({
  navigate: vi.fn(),
  upsert: vi.fn(),
  refreshCurrentModelAndProvider: vi.fn(),
  trackTelemetryPreference: vi.fn(),
  setTelemetryEnabled: vi.fn(),
}));

vi.mock('react-router', () => ({
  useNavigate: () => mocks.navigate,
}));

vi.mock('../../ConfigContext', () => ({
  useConfig: () => ({ upsert: mocks.upsert }),
}));

vi.mock('../../ModelAndProviderContext', () => ({
  useModelAndProvider: () => ({
    getFallbackModelAndProvider: vi.fn().mockResolvedValue({ provider: '', model: '' }),
    refreshCurrentModelAndProvider: mocks.refreshCurrentModelAndProvider,
  }),
}));

vi.mock('../../../acp/providers', () => ({
  acpReadDefaults: vi.fn().mockResolvedValue({ providerId: null, modelId: null }),
  acpListProviderDetails: vi.fn().mockResolvedValue([
    {
      name: 'test-provider',
      metadata: { display_name: 'Test Provider', default_model: 'test-model' },
    },
  ]),
  acpSaveDefaults: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('../../../utils/analytics', () => ({
  trackOnboardingStarted: vi.fn(),
  trackOnboardingCompleted: vi.fn(),
  trackOnboardingProviderSelected: vi.fn(),
  trackTelemetryPreference: mocks.trackTelemetryPreference,
  setTelemetryEnabled: mocks.setTelemetryEnabled,
}));

vi.mock('../ProviderSelector', () => ({
  default: ({ onConfigured }: { onConfigured: (provider: string, model: string) => void }) => (
    <button onClick={() => onConfigured('test-provider', 'test-model')}>Configure</button>
  ),
}));

vi.mock('../OnboardingSuccess', () => ({
  default: ({ onFinish }: { onFinish: (enabled: boolean) => void }) => (
    <>
      <button onClick={() => onFinish(false)}>Decline telemetry</button>
      <button onClick={() => onFinish(true)}>Accept telemetry</button>
    </>
  ),
}));

async function reachTelemetryChoice() {
  const user = userEvent.setup();
  render(
    <IntlTestWrapper>
      <OnboardingGuard>
        <div>Protected application</div>
      </OnboardingGuard>
    </IntlTestWrapper>
  );
  await user.click(await screen.findByRole('button', { name: 'Configure' }));
  return user;
}

describe('OnboardingGuard telemetry preference', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.refreshCurrentModelAndProvider.mockResolvedValue(undefined);
  });

  it('keeps onboarding retryable when an opt-out cannot be persisted', async () => {
    mocks.upsert.mockRejectedValue(new Error('config write failed'));
    const user = await reachTelemetryChoice();

    await user.click(await screen.findByRole('button', { name: 'Decline telemetry' }));

    await waitFor(() =>
      expect(mocks.upsert).toHaveBeenCalledWith('GOOSE_TELEMETRY_ENABLED', false, false)
    );
    expect(mocks.navigate).not.toHaveBeenCalled();
    expect(mocks.trackTelemetryPreference).not.toHaveBeenCalled();
    expect(mocks.setTelemetryEnabled).not.toHaveBeenCalled();
    expect(screen.getByRole('button', { name: 'Decline telemetry' })).toBeInTheDocument();
    expect(screen.queryByText('Protected application')).not.toBeInTheDocument();
  });

  it('enters the application after persisting an opt-out', async () => {
    mocks.upsert.mockResolvedValue(undefined);
    const user = await reachTelemetryChoice();

    await user.click(await screen.findByRole('button', { name: 'Decline telemetry' }));

    await screen.findByText('Protected application');
    expect(mocks.navigate).toHaveBeenCalledWith('/', { replace: true });
    expect(mocks.trackTelemetryPreference).toHaveBeenCalledWith(false, 'onboarding');
    expect(mocks.setTelemetryEnabled).toHaveBeenCalledWith(false);
  });

  it('enters the application after persisting an opt-in', async () => {
    mocks.upsert.mockResolvedValue(undefined);
    const user = await reachTelemetryChoice();

    await user.click(await screen.findByRole('button', { name: 'Accept telemetry' }));

    await screen.findByText('Protected application');
    expect(mocks.navigate).toHaveBeenCalledWith('/', { replace: true });
    expect(mocks.trackTelemetryPreference).toHaveBeenCalledWith(true, 'onboarding');
    expect(mocks.setTelemetryEnabled).not.toHaveBeenCalled();
  });
});
