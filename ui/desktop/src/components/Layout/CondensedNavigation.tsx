import React, { useState, useEffect, useCallback, useRef } from 'react';
import {
  MessageSquare,
  History,
  GripVertical,
  Menu,
  ChevronDown,
  ChevronRight,
  Plus,
  ChefHat,
} from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
import { useNavigationContext } from './NavigationContext';
import { Z_INDEX } from './constants';
import { cn } from '../../utils';
import { useSidebarSessionStatus } from '../../hooks/useSidebarSessionStatus';
import {
  useNavigationSessions,
  getSessionDisplayName,
  truncateMessage,
} from '../../hooks/useNavigationSessions';
import { useNavigationDragDrop } from '../../hooks/useNavigationDragDrop';
import { useNavigationItems, useEscapeToClose } from '../../hooks/useNavigationItems';
import { SessionIndicators } from '../SessionIndicators';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '../ui/dropdown-menu';
import * as PopoverPrimitive from '@radix-ui/react-popover';

interface CondensedNavigationProps {
  className?: string;
}

export const CondensedNavigation: React.FC<CondensedNavigationProps> = ({ className }) => {
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

  const handleOverlayClose = useCallback(() => {
    if (effectiveNavigationMode === 'overlay') {
      setIsNavExpanded(false);
    }
  }, [effectiveNavigationMode, setIsNavExpanded]);

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
    isOverlayMode: effectiveNavigationMode === 'overlay',
    onClose: () => setIsNavExpanded(false),
  });

  const [chatPopoverOpen, setChatPopoverOpen] = useState(false);
  const [newChatHoverOpen, setNewChatHoverOpen] = useState(false);
  const hoverTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Ref for focusing navigation when opened
  const navContainerRef = useRef<HTMLDivElement>(null);

  const { getSessionStatus, clearUnread } = useSidebarSessionStatus();

  // Hover handlers with delay to allow mouse to travel between trigger and popover
  const handleHoverOpen = useCallback(() => {
    if (hoverTimeoutRef.current) {
      clearTimeout(hoverTimeoutRef.current);
      hoverTimeoutRef.current = null;
    }
    setNewChatHoverOpen(true);
  }, []);

  const handleHoverClose = useCallback(() => {
    hoverTimeoutRef.current = setTimeout(() => {
      setNewChatHoverOpen(false);
    }, 300);
  }, []);

  // Cleanup timeout on unmount
  useEffect(() => {
    return () => {
      if (hoverTimeoutRef.current) {
        clearTimeout(hoverTimeoutRef.current);
      }
    };
  }, []);

  // Fetch sessions when expanded and focus navigation
  useEffect(() => {
    if (isNavExpanded) {
      fetchSessions();
      requestAnimationFrame(() => {
        navContainerRef.current?.focus();
      });
    }
  }, [isNavExpanded, fetchSessions]);

  const toggleChatExpanded = () => {
    setIsChatExpanded(!isChatExpanded);
  };

  const isVertical = navigationPosition === 'left' || navigationPosition === 'right';

  const isOverlayMode = effectiveNavigationMode === 'overlay';
  const isTopPosition = navigationPosition === 'top';
  const isBottomPosition = navigationPosition === 'bottom';

  const navContent = (
    <motion.div
      ref={navContainerRef}
      tabIndex={-1}
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={{ duration: 0.15 }}
      className={cn(
        'bg-app outline-none',
        isOverlayMode && 'rounded-xl backdrop-blur-md shadow-lg p-2',
        isVertical ? 'flex flex-col gap-[2px] h-full' : 'flex flex-row items-stretch gap-[2px]',
        // Add 2px padding on the edge facing the content for vertical (only when not icon-only)
        !isOverlayMode && navigationPosition === 'left' && !isCondensedIconOnly && 'pr-[2px]',
        !isOverlayMode && navigationPosition === 'right' && !isCondensedIconOnly && 'pl-[2px]',
        // Add 2px padding on the edge facing the content for horizontal
        !isOverlayMode && isTopPosition && 'pb-[2px] pt-0',
        !isOverlayMode && isBottomPosition && 'pt-[2px] pb-0',
        // Allow hover buttons to overflow outside the nav container
        !isCondensedIconOnly && 'overflow-visible',
        className
      )}
    >
      {/* Top spacer (vertical only) */}
      {isVertical && (
        <div
          className={cn(
            'bg-background-default rounded-lg flex-shrink-0',
            isCondensedIconOnly ? 'h-[80px] w-[40px]' : 'h-[48px] w-full'
          )}
        />
      )}

      {/* Left spacer (horizontal top position only) */}
      {!isVertical && isTopPosition && (
        <div className="bg-background-default rounded-lg self-stretch w-[160px] flex-shrink-0" />
      )}

      {/* Navigation items container (vertical only) */}
      {isVertical ? (
        <div className="flex-1 min-h-0 flex flex-col gap-[2px]">
          {visibleItems.map((item, index) => {
            const Icon = item.icon;
            const active = isActive(item.path);
            const isDragging = draggedItem === item.id;
            const isDragOver = dragOverItem === item.id;
            const isChatItem = item.id === 'chat';

            return (
              <motion.div
                key={item.id}
                draggable
                onDragStart={(e) => handleDragStart(e as unknown as React.DragEvent, item.id)}
                onDragOver={(e) => handleDragOver(e as unknown as React.DragEvent, item.id)}
                onDrop={(e) => handleDrop(e as unknown as React.DragEvent, item.id)}
                onDragEnd={handleDragEnd}
                initial={{ opacity: 0 }}
                animate={{
                  opacity: isDragging ? 0.5 : 1,
                }}
                transition={{
                  duration: 0.15,
                  delay: index * 0.02,
                }}
                className={cn(
                  'relative cursor-move group',
                  isCondensedIconOnly ? 'flex-shrink-0' : 'w-full flex-shrink-0',
                  isDragOver && 'ring-2 ring-blue-500 rounded-lg',
                  isChatItem && !isCondensedIconOnly && 'overflow-visible'
                )}
              >
                <div
                  className={cn(
                    'flex flex-col',
                    isCondensedIconOnly ? 'items-start' : 'w-full',
                    isChatItem && !isCondensedIconOnly && 'overflow-visible'
                  )}
                >
                  {/* Chat item with dropdown in icon-only mode */}
                  {isChatItem && isCondensedIconOnly ? (
                    <DropdownMenu open={chatPopoverOpen} onOpenChange={setChatPopoverOpen}>
                      <DropdownMenuTrigger asChild>
                        <button
                          className={cn(
                            'flex items-center justify-center',
                            'rounded-lg transition-colors duration-200 no-drag',
                            'p-2.5',
                            active
                              ? 'bg-background-accent text-text-on-accent'
                              : 'bg-background-default hover:bg-background-medium'
                          )}
                        >
                          <Icon className="w-5 h-5" />
                        </button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent
                        className="w-64 p-1 bg-background-default border-border-subtle rounded-lg shadow-lg"
                        side={navigationPosition === 'left' ? 'right' : 'left'}
                        align="start"
                        sideOffset={8}
                      >
                        <DropdownMenuItem
                          onClick={handleNewChat}
                          className="flex items-center gap-2 px-3 py-2 text-sm rounded-lg cursor-pointer"
                        >
                          <Plus className="w-4 h-4 flex-shrink-0" />
                          <span>New Chat</span>
                        </DropdownMenuItem>
                        {recentSessions.length > 0 && <DropdownMenuSeparator className="my-1" />}
                        {recentSessions.map((session) => {
                          const status = getSessionStatus(session.id);
                          const isStreaming = status?.streamState === 'streaming';
                          const hasError = status?.streamState === 'error';
                          const hasUnread = status?.hasUnreadActivity ?? false;
                          const isActiveSession = session.id === activeSessionId;
                          return (
                            <DropdownMenuItem
                              key={session.id}
                              onClick={() => {
                                clearUnread(session.id);
                                handleSessionClick(session.id);
                              }}
                              className={cn(
                                'flex items-center gap-2 px-3 py-2 text-sm rounded-lg cursor-pointer',
                                isActiveSession && 'bg-background-medium'
                              )}
                            >
                              {session.recipe ? (
                                <ChefHat className="w-4 h-4 flex-shrink-0 text-text-muted" />
                              ) : (
                                <MessageSquare className="w-4 h-4 flex-shrink-0 text-text-muted" />
                              )}
                              <span className="truncate flex-1">
                                {truncateMessage(getSessionDisplayName(session), 30)}
                              </span>
                              <SessionIndicators
                                isStreaming={isStreaming}
                                hasUnread={hasUnread}
                                hasError={hasError}
                              />
                            </DropdownMenuItem>
                          );
                        })}
                        {recentSessions.length > 0 && (
                          <>
                            <DropdownMenuSeparator className="my-1" />
                            <DropdownMenuItem
                              onClick={() => handleNavClick('/sessions')}
                              className="flex items-center gap-2 px-3 py-2 text-sm rounded-lg cursor-pointer text-text-muted"
                            >
                              <History className="w-4 h-4 flex-shrink-0" />
                              <span>Show All</span>
                            </DropdownMenuItem>
                          </>
                        )}
                      </DropdownMenuContent>
                    </DropdownMenu>
                  ) : (
                    <>
                      {/* Chat row with hover popover for new chat button */}
                      {isChatItem && !isCondensedIconOnly ? (
                        <PopoverPrimitive.Root open={newChatHoverOpen}>
                          <PopoverPrimitive.Trigger asChild>
                            <motion.button
                              onClick={toggleChatExpanded}
                              onMouseEnter={handleHoverOpen}
                              onMouseLeave={handleHoverClose}
                              whileHover={{ scale: 1.02 }}
                              whileTap={{ scale: 0.98 }}
                              className={cn(
                                'flex flex-row items-center gap-2 outline-none',
                                'relative rounded-lg transition-colors duration-200 no-drag',
                                'w-full pl-2 pr-4 py-2.5',
                                active
                                  ? 'bg-background-accent text-text-on-accent'
                                  : 'bg-background-default hover:bg-background-medium'
                              )}
                            >
                              <div className="opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0">
                                <GripVertical className="w-4 h-4 text-text-muted" />
                              </div>
                              <Icon className="w-5 h-5 flex-shrink-0" />
                              <span className="text-sm font-medium text-left flex-1">
                                {item.label}
                              </span>
                              <div className="flex-shrink-0">
                                {isChatExpanded ? (
                                  <ChevronDown className="w-3 h-3 text-text-muted" />
                                ) : (
                                  <ChevronRight className="w-3 h-3 text-text-muted" />
                                )}
                              </div>
                            </motion.button>
                          </PopoverPrimitive.Trigger>
                          <PopoverPrimitive.Portal>
                            <PopoverPrimitive.Content
                              side={navigationPosition === 'left' ? 'right' : 'left'}
                              align="center"
                              sideOffset={4}
                              onMouseEnter={handleHoverOpen}
                              onMouseLeave={handleHoverClose}
                              style={{ zIndex: Z_INDEX.POPOVER }}
                              className="outline-none"
                            >
                              <button
                                onClick={(e) => {
                                  e.stopPropagation();
                                  handleNewChat();
                                  setNewChatHoverOpen(false);
                                }}
                                className={cn(
                                  'p-1.5 rounded-md outline-none',
                                  'bg-background-medium hover:bg-background-accent hover:text-text-on-accent',
                                  'flex items-center justify-center',
                                  'shadow-sm transition-all duration-150',
                                  'hover:scale-110 active:scale-95'
                                )}
                                title="New Chat"
                              >
                                <Plus className="w-4 h-4" />
                              </button>
                            </PopoverPrimitive.Content>
                          </PopoverPrimitive.Portal>
                        </PopoverPrimitive.Root>
                      ) : (
                        <motion.button
                          onClick={() => handleNavClick(item.path)}
                          whileHover={{ scale: 1.02 }}
                          whileTap={{ scale: 0.98 }}
                          className={cn(
                            'flex flex-row items-center gap-2',
                            'relative rounded-lg transition-colors duration-200 no-drag',
                            isCondensedIconOnly
                              ? 'justify-center p-2.5'
                              : 'w-full pl-2 pr-4 py-2.5',
                            active
                              ? 'bg-background-accent text-text-on-accent'
                              : 'bg-background-default hover:bg-background-medium'
                          )}
                        >
                          {!isCondensedIconOnly && (
                            <div className="opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0">
                              <GripVertical className="w-4 h-4 text-text-muted" />
                            </div>
                          )}
                          <Icon className="w-5 h-5 flex-shrink-0" />
                          {!isCondensedIconOnly && (
                            <span className="text-sm font-medium text-left flex-1">
                              {item.label}
                            </span>
                          )}
                          {!isCondensedIconOnly && item.getTag && (
                            <div className="flex items-center gap-1 flex-shrink-0">
                              <span
                                className={cn(
                                  'text-xs font-mono px-2 py-0.5 rounded-full',
                                  active
                                    ? 'bg-background-default/20 text-text-on-accent/80'
                                    : 'bg-background-muted text-text-muted'
                                )}
                              >
                                {item.getTag()}
                              </span>
                            </div>
                          )}
                        </motion.button>
                      )}
                    </>
                  )}
                  <AnimatePresence>
                    {isChatItem && isChatExpanded && !isCondensedIconOnly && (
                      <motion.div
                        initial={{ height: 0, opacity: 0 }}
                        animate={{ height: 'auto', opacity: 1 }}
                        exit={{ height: 0, opacity: 0 }}
                        transition={{ duration: 0.2 }}
                        className="overflow-hidden mt-[2px]"
                      >
                        <div className="bg-background-default rounded-lg py-1 flex flex-col gap-[2px]">
                          {recentSessions.map((session) => {
                            const status = getSessionStatus(session.id);
                            const isStreaming = status?.streamState === 'streaming';
                            const hasError = status?.streamState === 'error';
                            const hasUnread = status?.hasUnreadActivity ?? false;
                            const isActiveSession = session.id === activeSessionId;
                            return (
                              <button
                                key={session.id}
                                onClick={() => {
                                  clearUnread(session.id);
                                  handleSessionClick(session.id);
                                }}
                                className={cn(
                                  'w-full text-left py-1.5 px-2 text-xs rounded-md',
                                  'hover:bg-background-medium transition-colors',
                                  'flex items-center gap-2',
                                  isActiveSession && 'bg-background-medium'
                                )}
                              >
                                <div className="w-4 flex-shrink-0" />{' '}
                                {/* Spacer to align with grip icon */}
                                {session.recipe ? (
                                  <ChefHat className="w-4 h-4 flex-shrink-0 text-text-muted" />
                                ) : (
                                  <MessageSquare className="w-4 h-4 flex-shrink-0 text-text-muted" />
                                )}
                                <span className="truncate text-text-default flex-1">
                                  {truncateMessage(getSessionDisplayName(session))}
                                </span>
                                <SessionIndicators
                                  isStreaming={isStreaming}
                                  hasUnread={hasUnread}
                                  hasError={hasError}
                                />
                              </button>
                            );
                          })}
                        </div>
                      </motion.div>
                    )}
                  </AnimatePresence>
                </div>
              </motion.div>
            );
          })}

          {/* Bottom filler block - fills remaining space below nav items */}
          <div
            className={cn(
              'bg-background-default rounded-lg flex-1 min-h-[40px]',
              isCondensedIconOnly ? 'w-[40px]' : 'w-full'
            )}
          />
        </div>
      ) : (
        /* Horizontal navigation items */
        visibleItems.map((item, index) => {
          const Icon = item.icon;
          const active = isActive(item.path);
          const isDragging = draggedItem === item.id;
          const isDragOver = dragOverItem === item.id;
          const isChatItem = item.id === 'chat';

          return (
            <motion.div
              key={item.id}
              draggable
              onDragStart={(e) => handleDragStart(e as unknown as React.DragEvent, item.id)}
              onDragOver={(e) => handleDragOver(e as unknown as React.DragEvent, item.id)}
              onDrop={(e) => handleDrop(e as unknown as React.DragEvent, item.id)}
              onDragEnd={handleDragEnd}
              initial={{ opacity: 0 }}
              animate={{
                opacity: isDragging ? 0.5 : 1,
              }}
              transition={{
                duration: 0.15,
                delay: index * 0.02,
              }}
              className={cn(
                'relative cursor-move group flex-shrink-0',
                isDragOver && 'ring-2 ring-blue-500 rounded-lg',
                isChatItem && !isCondensedIconOnly && 'overflow-visible'
              )}
            >
              <div className="flex flex-col">
                {/* Chat item with dropdown in horizontal mode */}
                {isChatItem ? (
                  <>
                    <DropdownMenu open={chatPopoverOpen} onOpenChange={setChatPopoverOpen}>
                      <DropdownMenuTrigger asChild>
                        <motion.button
                          whileHover={{ scale: 1.02 }}
                          whileTap={{ scale: 0.98 }}
                          className={cn(
                            'flex flex-row items-center justify-center gap-2',
                            'relative rounded-lg transition-colors duration-200 no-drag',
                            'px-3 py-2.5',
                            active
                              ? 'bg-background-accent text-text-on-accent'
                              : 'bg-background-default hover:bg-background-medium'
                          )}
                        >
                          <Icon className="w-5 h-5 flex-shrink-0" />
                          <span className="text-sm font-medium text-left hidden min-[1200px]:block">
                            {item.label}
                          </span>
                        </motion.button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent
                        className="w-64 p-1 bg-background-default border-border-subtle rounded-lg shadow-lg"
                        side={isTopPosition ? 'bottom' : 'top'}
                        align="start"
                        sideOffset={8}
                      >
                        <DropdownMenuItem
                          onClick={handleNewChat}
                          className="flex items-center gap-2 px-3 py-2 text-sm rounded-lg cursor-pointer"
                        >
                          <Plus className="w-4 h-4 flex-shrink-0" />
                          <span>New Chat</span>
                        </DropdownMenuItem>
                        {recentSessions.length > 0 && <DropdownMenuSeparator className="my-1" />}
                        {recentSessions.map((session) => {
                          const status = getSessionStatus(session.id);
                          const isStreaming = status?.streamState === 'streaming';
                          const hasError = status?.streamState === 'error';
                          const hasUnread = status?.hasUnreadActivity ?? false;
                          const isActiveSession = session.id === activeSessionId;
                          return (
                            <DropdownMenuItem
                              key={session.id}
                              onClick={() => {
                                clearUnread(session.id);
                                handleSessionClick(session.id);
                              }}
                              className={cn(
                                'flex items-center gap-2 px-3 py-2 text-sm rounded-lg cursor-pointer',
                                isActiveSession && 'bg-background-medium'
                              )}
                            >
                              {session.recipe ? (
                                <ChefHat className="w-4 h-4 flex-shrink-0 text-text-muted" />
                              ) : (
                                <MessageSquare className="w-4 h-4 flex-shrink-0 text-text-muted" />
                              )}
                              <span className="truncate flex-1">
                                {truncateMessage(getSessionDisplayName(session), 30)}
                              </span>
                              <SessionIndicators
                                isStreaming={isStreaming}
                                hasUnread={hasUnread}
                                hasError={hasError}
                              />
                            </DropdownMenuItem>
                          );
                        })}
                        {recentSessions.length > 0 && (
                          <>
                            <DropdownMenuSeparator className="my-1" />
                            <DropdownMenuItem
                              onClick={() => handleNavClick('/sessions')}
                              className="flex items-center gap-2 px-3 py-2 text-sm rounded-lg cursor-pointer text-text-muted"
                            >
                              <History className="w-4 h-4 flex-shrink-0" />
                              <span>Show All</span>
                            </DropdownMenuItem>
                          </>
                        )}
                      </DropdownMenuContent>
                    </DropdownMenu>
                    {!chatPopoverOpen && (
                      <motion.button
                        onClick={(e) => {
                          e.stopPropagation();
                          handleNewChat();
                        }}
                        whileHover={{ scale: 1.1 }}
                        whileTap={{ scale: 0.95 }}
                        className={cn(
                          'absolute left-1/2 -translate-x-1/2 p-1.5 rounded-md z-10',
                          'opacity-0 group-hover:opacity-100 transition-opacity',
                          'bg-background-medium hover:bg-background-accent hover:text-text-on-accent',
                          'flex items-center justify-center',
                          isTopPosition ? '-bottom-9' : '-top-9'
                        )}
                        title="New Chat"
                      >
                        <Plus className="w-4 h-4" />
                      </motion.button>
                    )}
                  </>
                ) : (
                  <motion.button
                    onClick={() => handleNavClick(item.path)}
                    whileHover={{ scale: 1.02 }}
                    whileTap={{ scale: 0.98 }}
                    className={cn(
                      'flex flex-row items-center gap-2 px-3 py-2.5',
                      'relative rounded-lg transition-colors duration-200 no-drag',
                      active
                        ? 'bg-background-accent text-text-on-accent'
                        : 'bg-background-default hover:bg-background-medium'
                    )}
                  >
                    <Icon className="w-5 h-5 flex-shrink-0" />
                    <span className="text-sm font-medium text-left hidden min-[1200px]:block">
                      {item.label}
                    </span>
                  </motion.button>
                )}
              </div>
            </motion.div>
          );
        })
      )}

      {/* Right spacer (horizontal only) */}
      {!isVertical && (
        <div className="bg-background-default rounded-lg self-stretch flex-1 min-w-[40px]" />
      )}
    </motion.div>
  );

  // Overlay mode: render with backdrop
  if (isOverlayMode) {
    return (
      <AnimatePresence>
        {isNavExpanded && (
          <div className="fixed inset-0" style={{ zIndex: Z_INDEX.OVERLAY }}>
            {/* Backdrop */}
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              className="absolute inset-0 bg-black/20 backdrop-blur-sm"
              onClick={() => setIsNavExpanded(false)}
            />

            {/* Scrollable container for navigation panel */}
            <div className="absolute inset-0 overflow-y-auto pointer-events-none">
              <div
                className={cn(
                  'min-h-full flex p-4',
                  navigationPosition === 'top' && 'items-start justify-center pt-16',
                  navigationPosition === 'bottom' && 'items-end justify-center pb-8',
                  navigationPosition === 'left' && 'items-center justify-start pl-4',
                  navigationPosition === 'right' && 'items-center justify-end pr-4'
                )}
              >
                <div className="pointer-events-auto">{navContent}</div>
              </div>
            </div>
          </div>
        )}
      </AnimatePresence>
    );
  }

  // Push mode: render inline
  if (!isNavExpanded) return null;
  return navContent;
};

// Trigger button to open navigation
interface NavTriggerProps {
  className?: string;
}

export const NavTrigger: React.FC<NavTriggerProps> = ({ className }) => {
  const { isNavExpanded, setIsNavExpanded } = useNavigationContext();

  return (
    <button
      onClick={() => setIsNavExpanded(!isNavExpanded)}
      className={cn(
        'p-2 rounded-lg transition-all duration-150',
        'hover:bg-background-medium',
        'flex items-center justify-center',
        isNavExpanded && 'bg-background-medium',
        className
      )}
      aria-label={isNavExpanded ? 'Close navigation' : 'Open navigation'}
    >
      <Menu className="w-5 h-5 text-text-muted" />
    </button>
  );
};
