import { describe, expect, it } from 'vitest';
import { mapLocaleToSupportedLanguage, normalizeUiLanguageSetting, resolveLanguage } from './language';

describe('language mapping', () => {
  it('maps simplified Chinese locales to zh-Hans', () => {
    expect(mapLocaleToSupportedLanguage('zh-CN')).toBe('zh-Hans');
    expect(mapLocaleToSupportedLanguage('zh-SG')).toBe('zh-Hans');
    expect(mapLocaleToSupportedLanguage('zh-Hans')).toBe('zh-Hans');
  });

  it('maps traditional Chinese locales to zh-Hant', () => {
    expect(mapLocaleToSupportedLanguage('zh-TW')).toBe('zh-Hant');
    expect(mapLocaleToSupportedLanguage('zh-HK')).toBe('zh-Hant');
    expect(mapLocaleToSupportedLanguage('zh-Hant')).toBe('zh-Hant');
  });

  it('falls back to en for non-Chinese locales', () => {
    expect(mapLocaleToSupportedLanguage('en-US')).toBe('en');
    expect(mapLocaleToSupportedLanguage('ja-JP')).toBe('en');
  });

  it('resolves user preference before system locale', () => {
    expect(resolveLanguage('zh-Hans', 'en-US')).toBe('zh-Hans');
    expect(resolveLanguage('zh-Hant', 'zh-CN')).toBe('zh-Hant');
    expect(resolveLanguage('en', 'zh-TW')).toBe('en');
  });

  it('uses system locale when setting is system', () => {
    expect(resolveLanguage('system', 'zh-CN')).toBe('zh-Hans');
    expect(resolveLanguage('system', 'zh-TW')).toBe('zh-Hant');
    expect(resolveLanguage('system', 'fr-FR')).toBe('en');
  });

  it('normalizes invalid ui language settings to system', () => {
    expect(normalizeUiLanguageSetting('zh-Hans')).toBe('zh-Hans');
    expect(normalizeUiLanguageSetting('system')).toBe('system');
    expect(normalizeUiLanguageSetting('foo')).toBe('system');
    expect(normalizeUiLanguageSetting(undefined)).toBe('system');
  });
});
