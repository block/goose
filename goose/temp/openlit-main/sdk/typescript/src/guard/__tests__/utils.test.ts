import { customRuleDetection, applyThresholdScore } from '../utils';

describe('customRuleDetection', () => {
  it('matches a custom rule and returns correct result', () => {
    const rules = [
      { pattern: 'foo', classification: 'test', verdict: 'yes' as const, guard: 'custom', score: 0.9, explanation: 'Matched foo' }
    ];
    const result = customRuleDetection('foo bar', rules);
    expect(result.verdict).toBe('yes');
    expect(result.classification).toBe('test');
    expect(result.guard).toBe('custom');
    expect(result.score).toBe(0.9);
    expect(result.explanation).toBe('Matched foo');
  });

  it('returns none result if no rules match', () => {
    const rules = [
      { pattern: 'baz', classification: 'none' }
    ];
    const result = customRuleDetection('foo bar', rules);
    expect(result.verdict).toBe('none');
    expect(result.classification).toBe('none');
    expect(result.guard).toBe('none');
    expect(result.score).toBe(0);
    expect(result.explanation).toBe('none');
  });

  it('skips invalid regex patterns and continues', () => {
    const rules = [
      { pattern: '[invalid', classification: 'bad' },
      { pattern: 'bar', classification: 'good', verdict: 'yes' as const }
    ];
    const result = customRuleDetection('foo bar', rules);
    expect(result.classification).toBe('good');
    expect(result.verdict).toBe('yes');
  });
});

describe('applyThresholdScore', () => {
  const baseResult = {
    score: 0.7,
    verdict: 'yes' as const,
    guard: 'test_guard',
    classification: 'test_class',
    explanation: 'test explanation'
  };

  it('returns the original result if score >= threshold', () => {
    const result = applyThresholdScore(baseResult, 0.5);
    expect(result).toEqual(baseResult);
  });

  it('returns a none result if score < threshold', () => {
    const result = applyThresholdScore(baseResult, 0.8);
    expect(result).toEqual({
      score: 0,
      verdict: 'none',
      guard: 'none',
      classification: 'none',
      explanation: 'none'
    });
  });

  it('returns the original result if score == threshold', () => {
    const result = applyThresholdScore(baseResult, 0.7);
    expect(result).toEqual(baseResult);
  });
});
