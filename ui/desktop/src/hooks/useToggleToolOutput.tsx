import React, { createContext, useContext, useState, useCallback, useEffect } from 'react';

export interface HotkeyConfig {
  key: string;
  ctrl: boolean;
  meta: boolean;
  shift: boolean;
  alt: boolean;
}

interface ToggleToolOutputContextType {
  isExpandAll: boolean;
  setExpandAll: (expand: boolean) => void;
  toggleExpandAll: () => void;
  hotkey: HotkeyConfig;
  setHotkey: (hotkey: HotkeyConfig) => void;
}

const defaultHotkey: HotkeyConfig = {
  key: 'e',
  ctrl: true,
  meta: false,
  shift: false,
  alt: false,
};

const ToggleToolOutputContext = createContext<ToggleToolOutputContextType | undefined>(undefined);

export function ToggleToolOutputProvider({ children }: { children: React.ReactNode }) {
  const [isExpandAll, setExpandAll] = useState(false);
  const [hotkey, setHotkeyState] = useState<HotkeyConfig>(defaultHotkey);

  const toggleExpandAll = useCallback(() => {
    setExpandAll((prev) => !prev);
  }, []);

  const setExpandAllWrapper = useCallback((expand: boolean) => {
    setExpandAll(expand);
  }, []);

  const setHotkey = useCallback((newHotkey: HotkeyConfig) => {
    setHotkeyState(newHotkey);
    // Store in localStorage for persistence
    try {
      localStorage.setItem('toggleToolOutputHotkey', JSON.stringify(newHotkey));
    } catch (error) {
      console.warn('Failed to save hotkey preference:', error);
    }
  }, []);

  // Load hotkey preference from localStorage on mount
  useEffect(() => {
    try {
      const savedHotkey = localStorage.getItem('toggleToolOutputHotkey');
      if (savedHotkey) {
        const parsedHotkey = JSON.parse(savedHotkey);
        setHotkeyState(parsedHotkey);
      }
    } catch (error) {
      console.warn('Failed to load hotkey preference:', error);
    }
  }, []);

  const contextValue = {
    isExpandAll,
    setExpandAll: setExpandAllWrapper,
    toggleExpandAll,
    hotkey,
    setHotkey,
  };

  return (
    <ToggleToolOutputContext.Provider value={contextValue}>
      {children}
    </ToggleToolOutputContext.Provider>
  );
}

export function useToggleToolOutputContext() {
  const context = useContext(ToggleToolOutputContext);
  if (!context) {
    throw new Error('useToggleToolOutputContext must be used within ToggleToolOutputProvider');
  }
  return context;
}

/**
 * Check if a keyboard event matches the hotkey configuration
 */
function matchesHotkey(event: KeyboardEvent, hotkey: HotkeyConfig): boolean {
  return (
    event.key.toLowerCase() === hotkey.key.toLowerCase() &&
    event.ctrlKey === hotkey.ctrl &&
    event.metaKey === hotkey.meta &&
    event.shiftKey === hotkey.shift &&
    event.altKey === hotkey.alt
  );
}

/**
 * Format hotkey for display (e.g., "Ctrl+E")
 */
export function formatHotkey(hotkey: HotkeyConfig): string {
  const modifiers = [];
  if (hotkey.ctrl) modifiers.push('Ctrl');
  if (hotkey.meta) modifiers.push('Cmd');
  if (hotkey.alt) modifiers.push('Alt');
  if (hotkey.shift) modifiers.push('Shift');

  const key = hotkey.key.length === 1 ? hotkey.key.toUpperCase() : hotkey.key;
  modifiers.push(key);

  return modifiers.join('+');
}

/**
 * Custom hook to handle hotkey for toggling full tool output display
 * This hook adds a global keyboard listener that toggles the expand all state
 */
export function useToggleToolOutput() {
  const { toggleExpandAll, hotkey } = useToggleToolOutputContext();

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (matchesHotkey(e, hotkey)) {
        e.preventDefault();
        toggleExpandAll();
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => {
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [toggleExpandAll, hotkey]);
}
