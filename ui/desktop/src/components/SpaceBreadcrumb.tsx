import React, { useState, useEffect, useRef, useCallback } from 'react';
import { ChevronRight, Hash, ChevronDown, Layers } from 'lucide-react';
import { matrixService } from '../services/MatrixService';
import { motion, AnimatePresence } from 'framer-motion';

interface SpaceBreadcrumbProps {
  roomId: string;
  className?: string;
  onSpaceClick?: (spaceId: string) => void;
  onRoomClick?: (roomId: string) => void;
}

interface BreadcrumbData {
  spaceName: string;
  spaceId: string;
  roomName: string;
  roomId: string;
}

export const SpaceBreadcrumb: React.FC<SpaceBreadcrumbProps> = ({ 
  roomId, 
  className = '',
  onSpaceClick,
  onRoomClick 
}) => {
  const [breadcrumb, setBreadcrumb] = useState<BreadcrumbData | null>(null);
  const [showSpaceDropdown, setShowSpaceDropdown] = useState(false);
  const [showRoomDropdown, setShowRoomDropdown] = useState(false);
  const [allSpaces, setAllSpaces] = useState<Array<{ roomId: string; name: string }>>([]);
  const [spaceRooms, setSpaceRooms] = useState<Array<{ roomId: string; name: string }>>([]);
  
  const spaceDropdownRef = useRef<HTMLDivElement>(null);
  const roomDropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const loadBreadcrumb = () => {
      try {
        // Get all Spaces
        const spaces = matrixService.getSpaces();
        setAllSpaces(spaces.map(s => ({ roomId: s.roomId, name: s.name || 'Unnamed Space' })));
        
        // Find which Space contains this room
        for (const space of spaces) {
          const children = matrixService.getSpaceChildren(space.roomId);
          const childRoom = children.find(child => child.roomId === roomId);
          
          if (childRoom) {
            // Found the parent Space!
            setBreadcrumb({
              spaceName: space.name || 'Unnamed Space',
              spaceId: space.roomId,
              roomName: childRoom.name || 'Unnamed Room',
              roomId: roomId,
            });
            
            // Load all rooms in this Space
            setSpaceRooms(children.map(c => ({ 
              roomId: c.roomId, 
              name: c.name || 'Unnamed Room' 
            })));
            return;
          }
        }
        
        // Room is not in any Space
        setBreadcrumb(null);
      } catch (error) {
        console.error('Failed to load Space breadcrumb:', error);
        setBreadcrumb(null);
      }
    };

    loadBreadcrumb();
    
    // Listen for Space changes
    const handleSpaceUpdate = () => {
      loadBreadcrumb();
    };
    
    matrixService.on('spaceChildAdded', handleSpaceUpdate);
    matrixService.on('spaceChildRemoved', handleSpaceUpdate);
    matrixService.on('ready', handleSpaceUpdate);
    
    return () => {
      matrixService.off('spaceChildAdded', handleSpaceUpdate);
      matrixService.off('spaceChildRemoved', handleSpaceUpdate);
      matrixService.off('ready', handleSpaceUpdate);
    };
  }, [roomId]);

  // Close dropdowns when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (spaceDropdownRef.current && !spaceDropdownRef.current.contains(event.target as Node)) {
        setShowSpaceDropdown(false);
      }
      if (roomDropdownRef.current && !roomDropdownRef.current.contains(event.target as Node)) {
        setShowRoomDropdown(false);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const handleSpaceSelect = useCallback((spaceId: string) => {
    setShowSpaceDropdown(false);
    onSpaceClick?.(spaceId);
  }, [onSpaceClick]);

  const handleRoomSelect = useCallback((selectedRoomId: string) => {
    setShowRoomDropdown(false);
    onRoomClick?.(selectedRoomId);
  }, [onRoomClick]);

  // Don't render if room is not in a Space
  if (!breadcrumb) {
    return null;
  }

  return (
    <motion.div
      initial={{ opacity: 0, y: -10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.2 }}
      className={`flex items-center gap-2 px-4 py-2 bg-background-muted border-b border-border-default ${className}`}
    >
      {/* Space Selector */}
      <div className="relative" ref={spaceDropdownRef}>
        <button
          onClick={() => setShowSpaceDropdown(!showSpaceDropdown)}
          className="flex items-center gap-2 text-text-muted hover:text-text-default transition-colors rounded px-2 py-1 hover:bg-background-hover"
          aria-label="Select space"
          aria-expanded={showSpaceDropdown}
        >
          <div className="w-5 h-5 bg-background-accent/20 rounded flex items-center justify-center">
            <Layers className="w-3 h-3 text-text-on-accent" />
          </div>
          <span className="text-sm font-medium">{breadcrumb.spaceName}</span>
          <ChevronDown className="w-3 h-3" />
        </button>

        <AnimatePresence>
          {showSpaceDropdown && (
            <motion.div
              initial={{ opacity: 0, y: -10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              transition={{ duration: 0.15 }}
              className="absolute top-full left-0 mt-1 w-64 bg-background-elevated border border-border-default rounded-lg shadow-lg overflow-hidden z-50"
            >
              <div className="max-h-80 overflow-y-auto">
                {allSpaces.map((space) => (
                  <button
                    key={space.roomId}
                    onClick={() => handleSpaceSelect(space.roomId)}
                    className={`w-full flex items-center gap-2 px-3 py-2 text-left hover:bg-background-hover transition-colors ${
                      space.roomId === breadcrumb.spaceId ? 'bg-background-accent/10 text-text-on-accent' : 'text-text-default'
                    }`}
                  >
                    <Layers className="w-4 h-4 flex-shrink-0" />
                    <span className="text-sm truncate">{space.name}</span>
                    {space.roomId === breadcrumb.spaceId && (
                      <span className="ml-auto text-xs text-text-muted">Current</span>
                    )}
                  </button>
                ))}
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>

      {/* Separator */}
      <ChevronRight className="w-4 h-4 text-text-muted" />

      {/* Room Selector */}
      <div className="relative" ref={roomDropdownRef}>
        <button
          onClick={() => setShowRoomDropdown(!showRoomDropdown)}
          className="flex items-center gap-2 text-text-default hover:text-text-emphasis transition-colors rounded px-2 py-1 hover:bg-background-hover"
          aria-label="Select room"
          aria-expanded={showRoomDropdown}
        >
          <Hash className="w-4 h-4" />
          <span className="text-sm">{breadcrumb.roomName}</span>
          <ChevronDown className="w-3 h-3" />
        </button>

        <AnimatePresence>
          {showRoomDropdown && (
            <motion.div
              initial={{ opacity: 0, y: -10 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -10 }}
              transition={{ duration: 0.15 }}
              className="absolute top-full left-0 mt-1 w-64 bg-background-elevated border border-border-default rounded-lg shadow-lg overflow-hidden z-50"
            >
              <div className="max-h-80 overflow-y-auto">
                {spaceRooms.map((room) => (
                  <button
                    key={room.roomId}
                    onClick={() => handleRoomSelect(room.roomId)}
                    className={`w-full flex items-center gap-2 px-3 py-2 text-left hover:bg-background-hover transition-colors ${
                      room.roomId === breadcrumb.roomId ? 'bg-background-accent/10 text-text-on-accent' : 'text-text-default'
                    }`}
                  >
                    <Hash className="w-4 h-4 flex-shrink-0" />
                    <span className="text-sm truncate">{room.name}</span>
                    {room.roomId === breadcrumb.roomId && (
                      <span className="ml-auto text-xs text-text-muted">Current</span>
                    )}
                  </button>
                ))}
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>

      {/* Space Room Badge */}
      <div className="ml-auto">
        <span className="text-xs px-2 py-0.5 rounded-full bg-blue-500/10 text-blue-600 dark:text-blue-400">
          Space Room
        </span>
      </div>
    </motion.div>
  );
};
