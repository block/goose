import { useMemo } from 'react';
import { cn } from '@/utils';
import { Skeleton } from '@/components/atoms/skeleton';

interface LoadingStateProps {
  variant?: 'spinner' | 'pulse' | 'skeleton';
  lines?: number;
  className?: string;
}

export function LoadingState({
  variant = 'skeleton',
  lines = 3,
  className,
}: LoadingStateProps) {
  const lineItems = useMemo(
    () =>
      Array.from({ length: lines }, (_, n) => ({
        key: `line-${n + 1}`,
        width: `${85 - n * 15}%`,
      })),
    [lines]
  );

  if (variant === 'spinner') {
    return (
      <div className={cn('flex items-center justify-center py-8', className)}>
        <div className="animate-spin rounded-full h-6 w-6 border-b-2 border-text-default" />
      </div>
    );
  }

  if (variant === 'pulse') {
    return (
      <div className={cn('space-y-3 py-4', className)}>
        {lineItems.map((item) => (
          <div
            key={item.key}
            className="h-4 bg-background-muted animate-pulse rounded"
            style={{ width: item.width }}
          />
        ))}
      </div>
    );
  }

  // Default: skeleton
  return (
    <div className={cn('space-y-3 py-4', className)}>
      {lineItems.map((item) => (
        <Skeleton key={item.key} className="h-12 w-full" />
      ))}
    </div>
  );
}
