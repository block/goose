import { BaseEval } from '../base';
import { EvalsInput } from '../types';

describe('BaseEval', () => {
  class DummyEval extends BaseEval {
    getSystemPrompt() {
      return 'PROMPT';
    }
    protected async llmResponse(): Promise<string> {
      // Simulate a model response
      return JSON.stringify({
        verdict: 'yes',
        evaluation: 'Bias',
        score: 0.9,
        classification: 'age',
        explanation: 'reason',
      });
    }
  }

  it('measure returns parsed result and records metrics if enabled', async () => {
    const evaler = new DummyEval({ collectMetrics: true });
    const input: EvalsInput = { text: 'foo' };
    const result = await evaler.measure(input);
    expect(result.verdict).toBe('yes');
    expect(result.evaluation).toBe('Bias');
    expect(result.score).toBe(0.9);
  });

  it('throws on unsupported provider', async () => {
    class BadEval extends BaseEval {
      getSystemPrompt() { return 'PROMPT'; }
    }
    // @ts-expect-error: purposely passing an invalid provider for test
    const evaler = new BadEval({ provider: 'unknown' });
    await expect(evaler.measure({ text: 'foo' })).rejects.toThrow('Unsupported provider');
  });
});
