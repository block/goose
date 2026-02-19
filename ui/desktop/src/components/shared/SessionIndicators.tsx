import { AlertCircle, Loader2 } from 'lucide-react';
import React from 'react';

interface SessionIndicatorsProps {
  isStreaming: boolean;
  hasUnread: boolean;
  hasError: boolean;
}

/**
 * Visual indicators for session status (priority order: error > streaming > unread)
 */
export const SessionIndicators = React.memo<SessionIndicatorsProps>(
  ({ isStreaming, hasUnread, hasError }) => {
    if (hasError) {
      return (
        <div className="flex items-center gap-1">
          <AlertCircle
            className="w-3.5 h-3.5 text-red-500"
            aria-label="Session encountered an error"
          />
        </div>
      );
    }

    if (isStreaming) {
      return (
        <div className="flex items-center gap-1">
          <Loader2 className="w-3 h-3 text-blue-500 animate-spin" aria-label="Streaming" />
        </div>
      );
    }

    if (hasUnread) {
      return (
        <div className="flex items-center gap-1">
          <div className="w-2 h-2 bg-green-500 rounded-full" aria-label="Has new activity" />
        </div>
      );
    }

    return null;
  }
);

SessionIndicators.displayName = 'SessionIndicators';
