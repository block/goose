import { describe, expect, it } from 'vitest';
import type { HfModelInfo, HfModelVariant } from '../../../acp/local-inference';
import {
  compareQuality,
  catalogueOrder,
  fitRank,
  isEligibleVariant,
  recommendedVariant,
} from './modelRecommendations';

const quality = (rank: number): HfModelInfo['quality'] => ({
  evaluatedModelId: 'org/model',
  inherited: true,
  aaModelId: 'aa-model',
  sourceUrl: 'https://artificialanalysis.ai/models/model',
  evaluationName: 'Model (reasoning)',
  catalogueOrder: rank,
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
    compatibility: 'compatible',
    suitability: 'suitable',
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

describe('recommendation eligibility', () => {
  it('requires positive suitability, compatibility, fit and benchmark evidence', () => {
    const model = variant('likelyFit');
    expect(recommendedVariant([model], quality(1))).toBe(model);
    expect(recommendedVariant([model], undefined)).toBeUndefined();
    for (const compatibility of ['unknown', 'incompatible'] as const) {
      const rejected = { ...model, assessment: { ...model.assessment!, compatibility } };
      expect(isEligibleVariant(rejected)).toBe(false);
      expect(recommendedVariant([rejected], quality(1))).toBeUndefined();
    }
    for (const suitability of ['unknown', 'unsuitable'] as const) {
      expect(
        isEligibleVariant({ ...model, assessment: { ...model.assessment!, suitability } })
      ).toBe(false);
    }
  });
});

describe('catalogue ordering and unbounded forecasts', () => {
  it('preserves catalogue ordering and rejects a missing order', () => {
    expect(catalogueOrder(quality(3))).toBe(3);
    expect(catalogueOrder(quality(0))).toBeUndefined();
    expect(compareQuality(quality(0), quality(3))).toBeGreaterThan(0);
  });

  it('surfaces uncertain upper bounds without treating missing or oversized forecasts as fits', () => {
    const uncertain = variant('unknown');
    uncertain.assessment!.lowerBytes = 7e9;
    expect(isEligibleVariant(uncertain)).toBe(true);
    expect(fitRank([uncertain])).toBe(0);
    expect(recommendedVariant([uncertain], quality(1))).toBe(uncertain);
    const bounded = { ...variant('likelyFit'), qualityRank: 1 };
    expect(recommendedVariant([uncertain, bounded], quality(1))).toBe(bounded);
    uncertain.assessment!.lowerBytes = 7e9 + 1;
    expect(isEligibleVariant(uncertain)).toBe(false);
    uncertain.assessment!.lowerBytes = 7e9;
    uncertain.assessment!.upperBytes = 10e9;
    expect(isEligibleVariant(uncertain)).toBe(false);
    delete uncertain.assessment!.upperBytes;
    uncertain.assessment!.availableMemoryBytes = 0;
    expect(isEligibleVariant(uncertain)).toBe(false);
    expect(isEligibleVariant(variant('unknown'))).toBe(false);
  });
});
