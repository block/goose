import { describe, it, expect, vi, afterEach } from 'vitest';
import { getLocale } from './index';

// Helper to mock window.appConfig for tests
function mockAppConfig(values: Record<string, unknown>) {
  (window as unknown as Record<string, unknown>).appConfig = {
    get: (key: string) => values[key],
    getAll: () => values,
  };
}

describe('getLocale', () => {
  afterEach(() => {
    // Clean up appConfig mock
    if (typeof window !== 'undefined') {
      delete (window as unknown as Record<string, unknown>).appConfig;
    }
    vi.restoreAllMocks();
  });

  it('returns "en" as the default fallback', () => {
    // navigator.languages contains only unsupported tags
    vi.stubGlobal('navigator', { languages: ['xx-XX'] });
    expect(getLocale()).toEqual({ locale: 'en', messageLocale: 'en' });
  });

  it('preserves regional tag for formatting when base language is supported', () => {
    vi.stubGlobal('navigator', { languages: ['en-US'] });
    expect(getLocale()).toEqual({ locale: 'en-US', messageLocale: 'en' });
  });

  it('returns exact match when navigator.languages contains a supported locale', () => {
    vi.stubGlobal('navigator', { languages: ['en'] });
    expect(getLocale()).toEqual({ locale: 'en', messageLocale: 'en' });
  });

  it('respects GOOSE_LOCALE over navigator.languages', () => {
    mockAppConfig({ GOOSE_LOCALE: 'en' });
    vi.stubGlobal('navigator', { languages: ['xx-XX'] });
    expect(getLocale()).toEqual({ locale: 'en', messageLocale: 'en' });
  });

  it('preserves regional tag from GOOSE_LOCALE', () => {
    mockAppConfig({ GOOSE_LOCALE: 'en-GB' });
    vi.stubGlobal('navigator', { languages: ['xx-XX'] });
    expect(getLocale()).toEqual({ locale: 'en-GB', messageLocale: 'en' });
  });

  it('falls back to base language tag for message catalog', () => {
    // "en-GB" should use "en" catalog but keep "en-GB" for formatting
    vi.stubGlobal('navigator', { languages: ['en-GB'] });
    expect(getLocale()).toEqual({ locale: 'en-GB', messageLocale: 'en' });
  });

  it('returns Russian when navigator.languages contains ru', () => {
    vi.stubGlobal('navigator', { languages: ['ru'] });
    expect(getLocale()).toEqual({ locale: 'ru', messageLocale: 'ru' });
  });

  it('preserves Russian regional tag for formatting', () => {
    vi.stubGlobal('navigator', { languages: ['ru-RU'] });
    expect(getLocale()).toEqual({ locale: 'ru-RU', messageLocale: 'ru' });
  });

  it('supports Turkish from navigator.languages', () => {
    vi.stubGlobal('navigator', { languages: ['tr-TR'] });
    expect(getLocale()).toEqual({ locale: 'tr-TR', messageLocale: 'tr' });
  });

  it('supports explicit Turkish locale', () => {
    mockAppConfig({ GOOSE_LOCALE: 'tr' });
    vi.stubGlobal('navigator', { languages: ['xx-XX'] });
    expect(getLocale()).toEqual({ locale: 'tr', messageLocale: 'tr' });
  });

  it('supports Korean from navigator.languages', () => {
    vi.stubGlobal('navigator', { languages: ['ko-KR'] });
    expect(getLocale()).toEqual({ locale: 'ko-KR', messageLocale: 'ko' });
  });

  it('supports explicit Korean locale', () => {
    mockAppConfig({ GOOSE_LOCALE: 'ko' });
    vi.stubGlobal('navigator', { languages: ['xx-XX'] });
    expect(getLocale()).toEqual({ locale: 'ko', messageLocale: 'ko' });
  });

  it('supports POSIX-style Korean locale from GOOSE_LOCALE', () => {
    mockAppConfig({ GOOSE_LOCALE: 'ko_KR' });
    vi.stubGlobal('navigator', { languages: ['xx-XX'] });
    expect(getLocale()).toEqual({ locale: 'ko-KR', messageLocale: 'ko' });
  });

  it('supports Japanese from navigator.languages', () => {
    vi.stubGlobal('navigator', { languages: ['ja-JP'] });
    expect(getLocale()).toEqual({ locale: 'ja-JP', messageLocale: 'ja' });
  });

  it('supports POSIX-style Japanese locale from GOOSE_LOCALE', () => {
    mockAppConfig({ GOOSE_LOCALE: 'ja_JP' });
    vi.stubGlobal('navigator', { languages: ['xx-XX'] });
    expect(getLocale()).toEqual({ locale: 'ja-JP', messageLocale: 'ja' });
  });

  it('supports Hindi from navigator.languages', () => {
    vi.stubGlobal('navigator', { languages: ['hi-IN'] });
    expect(getLocale()).toEqual({ locale: 'hi-IN', messageLocale: 'hi' });
  });

  it('supports explicit Hindi locale', () => {
    mockAppConfig({ GOOSE_LOCALE: 'hi' });
    vi.stubGlobal('navigator', { languages: ['xx-XX'] });
    expect(getLocale()).toEqual({ locale: 'hi', messageLocale: 'hi' });
  });

  it('supports Spanish from navigator.languages', () => {
    vi.stubGlobal('navigator', { languages: ['es-ES'] });
    expect(getLocale()).toEqual({ locale: 'es-ES', messageLocale: 'es' });
  });

  it('supports explicit Spanish locale', () => {
    mockAppConfig({ GOOSE_LOCALE: 'es' });
    vi.stubGlobal('navigator', { languages: ['xx-XX'] });
    expect(getLocale()).toEqual({ locale: 'es', messageLocale: 'es' });
  });

  it('falls back to base language when locale tag is invalid BCP 47', () => {
    // "en-" is not a valid BCP 47 tag and would cause RangeError in Intl APIs
    mockAppConfig({ GOOSE_LOCALE: 'en-' });
    vi.stubGlobal('navigator', { languages: ['xx-XX'] });
    expect(getLocale()).toEqual({ locale: 'en', messageLocale: 'en' });
  });
});

describe('loadMessages', () => {
  it('returns empty object for English locale', async () => {
    const { loadMessages } = await import('./index');
    const messages = await loadMessages('en');
    expect(messages).toEqual({});
  });

  it('returns empty object for unsupported locale (with warning)', async () => {
    const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
    const { loadMessages } = await import('./index');
    const messages = await loadMessages('xx');
    expect(messages).toEqual({});
    expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('No message catalog found'));
    warnSpy.mockRestore();
  });
});
