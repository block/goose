import React, { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react';
import type { UiLanguage } from '../utils/settings';
import {
  createTranslator,
  defaultUiLanguage,
  formatDate as formatDateForLanguage,
  formatNumber as formatNumberForLanguage,
  formatTime as formatTimeForLanguage,
  getIntlLocale,
  isUiLanguage,
  type TranslationKey,
} from '../i18n';

interface LocalizationContextValue {
  language: UiLanguage;
  intlLocale: string;
  setLanguage: (language: UiLanguage) => Promise<void>;
  t: (key: TranslationKey, params?: Record<string, string | number>) => string;
  formatDate: (
    value: Date | number | string,
    options?: Intl.DateTimeFormatOptions
  ) => string;
  formatTime: (
    value: Date | number | string,
    options?: Intl.DateTimeFormatOptions
  ) => string;
  formatNumber: (value: number, options?: Intl.NumberFormatOptions) => string;
}

const LocalizationContext = createContext<LocalizationContextValue | null>(null);

export function LocalizationProvider({ children }: { children: React.ReactNode }) {
  const [language, setLanguageState] = useState<UiLanguage>(defaultUiLanguage);

  useEffect(() => {
    async function loadLanguageFromSettings() {
      try {
        const savedLanguage = await window.electron.getSetting('uiLanguage');
        if (isUiLanguage(savedLanguage)) {
          setLanguageState(savedLanguage);
        }
      } catch (error) {
        console.warn('[LocalizationContext] Failed to load ui language:', error);
      }
    }

    loadLanguageFromSettings();
  }, []);

  useEffect(() => {
    document.documentElement.lang = language;
  }, [language]);

  const setLanguage = useCallback(
    async (nextLanguage: UiLanguage) => {
      if (nextLanguage === language) {
        return;
      }

      setLanguageState(nextLanguage);

      try {
        await window.electron.setSetting('uiLanguage', nextLanguage);
        window.electron.broadcastLanguageChange({ language: nextLanguage });
      } catch (error) {
        console.warn('[LocalizationContext] Failed to save ui language:', error);
      }
    },
    [language]
  );

  useEffect(() => {
    const handleLanguageChanged = (_event: unknown, ...args: unknown[]) => {
      const nextLanguage = (args[0] as { language?: unknown } | undefined)?.language;
      if (isUiLanguage(nextLanguage)) {
        setLanguageState(nextLanguage);
      }
    };

    window.electron.on('language-changed', handleLanguageChanged);
    return () => {
      window.electron.off('language-changed', handleLanguageChanged);
    };
  }, []);

  const t = useMemo(() => createTranslator(language), [language]);

  const value = useMemo<LocalizationContextValue>(
    () => ({
      language,
      intlLocale: getIntlLocale(language),
      setLanguage,
      t,
      formatDate: (input, options) => formatDateForLanguage(language, input, options),
      formatTime: (input, options) => formatTimeForLanguage(language, input, options),
      formatNumber: (input, options) => formatNumberForLanguage(language, input, options),
    }),
    [language, setLanguage, t]
  );

  return <LocalizationContext.Provider value={value}>{children}</LocalizationContext.Provider>;
}

export function useLocalization(): LocalizationContextValue {
  const context = useContext(LocalizationContext);
  if (!context) {
    throw new Error('useLocalization must be used within LocalizationProvider');
  }
  return context;
}
