import type * as React from 'react';
import { cn } from '../../../utils';

type AlertSeverity = 'info' | 'success' | 'warning' | 'error';

const severityStyles: Record<AlertSeverity, string> = {
  info: 'border-border-default bg-background-info text-text-info',
  success: 'border-border-default bg-background-success-muted text-text-success',
  warning: 'border-border-default bg-background-warning-muted text-text-warning',
  error: 'border-border-default bg-background-danger-muted text-text-danger',
};

const severityIcons: Record<AlertSeverity, string> = {
  info: 'ℹ️',
  success: '✅',
  warning: '⚠️',
  error: '❌',
};

interface AlertProps extends React.ComponentProps<'div'> {
  severity?: AlertSeverity;
  title?: string;
  message?: string;
}

export function Alert({ severity = 'info', title, message, className, ...props }: AlertProps) {
  return (
    <div
      className={cn('flex gap-3 rounded-lg border p-3', severityStyles[severity], className)}
      role="alert"
      {...props}
    >
      <span className="text-lg shrink-0">{severityIcons[severity]}</span>
      <div className="space-y-0.5">
        {title && <p className="text-sm font-medium">{title}</p>}
        {message && <p className="text-sm opacity-80">{message}</p>}
      </div>
    </div>
  );
}
