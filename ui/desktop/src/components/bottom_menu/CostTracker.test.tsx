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
});
