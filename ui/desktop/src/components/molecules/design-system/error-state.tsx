import { AlertTriangle, RefreshCw } from 'lucide-react';
import { cn } from '../../../utils';

interface ErrorStateProps {
  title?: string;
  message?: string;
  onRetry?: () => void;
  className?: string;
}

export function ErrorState({
  title = 'Something went wrong',
  message,
  onRetry,
  className,
}: ErrorStateProps) {
  return (
    <div
      className={cn('flex flex-col items-center justify-center py-12 px-4 text-center', className)}
    >
      <div className="h-12 w-12 rounded-full bg-background-danger-muted flex items-center justify-center mb-4">
        <AlertTriangle className="h-6 w-6 text-text-danger" />
      </div>
      <h3 className="text-sm font-medium text-text-default mb-1">{title}</h3>
      {message && <p className="text-sm text-text-muted max-w-sm">{message}</p>}
      {onRetry && (
        <button
          onClick={onRetry}
          className="mt-4 flex items-center gap-2 px-3 py-1.5 text-sm font-medium text-text-default bg-background-muted hover:bg-background-default border border-border-default rounded-md transition-colors"
        >
          <RefreshCw className="h-3.5 w-3.5" />
          Try again
        </button>
      )}
    </div>
  );
}
