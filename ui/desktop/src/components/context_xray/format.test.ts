import { describe, expect, it } from 'vitest';
import { formatPercentOf } from './format';

describe('formatPercentOf', () => {
  it.each([
    [500, 0, '0%'],
    [0, 200_000, '0%'],
    [1, 200_000, '<1%'],
    [1000, 200_000, '1%'],
    [100_000, 200_000, '50%'],
    [200_000, 200_000, '100%'],
    [260_000, 200_000, '130%'],
  ])('formats %d of %d as %s', (part, total, expected) => {
    expect(formatPercentOf(part, total)).toBe(expected);
  });
});
