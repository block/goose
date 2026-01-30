import React, {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
  useRef,
  ReactNode,
} from 'react';

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
  'history',
  'recipes',
  'scheduler',
  'extensions',
  'settings',
];

export const DEFAULT_ENABLED_ITEMS = [...DEFAULT_ITEM_ORDER];

// Breakpoint for responsive behavior (overlay mode for expanded, icon-only for condensed left/right)
const RESPONSIVE_BREAKPOINT = 700;

interface NavigationContextValue {
  // Navigation state
  isNavExpanded: boolean;
  setIsNavExpanded: (expanded: boolean) => void;

  // Mode: push content or overlay (user preference)
  navigationMode: NavigationMode;
  setNavigationMode: (mode: NavigationMode) => void;

  // Effective mode: accounts for responsive breakpoints
  effectiveNavigationMode: NavigationMode;

  // Style: expanded tiles or condensed list (user preference)
  navigationStyle: NavigationStyle;
  setNavigationStyle: (style: NavigationStyle) => void;

  // Effective style: overlay mode forces expanded
  effectiveNavigationStyle: NavigationStyle;

  // Position: where nav appears
  navigationPosition: NavigationPosition;
  setNavigationPosition: (position: NavigationPosition) => void;

  // Item customization
  preferences: NavigationPreferences;
  updatePreferences: (prefs: NavigationPreferences) => void;

  // Helpers
  isHorizontalNav: boolean;

  // Whether condensed nav should show icon-only (small screens + left/right position)
  isCondensedIconOnly: boolean;

  // Chat sessions list expansion state
  isChatExpanded: boolean;
  setIsChatExpanded: (expanded: boolean) => void;
}

const NavigationContext = createContext<NavigationContextValue | null>(null);

export const useNavigationContext = () => {
  const context = useContext(NavigationContext);
  if (!context) {
    throw new Error('useNavigationContext must be used within NavigationProvider');
  }
  return context;
};

// Safe hook that returns defaults if outside provider
export const useNavigationContextSafe = () => {
  const context = useContext(NavigationContext);
  return context;
};

interface NavigationProviderProps {
  children: ReactNode;
}

export const NavigationProvider: React.FC<NavigationProviderProps> = ({ children }) => {
  // Load initial state from localStorage
  const [isNavExpanded, setIsNavExpandedState] = useState<boolean>(() => {
    const stored = localStorage.getItem('navigation_expanded');
    return stored === 'true';
  });

  // Track if window is below breakpoint for responsive behavior (using matchMedia like use-mobile.ts)
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

  // Track window resize using matchMedia (more reliable than resize event)
  useEffect(() => {
    const mql = window.matchMedia(`(max-width: ${RESPONSIVE_BREAKPOINT - 1}px)`);
    const onChange = () => {
      const below = window.innerWidth < RESPONSIVE_BREAKPOINT;
      setIsBelowBreakpoint(below);
    };
    mql.addEventListener('change', onChange);
    // Set initial value
    setIsBelowBreakpoint(window.innerWidth < RESPONSIVE_BREAKPOINT);
    return () => mql.removeEventListener('change', onChange);
  }, []);

  // Persist changes to localStorage and dispatch events
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

  // Listen for external changes (e.g., from settings in another window)
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

  // Force overlay mode for expanded navigation when window is narrow
  const effectiveNavigationMode: NavigationMode =
    navigationStyle === 'expanded' && isBelowBreakpoint ? 'overlay' : navigationMode;

  // Force expanded style when overlay mode is selected (overlay is always expanded and centered)
  const effectiveNavigationStyle: NavigationStyle =
    navigationMode === 'overlay' ? 'expanded' : navigationStyle;

  // Condensed nav should show icon-only on small screens when positioned left/right
  // Uses the same breakpoint as expanded overlay
  const isCondensedIconOnly = !isHorizontalNav && isBelowBreakpoint;

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
    isChatExpanded,
    setIsChatExpanded,
  };

  return <NavigationContext.Provider value={value}>{children}</NavigationContext.Provider>;
};
