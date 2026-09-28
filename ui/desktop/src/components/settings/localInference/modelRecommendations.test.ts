import { describe, expect, it } from 'vitest';
import type { HfModelInfo, HfModelVariant } from '../../../acp/local-inference';
import { compareQuality, fitRank } from './modelRecommendations';

const quality = (rank: number): HfModelInfo['quality'] => ({
  evaluatedModelId: 'org/model',
  inherited: true,
  evidence: [
    {
      benchmark: 'TIGER-Lab/MMLU-Pro',
      rank,
      participants: 100,
      score: 50,
      verified: false,
      sourceUrl: 'https://huggingface.co/org/model',
      notes: '',
    },
  ],
});

const variant = (
  fit: 'likelyFit' | 'likelyShortfall' | 'unknown',
  supported = true
): HfModelVariant => ({
  variantId: 'Q4',
  label: 'Q4',
  backendId: 'llamacpp',
  format: 'gguf',
  modelId: 'org/model:Q4',
  downloadId: 'org/model:Q4',
  sizeBytes: 1,
  description: '',
  qualityRank: 45,
  sharded: false,
  supported,
  assessment: {
    fit,
    recommended: false,
    inputTokens: 4096,
    outputTokens: 1024,
    availableMemoryBytes: 8e9,
    reserveBytes: 1e9,
    reasons: [],
  },
});

describe('local recommendation ranking', () => {
  it('uses external benchmark rank before popularity and keeps unscored models distinct', () => {
    expect(compareQuality(quality(1), quality(50))).toBeLessThan(0);
    expect(compareQuality(quality(50), undefined)).toBeLessThan(0);
    expect(compareQuality(undefined, undefined)).toBe(0);
  });
  it('never infers fit from file size and preserves uncertainty in mixed repositories', () => {
    expect(fitRank([variant('likelyShortfall')])).toBe(2);
    expect(fitRank([variant('likelyShortfall'), variant('unknown')])).toBe(1);
    expect(fitRank([variant('likelyFit')])).toBe(0);
    expect(fitRank([variant('likelyFit', false)])).not.toBe(0);
    expect(fitRank([])).toBe(1);
  });
});
