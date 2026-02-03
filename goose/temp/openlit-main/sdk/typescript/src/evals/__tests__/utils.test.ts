import { formatPrompt, parseLlmResponse, formatCustomCategories } from '../utils';

describe('evals/utils', () => {
  describe('formatPrompt', () => {
    it('formats prompt with all fields', () => {
      const result = formatPrompt('SYSTEM', { prompt: 'p', contexts: ['c1', 'c2'], text: 't' });
      expect(result).toContain('SYSTEM');
      expect(result).toContain('Prompt: p');
      expect(result).toContain('Contexts: c1 | c2');
      expect(result).toContain('Text: t');
    });
    it('handles missing fields', () => {
      const result = formatPrompt('SYSTEM', { text: 't' });
      expect(result).toContain('Text: t');
    });
  });

  describe('parseLlmResponse', () => {
    it('parses valid JSON', () => {
      const obj = { verdict: 'yes', evaluation: 'Bias', score: 1, classification: 'age', explanation: 'reason' };
      expect(parseLlmResponse(JSON.stringify(obj))).toEqual(obj);
    });
    it('returns fallback on invalid JSON', () => {
      const originalError = console.error;
      console.error = jest.fn(); // suppress error output
      const result = parseLlmResponse('not json');
      expect(result.verdict).toBe('no');
      expect(result.classification).toBe('none');
      console.error = originalError; // restore
    });
  });

  describe('formatCustomCategories', () => {
    it('returns empty string if no categories', () => {
      expect(formatCustomCategories()).toBe('');
      expect(formatCustomCategories({})).toBe('');
    });
    it('formats categories with label', () => {
      const cats = { foo: 'desc1', bar: 'desc2' };
      const out = formatCustomCategories(cats, 'Bias');
      expect(out).toContain('Additional Bias Categories:');
      expect(out).toContain('- foo: desc1');
      expect(out).toContain('- bar: desc2');
    });
  });
});
