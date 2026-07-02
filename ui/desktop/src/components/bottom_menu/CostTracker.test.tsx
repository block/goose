import { render, screen, type RenderOptions } from '@testing-library/react';
import type React from 'react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { IntlTestWrapper } from '../../i18n/test-utils';
import { fetchCanonicalModelInfo } from '../../utils/canonical';
import { CostTracker } from './CostTracker';

vi.mock('../../utils/canonical', () => ({
  fetchCanonicalModelInfo: vi.fn(),
}));

vi.mock('../ui/Tooltip', () => ({
  Tooltip: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  TooltipTrigger: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  TooltipContent: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));

const renderWithIntl = (ui: React.ReactElement, options?: RenderOptions) =>
  render(ui, { wrapper: IntlTestWrapper, ...options });

const fetchCanonicalModelInfoMock = vi.mocked(fetchCanonicalModelInfo);

describe('CostTracker', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fetchCanonicalModelInfoMock.mockResolvedValue({
      provider: 'test-provider',
      model: 'gpt-5.5-2026-04-23',
      contextLimit: 128000,
      maxOutputTokens: null,
      reasoning: true,
      inputTokenCost: 500,
      outputTokenCost: 3000,
      cacheReadTokenCost: null,
      cacheWriteTokenCost: null,
      currency: 'RUB',
    });
  });

  it('shows input and output cost breakdown for provider usage snapshots', async () => {
    renderWithIntl(
      <CostTracker
        model="gpt-5.5"
        provider="test-provider"
        providerUsage={[
          {
            providerId: 'test-provider',
            modelId: 'gpt-5.5-2026-04-23',
            lastUsedAt: '2026-07-02T18:10:38Z',
            inputTokens: 10000,
            outputTokens: 1000,
          },
        ]}
      />
    );

    expect(await screen.findByText('₽8.00')).toBeInTheDocument();
    expect(
      screen.getByText(/Input: 10,000 tokens \(₽5.00\) \| Output: 1,000 tokens \(₽3.00\)/)
    ).toBeInTheDocument();
    expect(screen.getByText(/Total session cost: ₽8.00/)).toBeInTheDocument();
    expect(fetchCanonicalModelInfoMock).toHaveBeenCalledWith(
      'test-provider',
      'gpt-5.5-2026-04-23'
    );
  });

  it('uses the same cost calculation for current session token usage', async () => {
    renderWithIntl(
      <CostTracker
        model="gpt-5.5"
        provider="test-provider"
        inputTokens={10000}
        outputTokens={1000}
      />
    );

    expect(await screen.findByText('₽8.00')).toBeInTheDocument();
    expect(
      screen.getByText(/Input: 10,000 tokens \(₽5.00\) \| Output: 1,000 tokens \(₽3.00\)/)
    ).toBeInTheDocument();
    expect(fetchCanonicalModelInfoMock).toHaveBeenCalledWith('test-provider', 'gpt-5.5');
  });

  it('prefers accumulated cost when provider usage snapshots are unavailable', async () => {
    renderWithIntl(
      <CostTracker
        model="gpt-5.5"
        provider="test-provider"
        inputTokens={10000}
        outputTokens={1000}
        accumulatedCost={0.03}
      />
    );

    expect(await screen.findByText('0.03')).toBeInTheDocument();
    expect(screen.getByText(/Total session cost: 0.03/)).toBeInTheDocument();
    expect(fetchCanonicalModelInfoMock).not.toHaveBeenCalled();
  });

  it('prices cached input tokens with cache rates when available', async () => {
    fetchCanonicalModelInfoMock.mockResolvedValue({
      provider: 'test-provider',
      model: 'gpt-5.5-2026-04-23',
      contextLimit: 128000,
      maxOutputTokens: null,
      reasoning: true,
      inputTokenCost: 500,
      outputTokenCost: 3000,
      cacheReadTokenCost: 100,
      cacheWriteTokenCost: 700,
      currency: 'RUB',
    });

    renderWithIntl(
      <CostTracker
        model="gpt-5.5"
        provider="test-provider"
        providerUsage={[
          {
            providerId: 'test-provider',
            modelId: 'gpt-5.5-2026-04-23',
            lastUsedAt: '2026-07-02T18:10:38Z',
            inputTokens: 10000,
            outputTokens: 1000,
            cacheReadInputTokens: 4000,
            cacheWriteInputTokens: 1000,
          },
        ]}
      />
    );

    expect(await screen.findByText('₽6.60')).toBeInTheDocument();
    expect(
      screen.getByText(/Input: 10,000 tokens \(₽3.60\) \| Output: 1,000 tokens \(₽3.00\)/)
    ).toBeInTheDocument();
  });

  it('adds usage from different providers with the same currency using each provider price', async () => {
    fetchCanonicalModelInfoMock.mockImplementation(async (provider) => ({
      provider,
      model: 'shared-model',
      contextLimit: 128000,
      maxOutputTokens: null,
      reasoning: true,
      inputTokenCost: provider === 'cheap-provider' ? 100 : 500,
      outputTokenCost: provider === 'cheap-provider' ? 200 : 1000,
      cacheReadTokenCost: null,
      cacheWriteTokenCost: null,
      currency: 'USD',
    }));

    renderWithIntl(
      <CostTracker
        model="shared-model"
        provider="expensive-provider"
        providerUsage={[
          {
            providerId: 'cheap-provider',
            modelId: 'shared-model',
            lastUsedAt: '2026-07-02T18:10:38Z',
            inputTokens: 10000,
            outputTokens: 1000,
          },
          {
            providerId: 'expensive-provider',
            modelId: 'shared-model',
            lastUsedAt: '2026-07-02T18:12:38Z',
            inputTokens: 10000,
            outputTokens: 1000,
          },
        ]}
      />
    );

    expect(await screen.findByText('$7.20')).toBeInTheDocument();
    expect(
      screen.getByText(/Input: 20,000 tokens \(\$6.00\) \| Output: 2,000 tokens \(\$1.20\)/)
    ).toBeInTheDocument();
    expect(fetchCanonicalModelInfoMock).toHaveBeenCalledWith('cheap-provider', 'shared-model');
    expect(fetchCanonicalModelInfoMock).toHaveBeenCalledWith('expensive-provider', 'shared-model');
  });

  it('keeps separate totals when provider usage contains different currencies', async () => {
    fetchCanonicalModelInfoMock.mockImplementation(async (provider) => ({
      provider,
      model: 'shared-model',
      contextLimit: 128000,
      maxOutputTokens: null,
      reasoning: true,
      inputTokenCost: provider === 'rub-provider' ? 500 : 1,
      outputTokenCost: provider === 'rub-provider' ? 3000 : 2,
      cacheReadTokenCost: null,
      cacheWriteTokenCost: null,
      currency: provider === 'rub-provider' ? 'RUB' : 'USD',
    }));

    renderWithIntl(
      <CostTracker
        model="shared-model"
        provider="usd-provider"
        providerUsage={[
          {
            providerId: 'rub-provider',
            modelId: 'shared-model',
            lastUsedAt: '2026-07-02T18:10:38Z',
            inputTokens: 10000,
            outputTokens: 1000,
          },
          {
            providerId: 'usd-provider',
            modelId: 'shared-model',
            lastUsedAt: '2026-07-02T18:12:38Z',
            inputTokens: 1000000,
            outputTokens: 1000000,
          },
        ]}
      />
    );

    expect(await screen.findByText('₽8.00 + $3.00')).toBeInTheDocument();
    expect(
      screen.getByText(
        /Input: 1,010,000 tokens \(₽5.00 \+ \$1.00\) \| Output: 1,001,000 tokens \(₽3.00 \+ \$2.00\)/
      )
    ).toBeInTheDocument();
  });
});
