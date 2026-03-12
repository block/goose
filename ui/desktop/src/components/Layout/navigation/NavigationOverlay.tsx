import React from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { cn } from '../../../utils';
import { Z_INDEX } from '../constants';

type NavigationPosition = 'top' | 'bottom' | 'left' | 'right';

interface NavigationOverlayProps {
  isOpen: boolean;
  position: NavigationPosition;
  onClose: () => void;
  children: React.ReactNode;
}

export const NavigationOverlay: React.FC<NavigationOverlayProps> = ({
  isOpen,
  position,
  onClose,
  children,
}) => {
  return (
    <AnimatePresence>
      {isOpen && (
        <div className="fixed inset-0" style={{ zIndex: Z_INDEX.OVERLAY }}>
          {/* Backdrop */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            className="absolute inset-0 bg-black/20 backdrop-blur-sm"
            onClick={onClose}
          />

          {/* Scrollable container for navigation panel */}
          <div className="absolute inset-0 overflow-y-auto pointer-events-none">
            <div
              className={cn(
                'min-h-full flex p-4',
                position === 'top' && 'items-start justify-center pt-16',
                position === 'bottom' && 'items-end justify-center pb-8',
                position === 'left' && 'items-center justify-start pl-4',
                position === 'right' && 'items-center justify-end pr-4'
              )}
            >
              <div className="pointer-events-auto">{children}</div>
            </div>
          </div>
        </div>
      )}
    </AnimatePresence>
  );
};
