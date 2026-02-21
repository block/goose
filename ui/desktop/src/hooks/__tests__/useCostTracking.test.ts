/**
 * @vitest-environment jsdom
 */

import { renderHook, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { useCostTracking } from '../useCostTracking';

const mockUseModelAndProvider = vi.fn();
vi.mock('../../contexts/ModelAndProviderContext', () => ({
  useModelAndProvider: () => mockUseModelAndProvider(),
}));

const mockFetchModelPricing = vi.fn();
vi.mock('../../utils/pricing', () => ({
  fetchModelPricing: (...args: unknown[]) => mockFetchModelPricing(...args),
}));

describe('useCostTracking', () => {
  it('attributes token deltas to the current provider/model and caches pricing', async () => {
    mockUseModelAndProvider.mockReturnValue({ currentProvider: 'openai', currentModel: 'gpt-4o' });
    mockFetchModelPricing.mockResolvedValue({ input_token_cost: 0.01, output_token_cost: 0.02 });

    const { rerender, result } = renderHook(
      ({ inTok, outTok }) =>
        useCostTracking({
          sessionInputTokens: inTok,
          sessionOutputTokens: outTok,
          localInputTokens: 0,
          localOutputTokens: 0,
          session: { id: 's1' } as any,
        }),
      { initialProps: { inTok: 0, outTok: 0 } }
    );

    expect(result.current.sessionCosts).toEqual({});

    // 0 -> 10 input, 0 -> 5 output
    rerender({ inTok: 10, outTok: 5 });

    await waitFor(() => {
      expect(result.current.sessionCosts['openai/gpt-4o']).toBeDefined();
    });

    expect(result.current.sessionCosts['openai/gpt-4o']).toEqual({
      inputTokens: 10,
      outputTokens: 5,
      totalCost: 10 * 0.01 + 5 * 0.02,
    });

    // 10 -> 10 input (no delta), 5 -> 8 output (delta=3)
    rerender({ inTok: 10, outTok: 8 });

    await waitFor(() => {
      expect(result.current.sessionCosts['openai/gpt-4o']?.outputTokens).toBe(8);
    });

    expect(result.current.sessionCosts['openai/gpt-4o']).toEqual({
      inputTokens: 10,
      outputTokens: 8,
      totalCost: 10 * 0.01 + 8 * 0.02,
    });

    expect(mockFetchModelPricing).toHaveBeenCalledTimes(1);
  });

  it('resets when session changes', async () => {
    mockUseModelAndProvider.mockReturnValue({ currentProvider: 'openai', currentModel: 'gpt-4o' });
    mockFetchModelPricing.mockResolvedValue({ input_token_cost: 0.01, output_token_cost: 0.02 });

    const { rerender, result } = renderHook(
      ({ sessionId, inTok, outTok }) =>
        useCostTracking({
          sessionInputTokens: inTok,
          sessionOutputTokens: outTok,
          localInputTokens: 0,
          localOutputTokens: 0,
          session: { id: sessionId } as any,
        }),
      { initialProps: { sessionId: 's1', inTok: 0, outTok: 0 } }
    );

    rerender({ sessionId: 's1', inTok: 10, outTok: 0 });

    await waitFor(() => {
      expect(Object.keys(result.current.sessionCosts).length).toBe(1);
    });

    rerender({ sessionId: 's2', inTok: 0, outTok: 0 });

    await waitFor(() => {
      expect(result.current.sessionCosts).toEqual({});
    });
  });
});
