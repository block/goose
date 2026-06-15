import { describe, expect, it } from 'vitest';

import {
  brandMessageCatalog,
  getLauncherPlaceholder,
  getOnboardingDescription,
  getOnboardingTitle,
  getTaskCompleteBody,
  getTaskCompleteTitle,
} from './productText';

describe('productText', () => {
  it('brands onboarding and launcher copy with the product name', () => {
    expect(getOnboardingTitle('Security Goose')).toBe('Welcome to Security Goose');
    expect(getLauncherPlaceholder('Security Goose')).toBe('Ask Security Goose anything...');
    expect(getOnboardingDescription('Security Goose')).toContain('Security Goose');
  });

  it('brands task completion notifications with the product name', () => {
    expect(getTaskCompleteTitle('Security Goose')).toBe('Security Goose finished the task.');
    expect(getTaskCompleteBody('Security Goose')).toBe(
      'Click here to bring Security Goose back into focus.'
    );
  });

  it('brands message catalogs without rewriting technical goose tokens', () => {
    expect(
      brandMessageCatalog(
        {
          title: 'Goose finished the task.',
          deepLink: 'Paste goose://recipe link here...',
          hints: 'Configure Project Hints (.goosehints)',
        },
        'Security Goose'
      )
    ).toEqual({
      title: 'Security Goose finished the task.',
      deepLink: 'Paste goose://recipe link here...',
      hints: 'Configure Project Hints (.goosehints)',
    });
  });
});
