import React, { useState, useEffect, useRef } from 'react';
import { MessageSquare, History, GripVertical, Plus, ChefHat } from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
import { useNavigationContext } from './NavigationContext';
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

interface ExpandedNavigationProps {
  className?: string;
}

export const ExpandedNavigation: React.FC<ExpandedNavigationProps> = ({ className }) => {
  const {
    isNavExpanded,
    setIsNavExpanded,
    effectiveNavigationMode,
    navigationPosition,
    preferences,
    updatePreferences,
  } = useNavigationContext();

  const { visibleItems, isActive } = useNavigationItems({ preferences });

  const handleOverlayClose = () => {
    if (effectiveNavigationMode === 'overlay') {
      setIsNavExpanded(false);
    }
  };

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

  const [chatDropdownOpen, setChatDropdownOpen] = useState(false);
  const [gridColumns, setGridColumns] = useState(2);
  const [gridMeasured, setGridMeasured] = useState(false);
  const [tilesReady, setTilesReady] = useState(false);
  const [isClosing, setIsClosing] = useState(false);
  const prevIsNavExpandedRef = useRef(isNavExpanded);
  const gridRef = useRef<HTMLDivElement>(null);
  const navContainerRef = useRef<HTMLDivElement>(null);

  const { getSessionStatus, clearUnread } = useSidebarSessionStatus();

  // Fetch sessions when expanded and focus navigation
  useEffect(() => {
    if (isNavExpanded) {
      fetchSessions();
      requestAnimationFrame(() => {
        navContainerRef.current?.focus();
      });
    }
  }, [isNavExpanded, fetchSessions]);

  // Detect when nav is closing (transition from expanded to collapsed)
  useEffect(() => {
    if (prevIsNavExpandedRef.current && !isNavExpanded) {
      setIsClosing(true);
      setTilesReady(false);
    } else if (!prevIsNavExpandedRef.current && isNavExpanded) {
      setIsClosing(false);
    }
    prevIsNavExpandedRef.current = isNavExpanded;
  }, [isNavExpanded]);

  // Control when tiles are ready to animate in (after panel opens)
  useEffect(() => {
    if (!isNavExpanded) {
      setTilesReady(false);
      return;
    }

    const timeoutId = setTimeout(() => {
      setTilesReady(true);
    }, 150);

    return () => clearTimeout(timeoutId);
  }, [isNavExpanded]);

  // Track grid columns for spacer tiles
  useEffect(() => {
    if (!isNavExpanded) {
      setGridMeasured(false);
      return;
    }

    const updateGridColumns = () => {
      if (gridRef.current) {
        const gridStyle = window.getComputedStyle(gridRef.current);
        const columns = gridStyle.gridTemplateColumns
          .split(' ')
          .filter((col) => col.trim() !== '').length;
        if (columns > 0) {
          setGridColumns(columns);
          setGridMeasured(true);
        }
      }
    };

    const timeoutId = setTimeout(updateGridColumns, 100);

    const resizeObserver = new ResizeObserver(() => {
      updateGridColumns();
    });

    if (gridRef.current) {
      resizeObserver.observe(gridRef.current);
    }

    window.addEventListener('resize', updateGridColumns);

    return () => {
      clearTimeout(timeoutId);
      resizeObserver.disconnect();
      window.removeEventListener('resize', updateGridColumns);
    };
  }, [isNavExpanded, navigationPosition]);

  const isOverlayMode = effectiveNavigationMode === 'overlay';

  // Determine if content should be visible (not during close animation for push mode)
  const showContent = !isClosing || isOverlayMode;

  const navContent = (
    <motion.div
      ref={navContainerRef}
      tabIndex={-1}
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      transition={{ duration: 0.15 }}
      className={cn(
        'bg-app h-full overflow-hidden outline-none',
        isOverlayMode && 'backdrop-blur-md shadow-2xl rounded-lg p-4',
        // Add 2px padding on the edge facing the content (push mode only)
        !isOverlayMode && navigationPosition === 'top' && 'pb-[2px]',
        !isOverlayMode && navigationPosition === 'bottom' && 'pt-[2px]',
        !isOverlayMode && navigationPosition === 'left' && 'pr-[2px]',
        !isOverlayMode && navigationPosition === 'right' && 'pl-[2px]',
        className
      )}
    >
      {/* Navigation grid - square tiles with scroll */}
      {/* Completely hide content during close animation to prevent layout thrashing */}
      {showContent ? (
        <div
          ref={gridRef}
          className={cn(
            'grid gap-[2px] overflow-y-auto overflow-x-hidden h-full',
            isOverlayMode && 'gap-3'
          )}
          style={{
            // Use CSS grid with auto-fill for responsive tiles based on container width
            gridTemplateColumns: isOverlayMode
              ? // For overlay mode: responsive - single row on larger screens, wraps to 2 rows on smaller
                'repeat(auto-fit, minmax(120px, 1fr))'
              : navigationPosition === 'left' || navigationPosition === 'right'
                ? // For left/right: larger min size (140px) to trigger single column sooner
                  'repeat(auto-fill, minmax(140px, 1fr))'
                : // For top/bottom: auto-fit with larger min size to fit all in 1 row on large screens, wrap to 2 rows on smaller
                  'repeat(auto-fit, minmax(160px, 1fr))',
            // Align items to start so they don't stretch vertically
            alignContent: 'start',
          }}
        >
          {visibleItems.map((item, index) => {
            const Icon = item.icon;
            const active = isActive(item.path);
            const isDragging = draggedItem === item.id;
            const isDragOver = dragOverItem === item.id;
            const isChatItem = item.id === 'chat';

            // Chat tile with dropdown
            if (isChatItem) {
              return (
                <DropdownMenu
                  key={item.id}
                  open={chatDropdownOpen}
                  onOpenChange={setChatDropdownOpen}
                >
                  <motion.div
                    draggable
                    onDragStart={(e) => handleDragStart(e as unknown as React.DragEvent, item.id)}
                    onDragOver={(e) => handleDragOver(e as unknown as React.DragEvent, item.id)}
                    onDrop={(e) => handleDrop(e as unknown as React.DragEvent, item.id)}
                    onDragEnd={handleDragEnd}
                    initial={{ opacity: 0 }}
                    animate={{
                      opacity: tilesReady ? (isDragging ? 0.5 : 1) : 0,
                    }}
                    transition={{
                      duration: 0.15,
                      delay: tilesReady ? index * 0.03 : 0,
                    }}
                    className={cn(
                      'relative cursor-move group',
                      isDragOver && 'ring-2 ring-blue-500 rounded-lg'
                    )}
                  >
                    <div className="relative">
                      <DropdownMenuTrigger asChild>
                        <motion.div
                          className={cn(
                            'w-full relative flex flex-col',
                            'rounded-lg',
                            'transition-colors duration-200',
                            'aspect-square cursor-pointer',
                            active
                              ? 'bg-background-accent text-text-on-accent'
                              : 'bg-background-default hover:bg-background-medium'
                          )}
                        >
                          <div className="flex-1 flex flex-col items-start justify-between p-5 no-drag text-left">
                            {/* Drag handle */}
                            <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity z-10">
                              <GripVertical className="w-4 h-4 text-text-muted" />
                            </div>

                            {/* Tag/Badge */}
                            {item.getTag && (
                              <div
                                className={cn(
                                  'absolute top-3 px-2 py-1 rounded-full',
                                  item.tagAlign === 'left' ? 'left-8' : 'right-8',
                                  'bg-background-muted'
                                )}
                              >
                                <span className="text-xs font-mono text-text-muted">
                                  {item.getTag()}
                                </span>
                              </div>
                            )}

                            {/* Icon and Label at bottom */}
                            <div className="mt-auto w-full">
                              <Icon className="w-6 h-6 mb-2" />
                              <h2 className="font-light text-left text-xl">{item.label}</h2>
                            </div>
                          </div>
                        </motion.div>
                      </DropdownMenuTrigger>

                      {/* New Chat button - bottom right corner, outside DropdownMenuTrigger */}
                      <motion.button
                        onClick={(e) => {
                          e.stopPropagation();
                          e.preventDefault();
                          handleNewChat();
                        }}
                        whileHover={{ scale: 1.1 }}
                        whileTap={{ scale: 0.95 }}
                        className={cn(
                          'absolute bottom-3 right-3 p-2 rounded-md z-10',
                          'opacity-0 group-hover:opacity-100 transition-opacity',
                          active
                            ? 'bg-background-default/20 hover:bg-background-default/30 text-text-on-accent'
                            : 'bg-background-medium hover:bg-background-accent hover:text-text-on-accent',
                          'flex items-center justify-center'
                        )}
                        title="New Chat"
                      >
                        <Plus className="w-4 h-4" />
                      </motion.button>
                    </div>
                    <DropdownMenuContent
                      className="w-64 p-1 bg-background-default border-border-subtle rounded-lg shadow-lg z-[10001]"
                      side="right"
                      align="start"
                      sideOffset={8}
                    >
                      {/* New chat button */}
                      <DropdownMenuItem
                        onClick={handleNewChat}
                        className="flex items-center gap-2 px-3 py-2 text-sm rounded-lg cursor-pointer"
                      >
                        <Plus className="w-4 h-4 flex-shrink-0" />
                        <span>New Chat</span>
                      </DropdownMenuItem>

                      {recentSessions.length > 0 && <DropdownMenuSeparator className="my-1" />}

                      {/* Recent sessions */}
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

                      {/* Show All button */}
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
                  </motion.div>
                </DropdownMenu>
              );
            }

            // Regular tile for non-chat items
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
                  opacity: tilesReady ? (isDragging ? 0.5 : 1) : 0,
                }}
                transition={{
                  duration: 0.15,
                  delay: tilesReady ? index * 0.03 : 0,
                }}
                className={cn(
                  'relative cursor-move group',
                  isDragOver && 'ring-2 ring-blue-500 rounded-lg'
                )}
              >
                <motion.div
                  className={cn(
                    'w-full relative flex flex-col',
                    'rounded-lg',
                    'transition-colors duration-200',
                    'aspect-square',
                    active
                      ? 'bg-background-accent text-text-on-accent'
                      : 'bg-background-default hover:bg-background-medium'
                  )}
                >
                  {/* Main button area */}
                  <button
                    onClick={() => handleNavClick(item.path)}
                    className="flex-1 flex flex-col items-start justify-between p-5 no-drag text-left"
                  >
                    {/* Drag handle */}
                    <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity z-10">
                      <GripVertical className="w-4 h-4 text-text-muted" />
                    </div>

                    {/* Tag/Badge */}
                    {item.getTag && (
                      <div
                        className={cn(
                          'absolute top-3 px-2 py-1 rounded-full',
                          item.tagAlign === 'left' ? 'left-8' : 'right-8',
                          'bg-background-muted'
                        )}
                      >
                        <span className="text-xs font-mono text-text-muted">{item.getTag()}</span>
                      </div>
                    )}

                    {/* Icon and Label at bottom */}
                    <div className="mt-auto w-full">
                      <Icon className="w-6 h-6 mb-2" />
                      <h2 className="font-light text-left text-xl">{item.label}</h2>
                    </div>
                  </button>
                </motion.div>
              </motion.div>
            );
          })}

          {/* Spacer tiles to fill empty grid spaces - only render after grid is measured */}
          {!isOverlayMode &&
            gridMeasured &&
            gridColumns >= 2 &&
            Array.from({
              // For left/right: add extra rows of spacers to fill vertical space
              // For top/bottom: just fill remaining spaces in the last row
              length:
                navigationPosition === 'left' || navigationPosition === 'right'
                  ? ((gridColumns - (visibleItems.length % gridColumns)) % gridColumns) +
                    gridColumns * 6 // Fill last row + 6 more rows
                  : (gridColumns - (visibleItems.length % gridColumns)) % gridColumns,
            }).map((_, index) => (
              <div key={`spacer-${index}`} className="relative">
                <div className="w-full aspect-square rounded-lg bg-background-default" />
              </div>
            ))}
        </div>
      ) : null}
    </motion.div>
  );

  // Overlay mode: render with backdrop
  if (isOverlayMode) {
    return (
      <AnimatePresence>
        {isNavExpanded && (
          <div className="fixed inset-0 z-[10000]">
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
              <div className="min-h-full flex items-center justify-center p-8">
                <div className="pointer-events-auto max-w-3xl w-full">{navContent}</div>
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
