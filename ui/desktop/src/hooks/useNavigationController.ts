import { useEffect, useRef, useCallback } from 'react';
import { useNavigationContext } from '../components/Layout/NavigationContext';
import { useSidebarSessionStatus } from './useSidebarSessionStatus';
import { useNavigationSessions } from './useNavigationSessions';
import { useNavigationDragDrop } from './useNavigationDragDrop';
import { useNavigationItems, useEscapeToClose } from './useNavigationItems';

export function useNavigationController() {
  const {
    isNavExpanded,
    setIsNavExpanded,
    effectiveNavigationMode,
    navigationPosition,
    preferences,
    updatePreferences,
    isCondensedIconOnly,
    isChatExpanded,
    setIsChatExpanded,
  } = useNavigationContext();

  const { visibleItems, isActive } = useNavigationItems({ preferences });

  const isOverlayMode = effectiveNavigationMode === 'overlay';

  const handleOverlayClose = useCallback(() => {
    if (isOverlayMode) {
      setIsNavExpanded(false);
    }
  }, [isOverlayMode, setIsNavExpanded]);

  const {
    recentSessions,
    activeSessionId,
    fetchSessions,
    handleNavClick,
    handleNewChat,
    handleSessionClick,
  } = useNavigationSessions({ onNavigate: handleOverlayClose });

  const { draggedItem, dragOverItem, handleDragStart, handleDragOver, handleDrop, handleDragEnd } =
    useNavigationDragDrop({ preferences, updatePreferences });

  useEscapeToClose({
    isOpen: isNavExpanded,
    isOverlayMode,
    onClose: () => setIsNavExpanded(false),
  });

  const { getSessionStatus, clearUnread } = useSidebarSessionStatus();

  const navFocusRef = useRef<HTMLDivElement>(null);

  // Fetch sessions and focus nav when expanded
  useEffect(() => {
    if (isNavExpanded) {
      fetchSessions();
      requestAnimationFrame(() => {
        navFocusRef.current?.focus();
      });
    }
  }, [isNavExpanded, fetchSessions]);

  return {
    // Navigation context values
    isNavExpanded,
    setIsNavExpanded,
    effectiveNavigationMode,
    navigationPosition,
    preferences,
    updatePreferences,
    isCondensedIconOnly,
    isChatExpanded,
    setIsChatExpanded,

    // Computed
    isOverlayMode,
    visibleItems,
    isActive,

    // Sessions
    recentSessions,
    activeSessionId,
    fetchSessions,
    handleNavClick,
    handleNewChat,
    handleSessionClick,
    getSessionStatus,
    clearUnread,

    // Drag and drop
    draggedItem,
    dragOverItem,
    handleDragStart,
    handleDragOver,
    handleDrop,
    handleDragEnd,

    // Refs
    navFocusRef,
  };
}
