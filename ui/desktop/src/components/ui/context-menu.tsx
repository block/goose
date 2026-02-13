import React, { useEffect, useRef, useCallback, useState } from 'react';
import { createPortal } from 'react-dom';
import { cn } from '../../utils';

interface ContextMenuPosition {
  x: number;
  y: number;
}

interface ContextMenuProps {
  children: React.ReactNode;
  content: React.ReactNode;
}

/**
 * A lightweight context menu (right-click menu) component.
 * Styled to match the existing DropdownMenu from Radix UI.
 *
 * Usage:
 *   <ContextMenu content={<><ContextMenuItem onSelect={...}>Rename</ContextMenuItem></>}>
 *     <button>Right-click me</button>
 *   </ContextMenu>
 */
export function ContextMenu({ children, content }: ContextMenuProps) {
  const [position, setPosition] = useState<ContextMenuPosition | null>(null);
  const menuRef = useRef<HTMLDivElement>(null);

  const handleContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setPosition({ x: e.clientX, y: e.clientY });
  }, []);

  const close = useCallback(() => {
    setPosition(null);
  }, []);

  // Close on click outside or Escape
  useEffect(() => {
    if (!position) return;

    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        close();
      }
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        close();
      }
    };

    // Use a microtask to avoid immediately closing from the same click
    requestAnimationFrame(() => {
      document.addEventListener('mousedown', handleClickOutside);
      document.addEventListener('keydown', handleKeyDown);
    });

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
      document.removeEventListener('keydown', handleKeyDown);
    };
  }, [position, close]);

  // Adjust position to keep menu within viewport
  useEffect(() => {
    if (!position || !menuRef.current) return;
    const menu = menuRef.current;
    const rect = menu.getBoundingClientRect();
    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;

    let adjustedX = position.x;
    let adjustedY = position.y;

    if (position.x + rect.width > viewportWidth) {
      adjustedX = viewportWidth - rect.width - 8;
    }
    if (position.y + rect.height > viewportHeight) {
      adjustedY = viewportHeight - rect.height - 8;
    }

    if (adjustedX !== position.x || adjustedY !== position.y) {
      setPosition({ x: adjustedX, y: adjustedY });
    }
  }, [position]);

  return (
    <>
      <div onContextMenu={handleContextMenu}>{children}</div>
      {position &&
        createPortal(
          <div
            ref={menuRef}
            className={cn(
              'fixed z-50 min-w-[8rem] overflow-hidden rounded-xl border p-1 shadow-lg',
              'bg-background-default text-text-default',
              'animate-in fade-in-0 zoom-in-95'
            )}
            style={{ top: position.y, left: position.x }}
          >
            <ContextMenuCloseContext.Provider value={close}>
              {content}
            </ContextMenuCloseContext.Provider>
          </div>,
          document.body
        )}
    </>
  );
}

// Internal context for closing the menu from menu items
const ContextMenuCloseContext = React.createContext<() => void>(() => {});

export function ContextMenuItem({
  children,
  onSelect,
  variant = 'default',
  disabled = false,
  className,
  dataTestId,
}: {
  children: React.ReactNode;
  onSelect: () => void;
  variant?: 'default' | 'destructive';
  disabled?: boolean;
  className?: string;
  dataTestId?: string;
}) {
  const close = React.useContext(ContextMenuCloseContext);

  const handleClick = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      if (disabled) return;
      close();
      onSelect();
    },
    [close, onSelect, disabled]
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        e.stopPropagation();
        if (disabled) return;
        close();
        onSelect();
      }
    },
    [close, onSelect, disabled]
  );

  return (
    <div
      role="menuitem"
      tabIndex={disabled ? -1 : 0}
      data-testid={dataTestId}
      onClick={handleClick}
      onKeyDown={handleKeyDown}
      className={cn(
        'relative flex cursor-default items-center gap-2 rounded-sm px-2 py-1.5 text-sm outline-hidden select-none',
        variant === 'destructive'
          ? 'text-text-danger hover:bg-background-danger/10 focus:bg-background-danger/10'
          : 'hover:bg-background-muted focus:bg-background-muted',
        disabled && 'pointer-events-none opacity-50',
        className
      )}
    >
      {children}
    </div>
  );
}

export function ContextMenuSeparator({ className }: { className?: string }) {
  return <div className={cn('bg-border-default -mx-1 my-1 h-px', className)} />;
}
