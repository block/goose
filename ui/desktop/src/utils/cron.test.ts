import { describe, expect, it } from 'vitest';
import { formatCronDescription } from './cron';

describe('formatCronDescription', () => {
  it('formats cron descriptions in English', () => {
    expect(formatCronDescription('0 0 14 * * *', 'en')).toBe('At 02:00 PM');
  });

  it('formats cron descriptions in Simplified Chinese', () => {
    expect(formatCronDescription('0 0 14 * * *', 'zh-CN')).toContain('下午');
  });

  it('can omit seconds before formatting', () => {
    expect(
      formatCronDescription('0 0 14 * * *', 'en', { dropSeconds: true, lowercase: true })
    ).toBe('at 02:00 pm');
  });
});
