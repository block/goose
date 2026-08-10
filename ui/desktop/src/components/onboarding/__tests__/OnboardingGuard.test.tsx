import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { IntlTestWrapper } from '../../../i18n/test-utils';
import OnboardingGuard from '../OnboardingGuard';

const mocks = vi.hoisted(() => ({
  navigate: vi.fn(),
  read: vi.fn(),
  readAll: vi.fn(),
  remove: vi.fn(),
  upsert: vi.fn(),
  acpReadDefaults: vi.fn(),
  acpListProviderDetails: vi.fn(),
  acpSaveDefaults: vi.fn(),
  getFallbackModelAndProvider: vi.fn(),
  refreshCurrentModelAndProvider: vi.fn(),
  trackTelemetryPreference: vi.fn(),
  setTelemetryEnabled: vi.fn(),
}));

vi.mock('react-router', () => ({
  useNavigate: () => mocks.navigate,
}));

vi.mock('../../ConfigContext', () => ({
  useConfig: () => ({ read: mocks.read, remove: mocks.remove, upsert: mocks.upsert }),
}));

vi.mock('../../../acp/config', () => ({
  acpReadAllConfig: mocks.readAll,
}));

vi.mock('../../ModelAndProviderContext', () => ({
  useModelAndProvider: () => ({
    getFallbackModelAndProvider: mocks.getFallbackModelAndProvider,
    refreshCurrentModelAndProvider: mocks.refreshCurrentModelAndProvider,
  }),
}));

vi.mock('../../../acp/providers', () => ({
  acpReadDefaults: mocks.acpReadDefaults,
  acpListProviderDetails: mocks.acpListProviderDetails,
  acpSaveDefaults: mocks.acpSaveDefaults,
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
    mocks.read.mockResolvedValue(null);
    mocks.readAll.mockResolvedValue({});
    mocks.remove.mockResolvedValue(undefined);
    mocks.upsert.mockResolvedValue(undefined);
    mocks.acpReadDefaults.mockResolvedValue({ providerId: null, modelId: null });
    mocks.acpListProviderDetails.mockResolvedValue([
      {
        name: 'test-provider',
        metadata: { display_name: 'Test Provider', default_model: 'test-model' },
      },
    ]);
    mocks.acpSaveDefaults.mockResolvedValue(undefined);
    mocks.getFallbackModelAndProvider.mockResolvedValue({ provider: '', model: '' });
    mocks.refreshCurrentModelAndProvider.mockResolvedValue(undefined);
  });

  it('keeps onboarding retryable when an opt-out cannot be persisted', async () => {
    mocks.upsert
      .mockResolvedValueOnce(undefined)
      .mockRejectedValueOnce(new Error('config write failed'));
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
    const user = await reachTelemetryChoice();

    await user.click(await screen.findByRole('button', { name: 'Decline telemetry' }));

    await screen.findByText('Protected application');
    expect(mocks.navigate).toHaveBeenCalledWith('/', { replace: true });
    expect(mocks.trackTelemetryPreference).toHaveBeenCalledWith(false, 'onboarding');
    expect(mocks.setTelemetryEnabled).toHaveBeenCalledWith(false);
    expect(mocks.remove).toHaveBeenCalledWith('GOOSE_ONBOARDING_TELEMETRY_PENDING', false);
  });

  it('enters the application after persisting an opt-in', async () => {
    const user = await reachTelemetryChoice();

    await user.click(await screen.findByRole('button', { name: 'Accept telemetry' }));

    await screen.findByText('Protected application');
    expect(mocks.navigate).toHaveBeenCalledWith('/', { replace: true });
    expect(mocks.trackTelemetryPreference).toHaveBeenCalledWith(true, 'onboarding');
    expect(mocks.setTelemetryEnabled).not.toHaveBeenCalled();
    expect(mocks.remove).toHaveBeenCalledWith('GOOSE_ONBOARDING_TELEMETRY_PENDING', false);
  });

  it('persists pending consent before saving provider defaults', async () => {
    await reachTelemetryChoice();

    expect(mocks.upsert).toHaveBeenCalledWith(
      'GOOSE_ONBOARDING_TELEMETRY_PENDING',
      true,
      false
    );
    expect(mocks.upsert.mock.invocationCallOrder[0]).toBeLessThan(
      mocks.acpSaveDefaults.mock.invocationCallOrder[0]
    );
  });

  it('restores pending consent after provider defaults were persisted', async () => {
    mocks.readAll.mockResolvedValue({ GOOSE_ONBOARDING_TELEMETRY_PENDING: true });
    mocks.acpReadDefaults.mockResolvedValue({
      providerId: 'test-provider',
      modelId: 'test-model',
    });

    render(
      <IntlTestWrapper>
        <OnboardingGuard>
          <div>Protected application</div>
        </OnboardingGuard>
      </IntlTestWrapper>
    );

    expect(await screen.findByRole('button', { name: 'Decline telemetry' })).toBeInTheDocument();
    expect(screen.queryByText('Protected application')).not.toBeInTheDocument();
    expect(mocks.getFallbackModelAndProvider).not.toHaveBeenCalled();
  });

  it('repairs a malformed pending marker before entering the application', async () => {
    mocks.readAll.mockResolvedValue({
      GOOSE_ONBOARDING_TELEMETRY_PENDING: 'not-a-boolean',
    });
    mocks.acpReadDefaults.mockResolvedValue({
      providerId: 'test-provider',
      modelId: 'test-model',
    });

    const user = userEvent.setup();
    render(
      <IntlTestWrapper>
        <OnboardingGuard>
          <div>Protected application</div>
        </OnboardingGuard>
      </IntlTestWrapper>
    );

    await user.click(await screen.findByRole('button', { name: 'Decline telemetry' }));

    expect(await screen.findByText('Protected application')).toBeInTheDocument();
    expect(mocks.remove).toHaveBeenCalledWith('GOOSE_ONBOARDING_TELEMETRY_PENDING', false);
  });

  it('does not save fallback defaults while consent is pending without a provider', async () => {
    mocks.readAll.mockResolvedValue({ GOOSE_ONBOARDING_TELEMETRY_PENDING: true });
    mocks.getFallbackModelAndProvider.mockResolvedValue({
      provider: 'fallback-provider',
      model: 'fallback-model',
    });

    render(
      <IntlTestWrapper>
        <OnboardingGuard>
          <div>Protected application</div>
        </OnboardingGuard>
      </IntlTestWrapper>
    );

    expect(await screen.findByRole('button', { name: 'Configure' })).toBeInTheDocument();
    expect(mocks.getFallbackModelAndProvider).not.toHaveBeenCalled();
    expect(screen.queryByText('Protected application')).not.toBeInTheDocument();
  });

  it('keeps onboarding pending when clearing the marker fails', async () => {
    mocks.remove.mockRejectedValueOnce(new Error('config remove failed'));
    const user = await reachTelemetryChoice();

    await user.click(await screen.findByRole('button', { name: 'Decline telemetry' }));

    await waitFor(() => expect(mocks.remove).toHaveBeenCalledTimes(1));
    expect(mocks.navigate).not.toHaveBeenCalled();
    expect(mocks.trackTelemetryPreference).not.toHaveBeenCalled();
    expect(screen.getByRole('button', { name: 'Decline telemetry' })).toBeInTheDocument();
  });

  it('enters the application for an existing provider with no pending marker', async () => {
    mocks.acpReadDefaults.mockResolvedValue({
      providerId: 'test-provider',
      modelId: 'test-model',
    });

    render(
      <IntlTestWrapper>
        <OnboardingGuard>
          <div>Protected application</div>
        </OnboardingGuard>
      </IntlTestWrapper>
    );

    expect(await screen.findByText('Protected application')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Decline telemetry' })).not.toBeInTheDocument();
  });
});
