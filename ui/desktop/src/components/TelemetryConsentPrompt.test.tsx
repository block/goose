import { render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { IntlTestWrapper } from '../i18n/test-utils';
import TelemetryConsentPrompt from './TelemetryConsentPrompt';

const mocks = vi.hoisted(() => ({
  read: vi.fn(),
  upsert: vi.fn(),
}));

vi.mock('./ConfigContext', () => ({
  useConfig: () => ({ read: mocks.read, upsert: mocks.upsert }),
}));

describe('TelemetryConsentPrompt', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('does not overlap onboarding while telemetry consent is pending', async () => {
    mocks.read.mockResolvedValueOnce(true);

    render(
      <IntlTestWrapper>
        <TelemetryConsentPrompt />
      </IntlTestWrapper>
    );

    await waitFor(() => expect(mocks.read).toHaveBeenCalledTimes(1));
    expect(screen.queryByRole('button', { name: 'No thanks' })).not.toBeInTheDocument();
    expect(mocks.read).toHaveBeenCalledTimes(1);
  });

  it('still prompts configured users with no telemetry preference', async () => {
    mocks.read
      .mockResolvedValueOnce(null)
      .mockResolvedValueOnce('test-provider')
      .mockResolvedValueOnce(null);

    render(
      <IntlTestWrapper>
        <TelemetryConsentPrompt />
      </IntlTestWrapper>
    );

    expect(await screen.findByRole('button', { name: 'No thanks' })).toBeInTheDocument();
  });
});
