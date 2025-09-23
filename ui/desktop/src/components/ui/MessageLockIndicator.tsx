import React from 'react';
import { cn } from '../../utils';

interface MessageLockIndicatorProps {
  messageId: string;
  onUnlock: () => void;
  onScrollToBottom: () => void;
  className?: string;
}

/**
 * Visual indicator shown under a locked message
 * Provides user feedback and unlock controls
 */
export function MessageLockIndicator({ 
  messageId, 
  onUnlock, 
  onScrollToBottom, 
  className 
}: MessageLockIndicatorProps) {
  return (
    <div className={cn(
      "flex items-center justify-between",
      "bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800",
      "rounded-lg px-3 py-2 mt-2 mb-4",
      "text-sm text-blue-700 dark:text-blue-300",
      "animate-in slide-in-from-top-2 duration-200",
      className
    )}>
      <div className="flex items-center gap-2">
        <div className="flex items-center gap-1">
          <span className="text-blue-500">🔒</span>
          <span className="font-medium">Scroll locked to this message</span>
        </div>
        <div className="text-xs text-blue-600 dark:text-blue-400 bg-blue-100 dark:bg-blue-800/30 px-2 py-0.5 rounded">
          {messageId.slice(-8)}
        </div>
      </div>
      
      <div className="flex items-center gap-2">
        <button
          onClick={onUnlock}
          className={cn(
            "text-xs px-2 py-1 rounded",
            "bg-blue-100 dark:bg-blue-800/30 text-blue-700 dark:text-blue-300",
            "hover:bg-blue-200 dark:hover:bg-blue-800/50",
            "transition-colors duration-150",
            "border border-blue-200 dark:border-blue-700"
          )}
        >
          Unlock
        </button>
        
        <button
          onClick={onScrollToBottom}
          className={cn(
            "text-xs px-2 py-1 rounded",
            "bg-blue-500 text-white",
            "hover:bg-blue-600",
            "transition-colors duration-150",
            "border border-blue-500"
          )}
        >
          Go to Bottom
        </button>
      </div>
    </div>
  );
}
