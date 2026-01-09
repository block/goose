import { AlertCircle } from 'lucide-react';
import React from 'react';
import { OptimizedSpinner } from './ui/optimized-spinner';

interface SessionIndicatorsProps {
  isStreaming?: boolean;
  hasUnread?: boolean;
  hasError?: boolean;
  className?: string;
}

/**
 * Visual indicators for session status
 * - Error: red alert icon (highest priority)
 * - Streaming: animated spinner with GPU acceleration
 * - Has activity: green dot (shown after streaming completes until viewed)
 */
export const SessionIndicators = React.memo<SessionIndicatorsProps>(
  ({ isStreaming, hasUnread, hasError, className = '' }) => {
    if (!isStreaming && !hasUnread && !hasError) {
      return null;
    }

    return (
      <div className={`flex items-center gap-1 ${className}`}>
        {hasError && (
          <AlertCircle
            className="w-3.5 h-3.5 text-red-500"
            aria-label="Session encountered an error"
          />
        )}
        {isStreaming && !hasError && (
          <OptimizedSpinner size="xs" className="text-blue-500" aria-label="Streaming" />
        )}
        {hasUnread && !isStreaming && !hasError && (
          <div
            className="w-2 h-2 bg-green-500 rounded-full"
            aria-label="Has new activity"
            style={{
              transform: 'translateZ(0)',
            }}
          />
        )}
      </div>
    );
  }
);

SessionIndicators.displayName = 'SessionIndicators';
