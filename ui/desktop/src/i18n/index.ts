import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import { resources } from './resources';
import { DEFAULT_LANGUAGE } from './language';

i18n.use(initReactI18next).init({
  resources,
  lng: DEFAULT_LANGUAGE,
  fallbackLng: DEFAULT_LANGUAGE,
  interpolation: {
    escapeValue: false,
  },
  returnNull: false,
});

export default i18n;
