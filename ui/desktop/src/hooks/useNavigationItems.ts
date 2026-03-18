import { Home, MessageSquare, FileText, AppWindow, Clock, Puzzle, Settings } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import i18n from '../i18n';

export interface NavItem {
  id: string;
  path: string;
  label: string;
  icon: LucideIcon;
  getTag?: () => string;
  tagAlign?: 'left' | 'right';
  hasSubItems?: boolean;
}

const t = (key: string) => i18n.t(key);

export const NAV_ITEMS: NavItem[] = [
  { id: 'home', path: '/', label: t('nav.home'), icon: Home },
  { id: 'chat', path: '/pair', label: t('nav.chat'), icon: MessageSquare, hasSubItems: true },
  { id: 'recipes', path: '/recipes', label: t('nav.recipes'), icon: FileText },
  { id: 'apps', path: '/apps', label: t('nav.apps'), icon: AppWindow },
  { id: 'scheduler', path: '/schedules', label: t('nav.scheduler'), icon: Clock },
  { id: 'extensions', path: '/extensions', label: t('nav.extensions'), icon: Puzzle },
  { id: 'settings', path: '/settings', label: t('nav.settings'), icon: Settings },
];

export function getNavItemById(id: string): NavItem | undefined {
  return NAV_ITEMS.find((item) => item.id === id);
}
