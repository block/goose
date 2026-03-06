import cronstrue from 'cronstrue';
import 'cronstrue/locales/zh_CN';
import type { UiLanguage } from './settings';

const cronLocales: Record<UiLanguage, string> = {
  en: 'en',
  'zh-CN': 'zh_CN',
};

interface FormatCronDescriptionOptions {
  dropSeconds?: boolean;
  lowercase?: boolean;
}

export function formatCronDescription(
  cron: string,
  language: UiLanguage,
  options: FormatCronDescriptionOptions = {}
): string {
  const normalizedCron = options.dropSeconds ? cron.split(' ').slice(1).join(' ') : cron;
  const description = cronstrue.toString(normalizedCron, { locale: cronLocales[language] });
  return options.lowercase ? description.toLowerCase() : description;
}
