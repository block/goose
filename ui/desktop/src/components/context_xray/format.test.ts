import { describe, expect, it } from 'vitest';
import { formatPercentOf, formatTokenCount } from './format';
import { formatTokenCount as canonicalFormatTokenCount } from '../../utils/usageFormatting';

describe('formatPercentOf', () => {
  it.each([
    [0, 0, '0%'],
    [500, 0, '0%'],
    [500, -1, '0%'],
    [0, 200_000, '0%'],
    [1, 200_000, '<1%'],
    [999, 200_000, '<1%'],
    [1000, 200_000, '1%'],
    [2000, 200_000, '1%'],
    [100_000, 200_000, '50%'],
    [200_000, 200_000, '100%'],
    [260_000, 200_000, '130%'],
  ])('formats %d of %d as %s', (part, total, expected) => {
    expect(formatPercentOf(part, total)).toBe(expected);
  });
});

describe('formatTokenCount re-export', () => {
  it('is the canonical formatter, not a second implementation', () => {
    expect(formatTokenCount).toBe(canonicalFormatTokenCount);
  });
});
