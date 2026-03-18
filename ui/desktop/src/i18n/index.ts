import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import type { LanguagePreference } from '../utils/settings';
import en from './en.json';
import zhCN from './zh-CN.json';

const normalizeLanguage = (language?: string): 'en' | 'zh-CN' => {
  if (!language) return 'en';
  const lower = language.toLowerCase();
  if (lower.startsWith('zh')) return 'zh-CN';
  return 'en';
};

const resolveLanguage = (preference?: LanguagePreference): 'en' | 'zh-CN' => {
  if (!preference || preference === 'system') {
    return normalizeLanguage(typeof navigator === 'undefined' ? undefined : navigator.language);
  }
  if (preference === 'zh-CN') return 'zh-CN';
  return 'en';
};

export const applyLanguagePreference = (preference?: LanguagePreference) => {
  const nextLanguage = resolveLanguage(preference);
  if (i18n.language !== nextLanguage) {
    void i18n.changeLanguage(nextLanguage);
  }
};

const initialLanguage = resolveLanguage('system');

i18n.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    'zh-CN': { translation: zhCN },
  },
  lng: initialLanguage,
  fallbackLng: 'en',
  interpolation: {
    escapeValue: false,
  },
});

export default i18n;
