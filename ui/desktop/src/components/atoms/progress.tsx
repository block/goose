import type * as React from 'react';
import { useId } from 'react';
import { cn } from '@/utils';

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
  indeterminate?: boolean;
}

export function Progress({
  label,
  value,
  max = 100,
  color = 'default',
  showValue = true,
  indeterminate = false,
  className,
  ...props
}: ProgressProps) {
  const labelId = useId();

  if (indeterminate) {
    return (
      <div className={cn('space-y-1.5', className)} {...props}>
        {label && (
          <div className="flex justify-between text-sm">
            <span id={labelId} className="text-text-default">
              {label}
            </span>
          </div>
        )}
        <div
          className="h-2 bg-background-muted rounded-full overflow-hidden"
          role="progressbar"
          {...(label ? { 'aria-labelledby': labelId } : { 'aria-label': 'Progress' })}
        >
          <div
            className={cn('h-full w-1/3 rounded-full progress-indeterminate', colorStyles[color])}
          />
        </div>
      </div>
    );
  }

  const resolvedValue = value ?? 0;
  const safeMax = max > 0 ? max : 100;
  const safeValue = Math.min(safeMax, Math.max(0, resolvedValue));
  const pct = Math.min(100, Math.max(0, (safeValue / safeMax) * 100));

  return (
    <div className={cn('space-y-1.5', className)} {...props}>
      {(label || showValue) && (
        <div className="flex justify-between text-sm">
          {label && (
            <span id={labelId} className="text-text-default">
              {label}
            </span>
          )}
          {showValue && <span className="text-text-muted">{Math.round(pct)}%</span>}
        </div>
      )}
      <div
        className="h-2 bg-background-muted rounded-full overflow-hidden"
        role="progressbar"
        {...(label ? { 'aria-labelledby': labelId } : { 'aria-label': 'Progress' })}
        aria-valuemin={0}
        aria-valuemax={safeMax}
        aria-valuenow={safeValue}
        aria-valuetext={`${Math.round(pct)}%`}
      >
        <div
          className={cn('h-full rounded-full transition-all', colorStyles[color])}
          style={{ width: `${pct}%` }}
        />
      </div>
    </div>
  );
}
