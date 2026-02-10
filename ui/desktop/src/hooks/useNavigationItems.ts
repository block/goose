import { useMemo, useEffect } from 'react';
import { useLocation } from 'react-router-dom';
import { Home, MessageSquare, FileText, AppWindow, Clock, Puzzle, Settings } from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { useConfig } from '../components/ConfigContext';
import type { NavigationPreferences } from '../components/Layout/NavigationContext';

export interface NavItem {
  id: string;
  path: string;
  label: string;
  icon: LucideIcon;
  getTag?: () => string;
  tagAlign?: 'left' | 'right';
  hasSubItems?: boolean;
}

export const NAV_ITEMS: NavItem[] = [
  { id: 'home', path: '/', label: 'Home', icon: Home },
  { id: 'chat', path: '/pair', label: 'Chat', icon: MessageSquare, hasSubItems: true },
  { id: 'recipes', path: '/recipes', label: 'Recipes', icon: FileText },
  { id: 'apps', path: '/apps', label: 'Apps', icon: AppWindow },
  { id: 'scheduler', path: '/schedules', label: 'Scheduler', icon: Clock },
  { id: 'extensions', path: '/extensions', label: 'Extensions', icon: Puzzle },
  { id: 'settings', path: '/settings', label: 'Settings', icon: Settings },
];

export function getNavItemById(id: string): NavItem | undefined {
  return NAV_ITEMS.find((item) => item.id === id);
}

interface UseNavigationItemsOptions {
  preferences: NavigationPreferences;
}

export function useNavigationItems({ preferences }: UseNavigationItemsOptions) {
  const location = useLocation();
  const configContext = useConfig();

  const appsExtensionEnabled = !!configContext.extensionsList?.find((ext) => ext.name === 'apps')
    ?.enabled;

  const visibleItems = useMemo(() => {
    return preferences.itemOrder
      .filter((id) => preferences.enabledItems.includes(id))
      .map((id) => getNavItemById(id))
      .filter((item): item is NavItem => item !== undefined)
      .filter((item) => {
        if (item.path === '/apps') {
          return appsExtensionEnabled;
        }
        return true;
      });
  }, [preferences.itemOrder, preferences.enabledItems, appsExtensionEnabled]);

  const isActive = (path: string) => location.pathname === path;

  return {
    visibleItems,
    isActive,
    appsExtensionEnabled,
  };
}

interface UseEscapeToCloseOptions {
  isOpen: boolean;
  isOverlayMode: boolean;
  onClose: () => void;
}

export function useEscapeToClose({ isOpen, isOverlayMode, onClose }: UseEscapeToCloseOptions) {
  useEffect(() => {
    if (!(isOverlayMode && isOpen)) {
      return;
    }

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        onClose();
      }
    };

    document.addEventListener('keydown', handleKeyDown, { capture: true });
    return () => document.removeEventListener('keydown', handleKeyDown, { capture: true });
  }, [isOpen, isOverlayMode, onClose]);
}
