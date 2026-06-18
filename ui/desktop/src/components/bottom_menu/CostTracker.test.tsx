import { afterEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';

import { IntlTestWrapper } from '../../i18n/test-utils';
import { CostTracker } from './CostTracker';

vi.mock('../../utils/canonical', () => ({
  fetchCanonicalModelInfo: vi.fn(async () => ({
    provider: 'openai',
    model: 'deepseek-v4-flash',
    context_limit: 128000,
    max_output_tokens: 8192,
    reasoning: false,
    input_token_cost: 1,
    output_token_cost: 2,
    cache_read_token_cost: null,
    cache_write_token_cost: null,
    currency: '$',
  })),
}));

describe('CostTracker', () => {
  afterEach(() => {
    delete (window as unknown as Record<string, unknown>).appConfig;
  });

  it('hides pricing UI entirely when Security Goose marks Token Plan pricing as unsupported', () => {
    (window as unknown as Record<string, unknown>).appConfig = {
      get: (key: string) => (key === 'SECURITY_MODEL_PRICING_MODE' ? 'disabled-token-plan' : undefined),
      getAll: () => ({ SECURITY_MODEL_PRICING_MODE: 'disabled-token-plan' }),
    };

    const { container } = render(
      <CostTracker model="minimax-m3" provider="openai" inputTokens={10} outputTokens={20} />,
      { wrapper: IntlTestWrapper }
    );

    expect(container).toBeEmptyDOMElement();
  });

  it('still renders cost UI when pricing mode stays enabled', async () => {
    (window as unknown as Record<string, unknown>).appConfig = {
      get: () => undefined,
      getAll: () => ({}),
    };

    render(<CostTracker model="deepseek-v4-flash" provider="openai" inputTokens={10} outputTokens={20} />, {
      wrapper: IntlTestWrapper,
    });

    await waitFor(() => {
      expect(screen.getByText('0.00')).toBeInTheDocument();
    });
  });
});
