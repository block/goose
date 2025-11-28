import { Loader2, AlertCircle } from 'lucide-react';

interface SessionIndicatorsProps {
  isStreaming?: boolean;
  hasUnread?: boolean;
  hasError?: boolean;
  className?: string;
}

/**
 * Visual indicators for session status
 * - Error: red alert icon (highest priority)
 * - Streaming: animated spinner
 * - Has activity: green dot (shown after streaming completes until viewed)
 */
export function SessionIndicators({
  isStreaming,
  hasUnread,
  hasError,
  className = '',
}: SessionIndicatorsProps) {
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
        <Loader2 className="w-3.5 h-3.5 text-blue-500 animate-spin" aria-label="Streaming" />
      )}
      {hasUnread && !isStreaming && !hasError && (
        <div className="w-2 h-2 bg-green-500 rounded-full" aria-label="Has new activity" />
      )}
    </div>
  );
}
