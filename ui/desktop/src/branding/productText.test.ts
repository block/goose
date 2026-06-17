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
    expect(getOnboardingTitle('收到')).toBe('Welcome to 收到');
    expect(getLauncherPlaceholder('收到')).toBe('Ask 收到 anything...');
    expect(getOnboardingDescription('收到')).toContain('收到');
  });

  it('brands task completion notifications with the product name', () => {
    expect(getTaskCompleteTitle('收到')).toBe('收到 finished the task.');
    expect(getTaskCompleteBody('收到')).toBe(
      'Click here to bring 收到 back into focus.'
    );
  });

  it('brands message catalogs without rewriting technical goose tokens', () => {
    expect(
      brandMessageCatalog(
        {
          title: 'Security Goose finished the task.',
          deepLink: 'Paste goose://recipe link here...',
          hints: 'Configure Project Hints (.goosehints) to improve communication with Goose.',
          backend: 'goosed must keep running.',
        },
        '收到'
      )
    ).toEqual({
      title: '收到 finished the task.',
      deepLink: 'Paste goose://recipe link here...',
      hints: 'Configure Project Hints (.goosehints) to improve communication with 收到.',
      backend: 'goosed must keep running.',
    });
  });
});
