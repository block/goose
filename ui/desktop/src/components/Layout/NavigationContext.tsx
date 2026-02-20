import React, {
  createContext,
  ReactNode,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { useLocation } from 'react-router-dom';
import { useConfig } from '../ConfigContext';
import { AppEvents } from '../../constants/events';
import { getNavItemById, type NavItem } from '../../hooks/useNavigationItems';

export type NavigationMode = 'push' | 'overlay';
export type NavigationStyle = 'expanded' | 'condensed';
export type NavigationPosition = 'top' | 'bottom' | 'left' | 'right';

export interface NavigationPreferences {
  itemOrder: string[];
  enabledItems: string[];
}

export const DEFAULT_ITEM_ORDER = [
  'home',
  'chat',
  'recipes',
  'apps',
  'scheduler',
  'extensions',
  'settings',
];

export const DEFAULT_ENABLED_ITEMS = [...DEFAULT_ITEM_ORDER];

const RESPONSIVE_BREAKPOINT = 700;

export type StreamState = 'idle' | 'loading' | 'streaming' | 'error';

export interface SessionStatus {
  streamState: StreamState;
  hasUnreadActivity: boolean;
}

interface NavigationContextValue {
  isNavExpanded: boolean;
  setIsNavExpanded: (expanded: boolean) => void;
  navigationMode: NavigationMode;
  setNavigationMode: (mode: NavigationMode) => void;
  effectiveNavigationMode: NavigationMode;
  navigationStyle: NavigationStyle;
  setNavigationStyle: (style: NavigationStyle) => void;
  effectiveNavigationStyle: NavigationStyle;
  navigationPosition: NavigationPosition;
  setNavigationPosition: (position: NavigationPosition) => void;
  preferences: NavigationPreferences;
  updatePreferences: (prefs: NavigationPreferences) => void;
  isHorizontalNav: boolean;
  isCondensedIconOnly: boolean;
  isOverlayMode: boolean;
  isChatExpanded: boolean;
  setIsChatExpanded: (expanded: boolean) => void;
  visibleItems: NavItem[];
  isActive: (path: string) => boolean;
  draggedItem: string | null;
  dragOverItem: string | null;
  handleDragStart: (e: React.DragEvent, itemId: string) => void;
  handleDragOver: (e: React.DragEvent, itemId: string) => void;
  handleDrop: (e: React.DragEvent, dropItemId: string) => void;
  handleDragEnd: () => void;
  getSessionStatus: (sessionId: string) => SessionStatus | undefined;
  clearUnread: (sessionId: string) => void;
}

const NavigationContext = createContext<NavigationContextValue | null>(null);

export const useNavigationContext = () => {
  const context = useContext(NavigationContext);
  if (!context) {
    throw new Error('useNavigationContext must be used within NavigationProvider');
  }
  return context;
};

export const useNavigationContextSafe = () => {
  return useContext(NavigationContext);
};

interface NavigationProviderProps {
  children: ReactNode;
}

export const NavigationProvider: React.FC<NavigationProviderProps> = ({ children }) => {
  const [isNavExpanded, setIsNavExpandedState] = useState<boolean>(() => {
    const stored = localStorage.getItem('navigation_expanded');
    return stored !== 'false';
  });

  const [isBelowBreakpoint, setIsBelowBreakpoint] = useState<boolean>(
    () => window.innerWidth < RESPONSIVE_BREAKPOINT
  );

  const [navigationMode, setNavigationModeState] = useState<NavigationMode>(() => {
    const stored = localStorage.getItem('navigation_mode');
    return (stored as NavigationMode) || 'push';
  });

  const [navigationStyle, setNavigationStyleState] = useState<NavigationStyle>(() => {
    const stored = localStorage.getItem('navigation_style');
    return (stored as NavigationStyle) || 'condensed';
  });

  const [navigationPosition, setNavigationPositionState] = useState<NavigationPosition>(() => {
    const stored = localStorage.getItem('navigation_position');
    return (stored as NavigationPosition) || 'left';
  });

  const [preferences, setPreferences] = useState<NavigationPreferences>(() => {
    const stored = localStorage.getItem('navigation_preferences');
    if (stored) {
      try {
        return JSON.parse(stored);
      } catch {
        console.error('Failed to parse navigation preferences');
      }
    }
    return {
      itemOrder: DEFAULT_ITEM_ORDER,
      enabledItems: DEFAULT_ENABLED_ITEMS,
    };
  });

  const [isChatExpanded, setIsChatExpandedState] = useState<boolean>(() => {
    const stored = localStorage.getItem('navigation_chat_expanded');
    return stored !== 'false';
  });

  useEffect(() => {
    const mql = window.matchMedia(`(max-width: ${RESPONSIVE_BREAKPOINT - 1}px)`);
    const onChange = () => {
      const below = window.innerWidth < RESPONSIVE_BREAKPOINT;
      setIsBelowBreakpoint(below);
    };
    mql.addEventListener('change', onChange);
    setIsBelowBreakpoint(window.innerWidth < RESPONSIVE_BREAKPOINT);
    return () => mql.removeEventListener('change', onChange);
  }, []);

  const setIsNavExpanded = useCallback((expanded: boolean) => {
    setIsNavExpandedState(expanded);
    localStorage.setItem('navigation_expanded', String(expanded));
  }, []);

  const setNavigationMode = useCallback((mode: NavigationMode) => {
    setNavigationModeState(mode);
    localStorage.setItem('navigation_mode', mode);
    window.dispatchEvent(new CustomEvent('navigation-mode-changed', { detail: { mode } }));
  }, []);

  const setNavigationStyle = useCallback((style: NavigationStyle) => {
    setNavigationStyleState(style);
    localStorage.setItem('navigation_style', style);
    window.dispatchEvent(new CustomEvent('navigation-style-changed', { detail: { style } }));
  }, []);

  const setNavigationPosition = useCallback((position: NavigationPosition) => {
    setNavigationPositionState(position);
    localStorage.setItem('navigation_position', position);
    window.dispatchEvent(new CustomEvent('navigation-position-changed', { detail: { position } }));
  }, []);

  const updatePreferences = useCallback((newPrefs: NavigationPreferences) => {
    setPreferences(newPrefs);
    localStorage.setItem('navigation_preferences', JSON.stringify(newPrefs));
    window.dispatchEvent(new CustomEvent('navigation-preferences-updated', { detail: newPrefs }));
  }, []);

  const setIsChatExpanded = useCallback((expanded: boolean) => {
    setIsChatExpandedState(expanded);
    localStorage.setItem('navigation_chat_expanded', String(expanded));
  }, []);

  const isNavExpandedRef = useRef(isNavExpanded);
  useEffect(() => {
    isNavExpandedRef.current = isNavExpanded;
  }, [isNavExpanded]);

  useEffect(() => {
    const handleToggleNavigation = () => {
      setIsNavExpanded(!isNavExpandedRef.current);
    };

    window.electron.on('toggle-navigation', handleToggleNavigation);
    return () => {
      window.electron.off('toggle-navigation', handleToggleNavigation);
    };
  }, [setIsNavExpanded]);

  useEffect(() => {
    const handleModeChange = (e: Event) => {
      const { mode } = (e as CustomEvent).detail;
      setNavigationModeState(mode);
    };
    const handleStyleChange = (e: Event) => {
      const { style } = (e as CustomEvent).detail;
      setNavigationStyleState(style);
    };
    const handlePositionChange = (e: Event) => {
      const { position } = (e as CustomEvent).detail;
      setNavigationPositionState(position);
    };
    const handlePrefsChange = (e: Event) => {
      const prefs = (e as CustomEvent).detail;
      setPreferences(prefs);
    };

    window.addEventListener('navigation-mode-changed', handleModeChange);
    window.addEventListener('navigation-style-changed', handleStyleChange);
    window.addEventListener('navigation-position-changed', handlePositionChange);
    window.addEventListener('navigation-preferences-updated', handlePrefsChange);

    return () => {
      window.removeEventListener('navigation-mode-changed', handleModeChange);
      window.removeEventListener('navigation-style-changed', handleStyleChange);
      window.removeEventListener('navigation-position-changed', handlePositionChange);
      window.removeEventListener('navigation-preferences-updated', handlePrefsChange);
    };
  }, []);

  const isHorizontalNav = navigationPosition === 'top' || navigationPosition === 'bottom';
  const effectiveNavigationMode: NavigationMode =
    navigationStyle === 'expanded' && isBelowBreakpoint ? 'overlay' : navigationMode;
  const effectiveNavigationStyle: NavigationStyle =
    navigationMode === 'overlay' ? 'expanded' : navigationStyle;
  const isCondensedIconOnly = !isHorizontalNav && isBelowBreakpoint;
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

  const isActive = useCallback((path: string) => location.pathname === path, [location.pathname]);

  // --- Drag and drop ---

  const [draggedItem, setDraggedItem] = useState<string | null>(null);
  const [dragOverItem, setDragOverItem] = useState<string | null>(null);

  const handleDragStart = useCallback((e: React.DragEvent, itemId: string) => {
    setDraggedItem(itemId);
    e.dataTransfer.effectAllowed = 'move';
  }, []);

  const handleDragOver = useCallback(
    (e: React.DragEvent, itemId: string) => {
      e.preventDefault();
      if (draggedItem && draggedItem !== itemId) {
        setDragOverItem(itemId);
      }
    },
    [draggedItem]
  );

  const handleDrop = useCallback(
    (e: React.DragEvent, dropItemId: string) => {
      e.preventDefault();
      if (!draggedItem || draggedItem === dropItemId) return;

      const newOrder = [...preferences.itemOrder];
      const draggedIndex = newOrder.indexOf(draggedItem);
      const dropIndex = newOrder.indexOf(dropItemId);

      if (draggedIndex === -1 || dropIndex === -1) return;

      newOrder.splice(draggedIndex, 1);
      newOrder.splice(dropIndex, 0, draggedItem);

      updatePreferences({
        ...preferences,
        itemOrder: newOrder,
      });

      setDraggedItem(null);
      setDragOverItem(null);
    },
    [draggedItem, preferences, updatePreferences]
  );

  const handleDragEnd = useCallback(() => {
    setDraggedItem(null);
    setDragOverItem(null);
  }, []);

  const [sessionStatuses, setSessionStatuses] = useState<Map<string, SessionStatus>>(new Map());

  useEffect(() => {
    const handleStatusUpdate = (event: Event) => {
      const { sessionId, streamState } = (event as CustomEvent).detail;

      setSessionStatuses((prev) => {
        const existing = prev.get(sessionId);
        const wasStreaming = existing?.streamState === 'streaming';
        const isNowIdle = streamState === 'idle';
        const shouldMarkUnread = wasStreaming && isNowIdle;

        const next = new Map(prev);
        next.set(sessionId, {
          streamState,
          hasUnreadActivity: existing?.hasUnreadActivity || shouldMarkUnread,
        });
        return next;
      });
    };

    window.addEventListener(AppEvents.SESSION_STATUS_UPDATE, handleStatusUpdate);
    return () => window.removeEventListener(AppEvents.SESSION_STATUS_UPDATE, handleStatusUpdate);
  }, []);

  const getSessionStatus = useCallback(
    (sessionId: string): SessionStatus | undefined => {
      return sessionStatuses.get(sessionId);
    },
    [sessionStatuses]
  );

  const clearUnread = useCallback((sessionId: string) => {
    setSessionStatuses((prev) => {
      const status = prev.get(sessionId);
      if (status?.hasUnreadActivity) {
        const next = new Map(prev);
        next.set(sessionId, { ...status, hasUnreadActivity: false });
        return next;
      }
      return prev;
    });
  }, []);

  const isOverlayMode = effectiveNavigationMode === 'overlay';

  useEffect(() => {
    if (!(isOverlayMode && isNavExpanded)) {
      return;
    }

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        setIsNavExpanded(false);
      }
    };

    document.addEventListener('keydown', handleKeyDown, { capture: true });
    return () => document.removeEventListener('keydown', handleKeyDown, { capture: true });
  }, [isNavExpanded, isOverlayMode, setIsNavExpanded]);

  const value: NavigationContextValue = {
    isNavExpanded,
    setIsNavExpanded,
    navigationMode,
    setNavigationMode,
    effectiveNavigationMode,
    navigationStyle,
    setNavigationStyle,
    effectiveNavigationStyle,
    navigationPosition,
    setNavigationPosition,
    preferences,
    updatePreferences,
    isHorizontalNav,
    isCondensedIconOnly,
    isOverlayMode,
    isChatExpanded,
    setIsChatExpanded,
    visibleItems,
    isActive,
    draggedItem,
    dragOverItem,
    handleDragStart,
    handleDragOver,
    handleDrop,
    handleDragEnd,
    getSessionStatus,
    clearUnread,
  };

  return <NavigationContext.Provider value={value}>{children}</NavigationContext.Provider>;
};
