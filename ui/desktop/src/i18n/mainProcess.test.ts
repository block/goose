import { describe, expect, it } from 'vitest';
import { resolveMainLanguage, translateMain } from './mainProcess';

describe('main process i18n', () => {
  it('resolves system locale when setting is system', () => {
    expect(resolveMainLanguage('system', 'zh-CN')).toBe('zh-Hans');
    expect(resolveMainLanguage('system', 'zh-TW')).toBe('zh-Hant');
    expect(resolveMainLanguage('system', 'en-US')).toBe('en');
  });

  it('honors explicit language setting', () => {
    expect(resolveMainLanguage('zh-Hans', 'en-US')).toBe('zh-Hans');
    expect(resolveMainLanguage('zh-Hant', 'en-US')).toBe('zh-Hant');
    expect(resolveMainLanguage('en', 'zh-CN')).toBe('en');
  });

  it('falls back to english for unknown locale', () => {
    expect(
      translateMain('nativeMenu.settings', 'system', 'fr-FR')
    ).toBe('Settings');
  });

  it('supports interpolation', () => {
    expect(
      translateMain('nativeDialog.appStartupErrorMessage', 'en', 'en-US', { error: 'boom' })
    ).toContain('boom');
  });
});
