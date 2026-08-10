import { render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { IntlTestWrapper } from '../i18n/test-utils';
import TelemetryConsentPrompt from './TelemetryConsentPrompt';

const mocks = vi.hoisted(() => ({
  read: vi.fn(),
  readAll: vi.fn(),
  upsert: vi.fn(),
}));

vi.mock('./ConfigContext', () => ({
  useConfig: () => ({ read: mocks.read, upsert: mocks.upsert }),
}));

vi.mock('../acp/config', () => ({
  acpReadAllConfig: mocks.readAll,
}));

describe('TelemetryConsentPrompt', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.readAll.mockResolvedValue({});
  });

  it('does not overlap onboarding while telemetry consent is pending', async () => {
    mocks.readAll.mockResolvedValueOnce({ GOOSE_ONBOARDING_TELEMETRY_PENDING: true });

    render(
      <IntlTestWrapper>
        <TelemetryConsentPrompt />
      </IntlTestWrapper>
    );

    await waitFor(() => expect(mocks.readAll).toHaveBeenCalledTimes(1));
    expect(screen.queryByRole('button', { name: 'No thanks' })).not.toBeInTheDocument();
    expect(mocks.read).not.toHaveBeenCalled();
  });

  it('does not overlap onboarding while a malformed pending marker is repaired', async () => {
    mocks.readAll.mockResolvedValueOnce({
      GOOSE_ONBOARDING_TELEMETRY_PENDING: 'not-a-boolean',
    });

    render(
      <IntlTestWrapper>
        <TelemetryConsentPrompt />
      </IntlTestWrapper>
    );

    await waitFor(() => expect(mocks.readAll).toHaveBeenCalledTimes(1));
    expect(screen.queryByRole('button', { name: 'No thanks' })).not.toBeInTheDocument();
    expect(mocks.read).not.toHaveBeenCalled();
  });

  it('does not overlap onboarding while an explicit null pending marker is repaired', async () => {
    mocks.readAll.mockResolvedValueOnce({
      GOOSE_ONBOARDING_TELEMETRY_PENDING: null,
    });

    render(
      <IntlTestWrapper>
        <TelemetryConsentPrompt />
      </IntlTestWrapper>
    );

    await waitFor(() => expect(mocks.readAll).toHaveBeenCalledTimes(1));
    expect(screen.queryByRole('button', { name: 'No thanks' })).not.toBeInTheDocument();
    expect(mocks.read).not.toHaveBeenCalled();
  });

  it('still prompts configured users with no telemetry preference', async () => {
    mocks.read.mockResolvedValueOnce('test-provider').mockResolvedValueOnce(null);

    render(
      <IntlTestWrapper>
        <TelemetryConsentPrompt />
      </IntlTestWrapper>
    );

    expect(await screen.findByRole('button', { name: 'No thanks' })).toBeInTheDocument();
  });
});
