import en from './en.json';

type DictionaryValue = string | { [key: string]: DictionaryValue };
type Dictionary = { [key: string]: DictionaryValue };

const DEFAULT_LOCALE = 'en';
const dictionaries: Record<string, Dictionary> = {
  en: en as Dictionary,
};

function normalizeLocale(raw: string): string {
  return raw.trim().toLowerCase().replace(/_/g, '-');
}

function normalizeLocaleTag(raw: string): string {
  return raw
    .trim()
    .replace(/_/g, '-')
    .replace(/\..*$/, '')
    .replace(/@.*$/, '');
}

function toValidLocaleTag(raw: string): string | undefined {
  const normalized = normalizeLocaleTag(raw);
  if (!normalized) {
    return undefined;
  }

  try {
    const [canonical] = Intl.getCanonicalLocales(normalized);
    return canonical;
  } catch {
    return undefined;
  }
}

function resolveLocale(raw: string | undefined | null, availableLocales: string[]): string {
  if (!raw) {
    return DEFAULT_LOCALE;
  }

  const normalized = normalizeLocale(raw);
  const normalizedAvailable = availableLocales.map((locale) => normalizeLocale(locale));

  const exactIndex = normalizedAvailable.indexOf(normalized);
  if (exactIndex !== -1) {
    return availableLocales[exactIndex];
  }

  const language = normalized.split('-')[0];
  const baseIndex = normalizedAvailable.findIndex(
    (locale) => locale === language || locale.startsWith(`${language}-`)
  );
  if (baseIndex !== -1) {
    return availableLocales[baseIndex];
  }

  return DEFAULT_LOCALE;
}

function getConfiguredLocaleRaw(): string | undefined {
  if (typeof window !== 'undefined') {
    const configured = window.appConfig?.get('GOOSE_LOCALE');
    if (typeof configured === 'string') {
      return configured;
    }
  }

  if (typeof process !== 'undefined') {
    const configuredEnv = process.env?.GOOSE_LOCALE;
    if (typeof configuredEnv === 'string') {
      return configuredEnv;
    }
  }

  if (typeof navigator !== 'undefined' && typeof navigator.language === 'string') {
    return navigator.language;
  }

  return undefined;
}

function getByPath(dictionary: Dictionary, path: string): string | undefined {
  const segments = path.split('.');
  let current: DictionaryValue | undefined = dictionary;

  for (const segment of segments) {
    if (!current || typeof current === 'string') {
      return undefined;
    }
    current = current[segment];
  }

  return typeof current === 'string' ? current : undefined;
}

function interpolate(template: string, vars?: Record<string, string | number>): string {
  if (!vars) {
    return template;
  }

  return template.replace(/\{(\w+)\}/g, (_match, key: string) => {
    const value = vars[key];
    return value === undefined ? `{${key}}` : String(value);
  });
}

export function currentLocale(): string {
  return resolveLocale(getConfiguredLocaleRaw(), Object.keys(dictionaries));
}

export function currentLocaleTag(): string {
  const raw = getConfiguredLocaleRaw();
  if (raw) {
    const localeTag = toValidLocaleTag(raw);
    if (localeTag) {
      return localeTag;
    }
  }

  return 'en-US';
}

export function t(
  key: string,
  fallback?: string,
  vars?: Record<string, string | number>,
  localeOverride?: string
): string {
  const availableLocales = Object.keys(dictionaries);
  const locale = resolveLocale(localeOverride ?? getConfiguredLocaleRaw(), availableLocales);
  const localized = getByPath(dictionaries[locale], key);

  if (localized) {
    return interpolate(localized, vars);
  }

  const english = getByPath(dictionaries[DEFAULT_LOCALE], key);
  if (english) {
    return interpolate(english, vars);
  }

  return interpolate(fallback ?? key, vars);
}
