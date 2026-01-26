import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { CostTracker } from './CostTracker';
import { useModelAndProvider } from '../ModelAndProviderContext';
import { fetchModelPricing } from '../../utils/pricing';
import type { PricingData } from '../../api';

// Mock dependencies
vi.mock('../ModelAndProviderContext');
vi.mock('../../utils/pricing');
vi.mock('../icons', () => ({
  CoinIcon: () => <div data-testid="coin-icon">💰</div>,
}));
vi.mock('../ui/Tooltip', () => ({
  Tooltip: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  TooltipTrigger: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
  TooltipContent: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="tooltip-content">{children}</div>
  ),
}));

describe('CostTracker', () => {
  const mockUseModelAndProvider = vi.mocked(useModelAndProvider);
  const mockFetchModelPricing = vi.mocked(fetchModelPricing);

  beforeEach(() => {
    vi.clearAllMocks();
    // Mock localStorage to enable pricing
    vi.spyOn(window.localStorage, 'getItem').mockReturnValue(null);

    mockUseModelAndProvider.mockReturnValue({
      currentModel: 'anthropic.claude-3-5-sonnet-20241022-v2:0',
      currentProvider: 'aws_bedrock',
      setCurrentModel: vi.fn(),
      setCurrentProvider: vi.fn(),
      changeModel: vi.fn(async () => {}),
      getCurrentModelAndProvider: vi.fn(async () => ({
        model: 'anthropic.claude-3-5-sonnet-20241022-v2:0',
        provider: 'aws_bedrock',
      })),
      getFallbackModelAndProvider: vi.fn(async () => ({
        model: 'anthropic.claude-3-5-sonnet-20241022-v2:0',
        provider: 'aws_bedrock',
      })),
      getCurrentModelAndProviderForDisplay: vi.fn(async () => ({
        model: 'anthropic.claude-3-5-sonnet-20241022-v2:0',
        provider: 'aws_bedrock',
      })),
      getCurrentModelDisplayName: vi.fn(async () => 'anthropic.claude-3-5-sonnet-20241022-v2:0'),
      getCurrentProviderDisplayName: vi.fn(async () => 'aws_bedrock'),
      refreshCurrentModelAndProvider: vi.fn(async () => {}),
    });
  });

  describe('Cost Calculation - Per Million Tokens Fix', () => {
    it('calculates cost correctly with per-million token pricing', async () => {
      const mockPricingData: PricingData = {
        provider: 'aws_bedrock',
        model: 'anthropic.claude-3-5-sonnet-20241022-v2:0',
        input_token_cost: 3.0, // $3.00 per million tokens
        output_token_cost: 15.0, // $15.00 per million tokens
        currency: '$',
        context_length: 200000,
      };

      mockFetchModelPricing.mockResolvedValue(mockPricingData);

      render(<CostTracker inputTokens={5_982_210} outputTokens={83_497} />);

      await waitFor(() => {
        const costDisplay = screen.getByText(/[0-9.]+/);
        expect(costDisplay).toBeInTheDocument();
      });

      // Expected calculation:
      // Input: 5,982,210 / 1,000,000 * 3.0 = $17.9466
      // Output: 83,497 / 1,000,000 * 15.0 = $1.2525
      // Total: $19.1991

      // The display shows 4 decimal places
      const costDisplay = screen.getByText(/19\.199/);
      expect(costDisplay).toBeInTheDocument();
    });

    it('calculates small token amounts correctly', async () => {
      const mockPricingData: PricingData = {
        provider: 'aws_bedrock',
        model: 'anthropic.claude-3-5-sonnet-20241022-v2:0',
        input_token_cost: 3.0,
        output_token_cost: 15.0,
        currency: '$',
        context_length: 200000,
      };

      mockFetchModelPricing.mockResolvedValue(mockPricingData);

      // Small conversation: 1000 input, 500 output
      render(<CostTracker inputTokens={1_000} outputTokens={500} />);

      await waitFor(() => {
        const costDisplay = screen.getByText(/[0-9.]+/);
        expect(costDisplay).toBeInTheDocument();
      });

      // Expected calculation:
      // Input: 1,000 / 1,000,000 * 3.0 = $0.003
      // Output: 500 / 1,000,000 * 15.0 = $0.0075
      // Total: $0.0105

      const costDisplay = screen.getByText(/0\.010/);
      expect(costDisplay).toBeInTheDocument();
    });

    it('calculates zero tokens correctly', async () => {
      const mockPricingData: PricingData = {
        provider: 'aws_bedrock',
        model: 'anthropic.claude-3-5-sonnet-20241022-v2:0',
        input_token_cost: 3.0,
        output_token_cost: 15.0,
        currency: '$',
        context_length: 200000,
      };

      mockFetchModelPricing.mockResolvedValue(mockPricingData);

      render(<CostTracker inputTokens={0} outputTokens={0} />);

      await waitFor(() => {
        // Look for the cost display element (not in tooltip)
        const costElements = screen.getAllByText(/0\.0000/);
        // Should be displayed (first element is the visible cost, not in tooltip)
        expect(costElements.length).toBeGreaterThan(0);
        expect(costElements[0]).toBeInTheDocument();
      });
    });

    it('handles different Bedrock models with varying costs', async () => {
      const mockPricingData: PricingData = {
        provider: 'aws_bedrock',
        model: 'amazon.nova-micro',
        input_token_cost: 0.035, // $0.035 per million tokens
        output_token_cost: 0.14, // $0.14 per million tokens
        currency: '$',
        context_length: 128000,
      };

      mockFetchModelPricing.mockResolvedValue(mockPricingData);

      // 100,000 tokens in, 50,000 tokens out
      render(<CostTracker inputTokens={100_000} outputTokens={50_000} />);

      await waitFor(() => {
        const costDisplay = screen.getByText(/[0-9.]+/);
        expect(costDisplay).toBeInTheDocument();
      });

      // Expected calculation:
      // Input: 100,000 / 1,000,000 * 0.035 = $0.0035
      // Output: 50,000 / 1,000,000 * 0.14 = $0.007
      // Total: $0.0105

      const costDisplay = screen.getByText(/0\.010/);
      expect(costDisplay).toBeInTheDocument();
    });

    it('handles session costs with multiple models', async () => {
      const mockPricingData: PricingData = {
        provider: 'aws_bedrock',
        model: 'anthropic.claude-3-5-sonnet-20241022-v2:0',
        input_token_cost: 3.0,
        output_token_cost: 15.0,
        currency: '$',
        context_length: 200000,
      };

      mockFetchModelPricing.mockResolvedValue(mockPricingData);

      // Session costs from previous models (already calculated correctly by useCostTracking)
      const sessionCosts = {
        'aws_bedrock/amazon.nova-micro': {
          inputTokens: 50_000,
          outputTokens: 25_000,
          totalCost: 0.0053, // Pre-calculated: 50k * 0.035 / 1M + 25k * 0.14 / 1M
        },
      };

      render(
        <CostTracker inputTokens={100_000} outputTokens={50_000} sessionCosts={sessionCosts} />
      );

      await waitFor(() => {
        const costDisplay = screen.getByText(/[0-9.]+/);
        expect(costDisplay).toBeInTheDocument();
      });

      // Current model cost: 100k * 3.0 / 1M + 50k * 15.0 / 1M = 0.3 + 0.75 = 1.05
      // Previous model cost: 0.0053
      // Total: 1.0553

      // Query for the specific cost display (the span element, not the tooltip)
      const costElements = screen.getAllByText(/1\.055/);
      // First element should be the main display, second would be in the tooltip
      expect(costElements.length).toBeGreaterThan(0);
      expect(costElements[0].textContent).toMatch(/1\.0553/);
    });
  });

  describe('Free/Local Provider Handling', () => {
    it('shows zero cost for ollama provider', async () => {
      mockUseModelAndProvider.mockReturnValue({
        currentModel: 'llama2',
        currentProvider: 'ollama',
        setCurrentModel: vi.fn(),
        setCurrentProvider: vi.fn(),
        changeModel: vi.fn(async () => {}),
        getCurrentModelAndProvider: vi.fn(async () => ({
          model: 'llama2',
          provider: 'ollama',
        })),
        getFallbackModelAndProvider: vi.fn(async () => ({
          model: 'llama2',
          provider: 'ollama',
        })),
        getCurrentModelAndProviderForDisplay: vi.fn(async () => ({
          model: 'llama2',
          provider: 'ollama',
        })),
        getCurrentModelDisplayName: vi.fn(async () => 'llama2'),
        getCurrentProviderDisplayName: vi.fn(async () => 'ollama'),
        refreshCurrentModelAndProvider: vi.fn(async () => {}),
      });

      mockFetchModelPricing.mockResolvedValue(null);

      render(<CostTracker inputTokens={10_000} outputTokens={5_000} />);

      await waitFor(() => {
        const costDisplay = screen.getByText(/0\.0000/);
        expect(costDisplay).toBeInTheDocument();
      });
    });
  });

  describe('Missing Pricing Data', () => {
    it('shows zero when pricing data is unavailable', async () => {
      mockFetchModelPricing.mockResolvedValue(null);

      render(<CostTracker inputTokens={10_000} outputTokens={5_000} />);

      await waitFor(() => {
        const costDisplay = screen.getByText(/0\.0000/);
        expect(costDisplay).toBeInTheDocument();
      });
    });

    it('shows zero when pricing data has undefined costs', async () => {
      const mockPricingData: PricingData = {
        provider: 'aws_bedrock',
        model: 'unknown-model',
        input_token_cost: undefined as unknown as number,
        output_token_cost: undefined as unknown as number,
        currency: '$',
        context_length: 0,
      };

      mockFetchModelPricing.mockResolvedValue(mockPricingData);

      render(<CostTracker inputTokens={10_000} outputTokens={5_000} />);

      await waitFor(() => {
        const costDisplay = screen.getByText(/0\.0000/);
        expect(costDisplay).toBeInTheDocument();
      });
    });
  });
});
