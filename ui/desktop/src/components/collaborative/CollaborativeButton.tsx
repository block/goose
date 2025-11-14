import React, { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Users, UserPlus, Loader2, AlertCircle } from 'lucide-react';
import { Tooltip, TooltipContent, TooltipTrigger } from '../ui/Tooltip';
import { Button } from '../ui/button';
import { useMatrix } from '../../contexts/MatrixContext';

interface CollaborativeButtonProps {
  isCollaborative: boolean;
  isCreatingSession: boolean;
  canCreateSession: boolean;
  error?: string | null;
  onStartSession: () => void;
  onShowSession: () => void;
  shouldShowIconOnly?: boolean;
  className?: string;
}

const CollaborativeButton: React.FC<CollaborativeButtonProps> = ({
  isCollaborative,
  isCreatingSession,
  canCreateSession,
  error,
  onStartSession,
  onShowSession,
  shouldShowIconOnly = false,
  className = '',
}) => {
  const { isConnected, friends } = useMatrix();
  const [showError, setShowError] = useState(false);

  const handleClick = () => {
    if (error) {
      setShowError(true);
      setTimeout(() => setShowError(false), 3000);
      return;
    }

    if (isCollaborative) {
      onShowSession();
    } else {
      onStartSession();
    }
  };

  const getButtonContent = () => {
    if (isCreatingSession) {
      return (
        <>
          <Loader2 className={`w-4 h-4 animate-spin ${shouldShowIconOnly ? '' : 'mr-1'}`} />
          {!shouldShowIconOnly && <span className="text-xs">Creating...</span>}
        </>
      );
    }

    if (isCollaborative) {
      return (
        <>
          <Users className={`w-4 h-4 text-blue-500 ${shouldShowIconOnly ? '' : 'mr-1'}`} />
          {!shouldShowIconOnly && <span className="text-xs text-blue-500">Collaborative</span>}
        </>
      );
    }

    return (
      <>
        <UserPlus className={`w-4 h-4 ${shouldShowIconOnly ? '' : 'mr-1'}`} />
        {!shouldShowIconOnly && <span className="text-xs">Invite Friend</span>}
      </>
    );
  };

  const getTooltipContent = () => {
    if (!isConnected) {
      return 'Connect to Matrix to enable collaborative sessions';
    }

    if (friends.length === 0) {
      return 'Add friends in the Peers page to start collaborative sessions';
    }

    if (error) {
      return `Error: ${error}`;
    }

    if (isCreatingSession) {
      return 'Creating collaborative session...';
    }

    if (isCollaborative) {
      return 'View collaborative session details';
    }

    return 'Start a collaborative AI session with friends';
  };

  const isDisabled = !canCreateSession || isCreatingSession || (!isConnected && !isCollaborative);

  return (
    <div className={`relative ${className}`}>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            type="button"
            onClick={handleClick}
            disabled={isDisabled}
            variant="ghost"
            size="sm"
            className={`flex items-center transition-colors !px-0 ${
              error 
                ? 'text-red-500 hover:text-red-600' 
                : isCollaborative 
                ? 'text-blue-500 hover:text-blue-600' 
                : 'text-text-default/70 hover:text-text-default'
            } ${isDisabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'}`}
          >
            {getButtonContent()}
          </Button>
        </TooltipTrigger>
        <TooltipContent>
          {getTooltipContent()}
        </TooltipContent>
      </Tooltip>

      {/* Error indicator */}
      <AnimatePresence>
        {error && (
          <motion.div
            initial={{ scale: 0 }}
            animate={{ scale: 1 }}
            exit={{ scale: 0 }}
            className="absolute -top-1 -right-1 w-3 h-3"
          >
            <div className="w-full h-full bg-red-500 rounded-full flex items-center justify-center">
              <AlertCircle className="w-2 h-2 text-white" />
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Error popup */}
      <AnimatePresence>
        {showError && error && (
          <motion.div
            initial={{ opacity: 0, y: 10, scale: 0.95 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 10, scale: 0.95 }}
            className="absolute bottom-full mb-2 left-1/2 transform -translate-x-1/2 z-50"
          >
            <div className="bg-red-500 text-white px-3 py-2 rounded-lg text-xs max-w-xs">
              <div className="flex items-center gap-2">
                <AlertCircle className="w-3 h-3 flex-shrink-0" />
                <span>{error}</span>
              </div>
              {/* Arrow pointing down */}
              <div className="absolute top-full left-1/2 transform -translate-x-1/2">
                <div className="border-4 border-transparent border-t-red-500"></div>
              </div>
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* Collaborative session indicator dot */}
      {isCollaborative && (
        <motion.div
          initial={{ scale: 0 }}
          animate={{ scale: 1 }}
          className="absolute -top-1 -right-1 w-3 h-3"
        >
          <div className="w-full h-full bg-blue-500 rounded-full animate-pulse"></div>
        </motion.div>
      )}
    </div>
  );
};

export default CollaborativeButton;
