import type * as React from 'react';
import { cn } from '../../../utils';

interface StatusDotProps extends React.ComponentProps<'span'> {
  status: 'active' | 'completed' | 'idle';
  size?: 'sm' | 'md';
}

const statusColors: Record<StatusDotProps['status'], string> = {
  active: 'bg-blue-400 animate-pulse',
  completed: 'bg-emerald-400',
  idle: 'bg-text-muted/30',
};

const dotSizes: Record<'sm' | 'md', string> = {
  sm: 'w-1.5 h-1.5',
  md: 'w-2 h-2',
};

/** A small colored dot indicating activity status. */
export function StatusDot({ status, size = 'sm', className, ...props }: StatusDotProps) {
  return (
    <span
      className={cn(
        'inline-block rounded-full shrink-0',
        statusColors[status],
        dotSizes[size],
        className
      )}
      {...props}
    />
  );
}
