import React, { createContext, useContext, useEffect, useMemo, useState } from 'react';
import i18n from '../i18n';
import {
  mapLocaleToSupportedLanguage,
  normalizeUiLanguageSetting,
  resolveLanguage,
  SupportedLanguage,
  UiLanguageSetting,
} from '../i18n/language';
import type { SettingKey, Settings } from '../utils/settings';

interface LanguageContextValue {
  uiLanguageSetting: UiLanguageSetting;
  resolvedLanguage: SupportedLanguage;
  setUiLanguageSetting: (language: UiLanguageSetting) => Promise<void>;
}

const LanguageContext = createContext<LanguageContextValue | null>(null);

interface LanguageProviderProps {
  children: React.ReactNode;
}

function getSystemLocale(): string | undefined {
  if (typeof navigator === 'undefined') {
    return undefined;
  }
  return navigator.language;
}

export function LanguageProvider({ children }: LanguageProviderProps) {
  const [uiLanguageSetting, setUiLanguageSettingState] = useState<UiLanguageSetting>('system');
  const [resolvedLanguage, setResolvedLanguage] = useState<SupportedLanguage>(
    mapLocaleToSupportedLanguage(getSystemLocale() || 'en')
  );

  useEffect(() => {
    async function loadLanguageFromSettings() {
      try {
        const saved = await window.electron.getSetting('uiLanguage');
        const setting = normalizeUiLanguageSetting(saved);
        const resolved = resolveLanguage(setting, getSystemLocale());

        setUiLanguageSettingState(setting);
        setResolvedLanguage(resolved);
        await i18n.changeLanguage(resolved);
      } catch (error) {
        console.warn('[LanguageContext] Failed to load language settings:', error);
        const fallback = resolveLanguage('system', getSystemLocale());
        setResolvedLanguage(fallback);
        await i18n.changeLanguage(fallback);
      }
    }

    loadLanguageFromSettings();
  }, []);

  useEffect(() => {
    const onSettingChanged = (key: SettingKey, value: Settings[SettingKey]) => {
      if (key !== 'uiLanguage') {
        return;
      }
      const setting = normalizeUiLanguageSetting(value);
      const resolved = resolveLanguage(setting, getSystemLocale());
      setUiLanguageSettingState(setting);
      setResolvedLanguage(resolved);
      void i18n.changeLanguage(resolved);
    };

    window.electron.onSettingChanged(onSettingChanged);
    return () => {
      window.electron.offSettingChanged(onSettingChanged);
    };
  }, []);

  useEffect(() => {
    if (uiLanguageSetting !== 'system') {
      return;
    }
    const onLanguageChanged = async () => {
      const resolved = resolveLanguage('system', getSystemLocale());
      setResolvedLanguage(resolved);
      await i18n.changeLanguage(resolved);
    };
    window.addEventListener('languagechange', onLanguageChanged);
    return () => {
      window.removeEventListener('languagechange', onLanguageChanged);
    };
  }, [uiLanguageSetting]);

  const setUiLanguageSetting = async (language: UiLanguageSetting) => {
    const resolved = resolveLanguage(language, getSystemLocale());
    setUiLanguageSettingState(language);
    setResolvedLanguage(resolved);
    await window.electron.setSetting('uiLanguage', language);
    await i18n.changeLanguage(resolved);
  };

  const value = useMemo(
    () => ({
      uiLanguageSetting,
      resolvedLanguage,
      setUiLanguageSetting,
    }),
    [uiLanguageSetting, resolvedLanguage]
  );

  return <LanguageContext.Provider value={value}>{children}</LanguageContext.Provider>;
}

export function useLanguage(): LanguageContextValue {
  const context = useContext(LanguageContext);
  if (!context) {
    throw new Error('useLanguage must be used within a LanguageProvider');
  }
  return context;
}
