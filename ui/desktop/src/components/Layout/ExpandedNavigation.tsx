import React, { useState, useEffect, useRef } from 'react';
import { GripVertical } from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
import { Z_INDEX } from './constants';
import { cn } from '../../utils';
import { useNavigationController } from '../../hooks/useNavigationController';
import { DropdownMenu, DropdownMenuTrigger } from '../ui/dropdown-menu';
import { ChatSessionsDropdown } from './navigation';

interface ExpandedNavigationProps {
  className?: string;
}

export const ExpandedNavigation: React.FC<ExpandedNavigationProps> = ({ className }) => {
  const {
    isNavExpanded,
    setIsNavExpanded,
    effectiveNavigationMode,
    navigationPosition,
    isOverlayMode,
    visibleItems,
    isActive,
    recentSessions,
    activeSessionId,
    handleNavClick,
    handleNewChat,
    handleSessionClick,
    getSessionStatus,
    clearUnread,
    draggedItem,
    dragOverItem,
    handleDragStart,
    handleDragOver,
    handleDrop,
    handleDragEnd,
    navFocusRef,
  } = useNavigationController();

  const [chatDropdownOpen, setChatDropdownOpen] = useState(false);
  const [gridColumns, setGridColumns] = useState(2);
  const [gridMeasured, setGridMeasured] = useState(false);
  const [tilesReady, setTilesReady] = useState(false);
  const [isClosing, setIsClosing] = useState(false);
  const prevIsNavExpandedRef = useRef(isNavExpanded);
  const gridRef = useRef<HTMLDivElement>(null);

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

    setGridMeasured(false);
    let rafId: number;

    const updateGridColumns = () => {
      if (!gridRef.current) return;

      const parent = gridRef.current.parentElement;
      if (!parent) return;

      const parentStyle = window.getComputedStyle(parent);
      const availableWidth =
        parent.clientWidth -
        parseFloat(parentStyle.paddingLeft) -
        parseFloat(parentStyle.paddingRight);

      const minSize = navigationPosition === 'left' || navigationPosition === 'right' ? 140 : 160;
      const isOverlay = effectiveNavigationMode === 'overlay';
      const gap = isOverlay ? 12 : 2; // gap-3 = 12px for overlay, gap-[2px] for push
      const cols = Math.max(1, Math.floor((availableWidth + gap) / (minSize + gap)));

      setGridColumns(cols);
      setGridMeasured(true);
    };

    const timeoutId = setTimeout(() => {
      rafId = requestAnimationFrame(updateGridColumns);
    }, 100);

    const resizeObserver = new ResizeObserver(() => {
      cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(updateGridColumns);
    });

    const parent = gridRef.current?.parentElement;
    if (parent) {
      resizeObserver.observe(parent);
    }

    return () => {
      clearTimeout(timeoutId);
      cancelAnimationFrame(rafId);
      resizeObserver.disconnect();
    };
  }, [isNavExpanded, navigationPosition, effectiveNavigationMode]);

  const isPushTopNav = !isOverlayMode && navigationPosition === 'top';
  const dragStyle = isPushTopNav ? ({ WebkitAppRegion: 'drag' } as React.CSSProperties) : undefined;

  // Determine if content should be visible (not during close animation for push mode)
  const showContent = !isClosing || isOverlayMode;

  const navContent = (
    <motion.div
      ref={navFocusRef}
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
            // When nav is at top in push mode, the global drag region is hidden.
            // Apply drag to the grid so empty space is draggable but the hamburger button area isn't.
            ...(dragStyle || {}),
            // Use CSS grid with auto-fit for responsive tiles based on container width
            gridTemplateColumns: isOverlayMode
              ? // For overlay mode: responsive - single row on larger screens, wraps to 2 rows on smaller
                'repeat(auto-fit, minmax(120px, 1fr))'
              : navigationPosition === 'left' || navigationPosition === 'right'
                ? // For left/right: auto-fit collapses empty tracks so items fill the container
                  'repeat(auto-fit, minmax(140px, 1fr))'
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
                    </div>
                    <ChatSessionsDropdown
                      sessions={recentSessions}
                      activeSessionId={activeSessionId}
                      side="right"
                      zIndex={Z_INDEX.DROPDOWN_ABOVE_OVERLAY}
                      getSessionStatus={getSessionStatus}
                      clearUnread={clearUnread}
                      onNewChat={handleNewChat}
                      onSessionClick={handleSessionClick}
                      onShowAll={() => handleNavClick('/sessions')}
                    />
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
