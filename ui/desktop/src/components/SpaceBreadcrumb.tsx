import React, { useState, useEffect } from 'react';
import { ChevronRight, Hash } from 'lucide-react';
import { matrixService } from '../services/MatrixService';
import { motion } from 'framer-motion';

interface SpaceBreadcrumbProps {
  roomId: string;
  className?: string;
}

interface BreadcrumbData {
  spaceName: string;
  spaceId: string;
  roomName: string;
  roomId: string;
}

export const SpaceBreadcrumb: React.FC<SpaceBreadcrumbProps> = ({ roomId, className = '' }) => {
  const [breadcrumb, setBreadcrumb] = useState<BreadcrumbData | null>(null);

  useEffect(() => {
    const loadBreadcrumb = () => {
      try {
        // Get all Spaces
        const spaces = matrixService.getSpaces();
        
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
      {/* Space Icon */}
      <div className="flex items-center gap-2 text-text-muted">
        <div className="w-5 h-5 bg-background-accent/20 rounded flex items-center justify-center">
          <Hash className="w-3 h-3 text-text-on-accent" />
        </div>
        <span className="text-sm font-medium">{breadcrumb.spaceName}</span>
      </div>

      {/* Separator */}
      <ChevronRight className="w-4 h-4 text-text-muted" />

      {/* Room Name */}
      <div className="flex items-center gap-2">
        <Hash className="w-4 h-4 text-text-muted" />
        <span className="text-sm text-text-default">{breadcrumb.roomName}</span>
      </div>

      {/* Optional: Add a badge to indicate it's a Space room */}
      <div className="ml-auto">
        <span className="text-xs px-2 py-0.5 rounded-full bg-blue-500/10 text-blue-600 dark:text-blue-400">
          Space Room
        </span>
      </div>
    </motion.div>
  );
};
