import React, { createContext, useContext, useState, useCallback, useEffect } from 'react';
import { useConfig } from '../components/ConfigContext';

export interface HotkeyConfig {
  key: string;
  ctrl: boolean;
  meta: boolean;
  shift: boolean;
  alt: boolean;
}

interface ToolOutputContextType {
  isExpandAll: boolean;
  setExpandAll: (expand: boolean) => void;
  toggleExpandAll: () => void;
  hotkey: HotkeyConfig;
  setHotkey: (hotkey: HotkeyConfig) => void;
  isHotkeyActive: boolean;
  setIsHotkeyActive: (active: boolean) => void;
}

const defaultHotkey: HotkeyConfig = {
  key: 'e',
  ctrl: true,
  meta: false,
  shift: false,
  alt: false,
};

const ToolOutputContext = createContext<ToolOutputContextType | undefined>(undefined);

export function ToolOutputProvider({ children }: { children: React.ReactNode }) {
  const [isExpandAll, setExpandAll] = useState(false);
  const [hotkey, setHotkeyState] = useState<HotkeyConfig>(defaultHotkey);
  const [isHotkeyActive, setIsHotkeyActive] = useState(false);
  const { read, upsert } = useConfig();

  const toggleExpandAll = useCallback(() => {
    setExpandAll((prev) => !prev);
  }, []);

  const setExpandAllWrapper = useCallback((expand: boolean) => {
    setExpandAll(expand);
  }, []);

  const setHotkey = useCallback(
    async (newHotkey: HotkeyConfig) => {
      setHotkeyState(newHotkey);
      // Store in config system
      try {
        await upsert('tool_output_hotkey', newHotkey, false);
      } catch (error) {
        console.warn('Failed to save hotkey preference:', error);
      }
    },
    [upsert]
  );

  // Load hotkey preference from config on mount
  useEffect(() => {
    const loadHotkey = async () => {
      try {
        const savedHotkey = await read('tool_output_hotkey', false);
        if (savedHotkey && typeof savedHotkey === 'object') {
          setHotkeyState(savedHotkey as HotkeyConfig);
        }
      } catch (error) {
        console.warn('Failed to load hotkey preference:', error);
      }
    };
    loadHotkey();
  }, [read]);

  const contextValue = {
    isExpandAll,
    setExpandAll: setExpandAllWrapper,
    toggleExpandAll,
    hotkey,
    setHotkey,
    isHotkeyActive,
    setIsHotkeyActive,
  };

  return <ToolOutputContext.Provider value={contextValue}>{children}</ToolOutputContext.Provider>;
}

export function useToolOutputContext() {
  const context = useContext(ToolOutputContext);
  if (!context) {
    throw new Error('useToolOutputContext must be used within ToolOutputProvider');
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
 * Custom hook to handle hotkey for toggling tool output display
 * This hook adds a global keyboard listener that only works when active
 */
export function useToolOutput() {
  const { toggleExpandAll, hotkey, isHotkeyActive, setIsHotkeyActive } = useToolOutputContext();

  // Auto-deactivate hotkey when user leaves conversation context
  useEffect(() => {
    if (!isHotkeyActive) return;

    const handleBlur = () => {
      setIsHotkeyActive(false);
    };

    window.addEventListener('blur', handleBlur);
    return () => {
      window.removeEventListener('blur', handleBlur);
    };
  }, [isHotkeyActive, setIsHotkeyActive]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!isHotkeyActive) return;
      if (matchesHotkey(e, hotkey)) {
        e.preventDefault();
        toggleExpandAll();
      }
    };

    if (isHotkeyActive) {
      document.addEventListener('keydown', handleKeyDown);
    }

    return () => {
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [toggleExpandAll, hotkey, isHotkeyActive]);
}
