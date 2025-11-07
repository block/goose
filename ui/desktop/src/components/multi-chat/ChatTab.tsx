import React, { useState, useRef, useEffect } from 'react';
import { X, GripVertical } from 'lucide-react';
import { Session } from '../../api';

interface ChatTabProps {
  session: Session | null;
  isActive: boolean;
  isLoading?: boolean;
  onSelect: () => void;
  onClose: () => void;
  onDragStart?: (e: React.DragEvent) => void;
  onDragOver?: (e: React.DragEvent) => void;
  onDrop?: (e: React.DragEvent) => void;
  hasUnread?: boolean;
  status?: 'waiting' | 'working' | 'done' | 'error';
}

export const ChatTab: React.FC<ChatTabProps> = ({
  session,
  isActive,
  isLoading = false,
  onSelect,
  onClose,
  onDragStart,
  onDragOver,
  onDrop,
  hasUnread = false,
  status = 'done',
}) => {
  const [isHovered, setIsHovered] = useState(false);
  const tabRef = useRef<HTMLDivElement>(null);

  // Truncate long session names
  const displayName = session?.name || 'New Chat';
  const truncatedName = displayName.length > 20 ? `${displayName.slice(0, 20)}...` : displayName;

  const handleClose = (e: React.MouseEvent) => {
    e.stopPropagation();
    onClose();
  };

  return (
    <div
      ref={tabRef}
      draggable
      onDragStart={onDragStart}
      onDragOver={onDragOver}
      onDrop={onDrop}
      onClick={onSelect}
      onMouseEnter={() => setIsHovered(true)}
      onMouseLeave={() => setIsHovered(false)}
      className={`
        group relative flex items-center gap-2 px-4 py-3 flex-1 min-w-[5px]
        mr-0.5 cursor-pointer
        transition-all duration-200
        ${isActive 
          ? 'bg-background-accent text-text-on-accent rounded-t-2xl' 
          : 'bg-background-default text-text-default hover:bg-background-medium rounded-2xl'
        }
      `}
      data-session-id={session?.id}
    >
      {/* Unread indicator */}
      {hasUnread && !isActive && (
        <div className="absolute left-2 top-1/2 -translate-y-1/2 w-2 h-2 bg-blue-500 rounded-full" />
      )}

      {/* Session name - clickable area */}
      <div className="flex-1 min-w-0 flex items-center gap-2">
        {/* Status indicator or Drag handle */}
        <div className="flex-shrink-0 w-3 h-3 flex items-center justify-center">
          {isHovered ? (
            <div className="cursor-grab active:cursor-grabbing">
              <GripVertical className={`w-3 h-3 ${isActive ? 'text-text-on-accent/70' : 'text-text-muted'}`} />
            </div>
          ) : (
            <div 
              className={`w-1 h-1 rounded-full ${
                status === 'waiting' ? 'bg-blue-500' :
                status === 'working' ? 'bg-blue-500 animate-pulse' :
                status === 'error' ? 'bg-red-500' :
                'bg-green-500'
              }`}
            />
          )}
        </div>
        
        <span className={`
          text-sm truncate block flex-1
          ${isLoading ? 'animate-pulse' : ''}
        `}>
          {truncatedName}
        </span>
      </div>

      {/* Close button - only render when hovered or active */}
      {(isHovered || isActive) && (
        <button
          onClick={handleClose}
          className={`
            flex-shrink-0 p-1 rounded transition-all duration-150
            ${isActive 
              ? 'hover:bg-background-accent-hover' 
              : 'hover:bg-background-strong'
            }
          `}
          aria-label="Close tab"
        >
          <X className="w-3.5 h-3.5" />
        </button>
      )}
    </div>
  );
};
