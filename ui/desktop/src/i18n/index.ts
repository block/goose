import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import en from './en.json';
import zhCN from './zh-CN.json';

const normalizeLanguage = (language?: string): 'en' | 'zh-CN' => {
  if (!language) return 'en';
  const lower = language.toLowerCase();
  if (lower.startsWith('zh')) return 'zh-CN';
  return 'en';
};

const initialLanguage = normalizeLanguage(typeof navigator === 'undefined' ? undefined : navigator.language);

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
