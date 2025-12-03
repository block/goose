import React, { useState, useEffect, useRef } from 'react';
import { ChevronRight, Hash, Layers, ChevronDown, Users } from 'lucide-react';
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

interface Space {
  roomId: string;
  name: string;
  memberCount: number;
}

interface Room {
  roomId: string;
  name: string;
  topic?: string;
  memberCount: number;
}

export const SpaceBreadcrumb: React.FC<SpaceBreadcrumbProps> = ({ 
  roomId, 
  className = '', 
  onSpaceClick,
  onRoomClick 
}) => {
  const [breadcrumb, setBreadcrumb] = useState<BreadcrumbData | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [showSpaceDropdown, setShowSpaceDropdown] = useState(false);
  const [showRoomDropdown, setShowRoomDropdown] = useState(false);
  const [allSpaces, setAllSpaces] = useState<Space[]>([]);
  const [roomsInCurrentSpace, setRoomsInCurrentSpace] = useState<Room[]>([]);
  
  const spaceDropdownRef = useRef<HTMLDivElement>(null);
  const roomDropdownRef = useRef<HTMLDivElement>(null);

  // Load breadcrumb data
  useEffect(() => {
    const loadBreadcrumb = () => {
      setIsLoading(true);
      try {
        // Get all Spaces
        const spaces = matrixService.getSpaces();
        
        // Map spaces for dropdown
        const spacesData: Space[] = spaces.map(space => ({
          roomId: space.roomId,
          name: space.name || 'Unnamed Space',
          memberCount: space.members.length,
        }));
        setAllSpaces(spacesData);
        
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
            
            // Load all rooms in this space for the room dropdown
            const roomsData: Room[] = children.map(child => {
              const matrixRoom = matrixService.client?.getRoom(child.roomId);
              return {
                roomId: child.roomId,
                name: child.name || 'Unnamed Room',
                topic: matrixRoom?.currentState.getStateEvents('m.room.topic', '')?.getContent()?.topic,
                memberCount: matrixRoom?.getMembers().length || 0,
              };
            });
            setRoomsInCurrentSpace(roomsData);
            
            setIsLoading(false);
            return;
          }
        }
        
        // Room is not in any Space
        setBreadcrumb(null);
        setIsLoading(false);
      } catch (error) {
        console.error('Failed to load Space breadcrumb:', error);
        setBreadcrumb(null);
        setIsLoading(false);
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

  // Handle click outside to close dropdowns
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
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, []);

  // Don't render if room is not in a Space
  if (!breadcrumb) {
    return null;
  }

  const handleSpaceSelect = (spaceId: string) => {
    setShowSpaceDropdown(false);
    if (onSpaceClick) {
      onSpaceClick(spaceId);
    }
  };

  const handleRoomSelect = (selectedRoomId: string) => {
    setShowRoomDropdown(false);
    if (onRoomClick) {
      onRoomClick(selectedRoomId);
    }
  };

  return (
    <motion.nav
      initial={{ opacity: 0, y: -10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.2 }}
      aria-label="Space breadcrumb"
      className={`relative flex items-center gap-2 px-4 py-2 bg-background-muted border-b border-border-default ${className}`}
    >
      {/* Space Dropdown */}
      <div className="relative" ref={spaceDropdownRef}>
        <button
          onClick={() => setShowSpaceDropdown(!showSpaceDropdown)}
          className="flex items-center gap-2 text-text-muted hover:text-text-default transition-colors px-2 py-1 rounded-lg hover:bg-background-default"
          aria-label="Select space"
          aria-expanded={showSpaceDropdown}
        >
          <div className="w-5 h-5 bg-background-accent/20 rounded flex items-center justify-center">
            <Layers className="w-3 h-3 text-text-on-accent" />
          </div>
          <span className="text-sm font-medium">{breadcrumb.spaceName}</span>
          <ChevronDown className={`w-3 h-3 transition-transform ${showSpaceDropdown ? 'rotate-180' : ''}`} />
        </button>

        {/* Space Dropdown Menu */}
        <AnimatePresence>
          {showSpaceDropdown && (
            <motion.div
              initial={{ opacity: 0, y: -10, scale: 0.95 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: -10, scale: 0.95 }}
              transition={{ duration: 0.15 }}
              className="absolute top-full left-0 mt-1 w-64 bg-background-default rounded-lg shadow-lg border border-border-default overflow-hidden z-50"
            >
              <div className="max-h-80 overflow-y-auto">
                {allSpaces.length === 0 ? (
                  <div className="px-4 py-3 text-sm text-text-muted">
                    No spaces available
                  </div>
                ) : (
                  allSpaces.map((space) => (
                    <button
                      key={space.roomId}
                      onClick={() => handleSpaceSelect(space.roomId)}
                      className={`w-full px-4 py-3 text-left hover:bg-background-medium transition-colors flex items-center gap-3 ${
                        space.roomId === breadcrumb.spaceId ? 'bg-background-accent/10' : ''
                      }`}
                    >
                      <div className="w-8 h-8 bg-background-accent/20 rounded flex items-center justify-center flex-shrink-0">
                        <Layers className="w-4 h-4 text-text-on-accent" />
                      </div>
                      <div className="flex-1 min-w-0">
                        <div className="text-sm font-medium text-text-default truncate">
                          {space.name}
                        </div>
                        <div className="text-xs text-text-muted flex items-center gap-1">
                          <Users className="w-3 h-3" />
                          <span>{space.memberCount} members</span>
                        </div>
                      </div>
                      {space.roomId === breadcrumb.spaceId && (
                        <div className="w-2 h-2 bg-background-accent rounded-full flex-shrink-0" />
                      )}
                    </button>
                  ))
                )}
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>

      {/* Separator */}
      <ChevronRight className="w-4 h-4 text-text-muted" aria-hidden="true" />

      {/* Room Dropdown */}
      <div className="relative" ref={roomDropdownRef}>
        <button
          onClick={() => setShowRoomDropdown(!showRoomDropdown)}
          className="flex items-center gap-2 text-text-default hover:text-text-default transition-colors px-2 py-1 rounded-lg hover:bg-background-default"
          aria-label="Select room"
          aria-expanded={showRoomDropdown}
          aria-current="page"
        >
          <Hash className="w-4 h-4 text-text-muted" />
          <span className="text-sm">{breadcrumb.roomName}</span>
          <ChevronDown className={`w-3 h-3 transition-transform ${showRoomDropdown ? 'rotate-180' : ''}`} />
        </button>

        {/* Room Dropdown Menu */}
        <AnimatePresence>
          {showRoomDropdown && (
            <motion.div
              initial={{ opacity: 0, y: -10, scale: 0.95 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: -10, scale: 0.95 }}
              transition={{ duration: 0.15 }}
              className="absolute top-full left-0 mt-1 w-72 bg-background-default rounded-lg shadow-lg border border-border-default overflow-hidden z-50"
            >
              <div className="max-h-80 overflow-y-auto">
                {roomsInCurrentSpace.length === 0 ? (
                  <div className="px-4 py-3 text-sm text-text-muted">
                    No rooms in this space
                  </div>
                ) : (
                  roomsInCurrentSpace.map((room) => (
                    <button
                      key={room.roomId}
                      onClick={() => handleRoomSelect(room.roomId)}
                      className={`w-full px-4 py-3 text-left hover:bg-background-medium transition-colors flex items-center gap-3 ${
                        room.roomId === breadcrumb.roomId ? 'bg-background-accent/10' : ''
                      }`}
                    >
                      <div className="w-8 h-8 bg-background-muted rounded flex items-center justify-center flex-shrink-0">
                        <Hash className="w-4 h-4 text-text-muted" />
                      </div>
                      <div className="flex-1 min-w-0">
                        <div className="text-sm font-medium text-text-default truncate">
                          {room.name}
                        </div>
                        {room.topic ? (
                          <div className="text-xs text-text-muted truncate">
                            {room.topic}
                          </div>
                        ) : (
                          <div className="text-xs text-text-muted flex items-center gap-1">
                            <Users className="w-3 h-3" />
                            <span>{room.memberCount} members</span>
                          </div>
                        )}
                      </div>
                      {room.roomId === breadcrumb.roomId && (
                        <div className="w-2 h-2 bg-background-accent rounded-full flex-shrink-0" />
                      )}
                    </button>
                  ))
                )}
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </motion.nav>
  );
};
