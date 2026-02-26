import { cn } from '../../../utils';
import { Skeleton } from '../../atoms/skeleton';

interface LoadingStateProps {
  variant?: 'spinner' | 'skeleton' | 'pulse';
  lines?: number;
  className?: string;
}

export function LoadingState({ variant = 'skeleton', lines = 3, className }: LoadingStateProps) {
  if (variant === 'spinner') {
    return (
      <div className={cn('flex items-center justify-center py-12', className)}>
        <div className="h-8 w-8 animate-spin rounded-full border-2 border-border-default border-t-border-accent" />
      </div>
    );
  }

  if (variant === 'pulse') {
    return (
      <div className={cn('space-y-3 py-4', className)}>
        {Array.from({ length: lines }).map((_, i) => (
          <div
            key={i}
            className="h-4 bg-background-muted animate-pulse rounded"
            style={{ width: `${85 - i * 15}%` }}
          />
        ))}
      </div>
    );
  }

  return (
    <div className={cn('space-y-3 py-4', className)}>
      {Array.from({ length: lines }).map((_, i) => (
        <Skeleton key={i} className="h-12 w-full" />
      ))}
    </div>
  );
}
