import type { UiLanguage } from '../utils/settings';
import { en, type TranslationSchema } from './locales/en';
import { zhCN } from './locales/zh-CN';

export const defaultUiLanguage: UiLanguage = 'en';

export const translations: Record<UiLanguage, TranslationSchema> = {
  en,
  'zh-CN': zhCN,
};

type TranslationLeaf = string;
type NestedTranslationKey<T> = {
  [K in keyof T & string]: T[K] extends TranslationLeaf
    ? K
    : T[K] extends Record<string, unknown>
      ? `${K}.${NestedTranslationKey<T[K]>}`
      : never;
}[keyof T & string];

export type TranslationKey = NestedTranslationKey<TranslationSchema>;

type TranslationParams = Record<string, string | number>;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function getTranslationValue(language: UiLanguage, key: TranslationKey): string | undefined {
  let current: unknown = translations[language];
  for (const part of key.split('.')) {
    if (!isRecord(current)) {
      return undefined;
    }
    current = current[part];
  }
  return typeof current === 'string' ? current : undefined;
}

function interpolate(template: string, params?: TranslationParams): string {
  if (!params) {
    return template;
  }

  return template.replace(/\{\{(\w+)\}\}/g, (_, key: string) => String(params[key] ?? ''));
}

export function translate(
  language: UiLanguage,
  key: TranslationKey,
  params?: TranslationParams
): string {
  const localized = getTranslationValue(language, key) ?? getTranslationValue(defaultUiLanguage, key);
  return interpolate(localized ?? key, params);
}

export function createTranslator(language: UiLanguage) {
  return (key: TranslationKey, params?: TranslationParams) => translate(language, key, params);
}

export function isUiLanguage(value: unknown): value is UiLanguage {
  return value === 'en' || value === 'zh-CN';
}

export function getIntlLocale(language: UiLanguage): string {
  return language;
}

export function formatDate(
  language: UiLanguage,
  value: Date | number | string,
  options?: Intl.DateTimeFormatOptions
): string {
  return new Intl.DateTimeFormat(getIntlLocale(language), options).format(new Date(value));
}

export function formatTime(
  language: UiLanguage,
  value: Date | number | string,
  options?: Intl.DateTimeFormatOptions
): string {
  return new Intl.DateTimeFormat(getIntlLocale(language), options).format(new Date(value));
}

export function formatNumber(
  language: UiLanguage,
  value: number,
  options?: Intl.NumberFormatOptions
): string {
  return new Intl.NumberFormat(getIntlLocale(language), options).format(value);
}
