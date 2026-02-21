import type * as React from 'react';
import { cn } from '../../../utils';

type ProgressColor = 'default' | 'success' | 'warning' | 'danger' | 'info';

const colorStyles: Record<ProgressColor, string> = {
  default: 'bg-accent',
  success: 'bg-text-success',
  warning: 'bg-text-warning',
  danger: 'bg-text-danger',
  info: 'bg-text-info',
};

interface ProgressProps extends React.ComponentProps<'div'> {
  label?: string;
  value?: number;
  max?: number;
  color?: ProgressColor;
  showValue?: boolean;
}

export function Progress({
  label,
  value = 0,
  max = 100,
  color = 'default',
  showValue = true,
  className,
  ...props
}: ProgressProps) {
  const pct = Math.min(100, Math.max(0, (value / max) * 100));

  return (
    <div className={cn('space-y-1.5', className)} {...props}>
      {(label || showValue) && (
        <div className="flex justify-between text-sm">
          {label && <span className="text-text-default">{label}</span>}
          {showValue && <span className="text-text-muted">{Math.round(pct)}%</span>}
        </div>
      )}
      <div className="h-2 bg-background-muted rounded-full overflow-hidden">
        <div
          className={cn('h-full rounded-full transition-all', colorStyles[color])}
          style={{ width: `${pct}%` }}
        />
      </div>
    </div>
  );
}
