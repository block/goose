export type SupportedLanguage = 'en' | 'zh-Hans' | 'zh-Hant';
export type UiLanguageSetting = 'system' | SupportedLanguage;

export const DEFAULT_LANGUAGE: SupportedLanguage = 'en';
const VALID_UI_LANGUAGE_SETTINGS: ReadonlySet<UiLanguageSetting> = new Set([
  'system',
  'en',
  'zh-Hans',
  'zh-Hant',
]);

function normalizeLocale(locale: string): string {
  return locale.trim().toLowerCase().replace(/_/g, '-');
}

export function normalizeUiLanguageSetting(value: unknown): UiLanguageSetting {
  if (typeof value === 'string' && VALID_UI_LANGUAGE_SETTINGS.has(value as UiLanguageSetting)) {
    return value as UiLanguageSetting;
  }
  return 'system';
}

export function mapLocaleToSupportedLanguage(locale: string): SupportedLanguage {
  const normalized = normalizeLocale(locale);

  if (normalized.startsWith('zh-cn') || normalized.startsWith('zh-sg')) {
    return 'zh-Hans';
  }

  if (normalized.startsWith('zh-tw') || normalized.startsWith('zh-hk')) {
    return 'zh-Hant';
  }

  if (normalized.startsWith('zh-hans')) {
    return 'zh-Hans';
  }

  if (normalized.startsWith('zh-hant')) {
    return 'zh-Hant';
  }

  return DEFAULT_LANGUAGE;
}

export function resolveLanguage(
  setting: UiLanguageSetting,
  systemLocale: string | undefined
): SupportedLanguage {
  if (setting !== 'system') {
    return setting;
  }
  if (!systemLocale) {
    return DEFAULT_LANGUAGE;
  }
  return mapLocaleToSupportedLanguage(systemLocale);
}
