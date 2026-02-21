import type * as React from 'react';
import { cn } from '../../../utils';

type AlertSeverity = 'info' | 'success' | 'warning' | 'error';

const severityStyles: Record<AlertSeverity, string> = {
  info: 'border-blue-500/30 bg-blue-500/5 text-blue-600 dark:text-blue-400',
  success: 'border-green-500/30 bg-green-500/5 text-green-600 dark:text-green-400',
  warning: 'border-amber-500/30 bg-amber-500/5 text-amber-600 dark:text-amber-400',
  error: 'border-red-500/30 bg-red-500/5 text-red-600 dark:text-red-400',
};

const severityIcons: Record<AlertSeverity, string> = {
  info: 'ℹ️',
  success: '✅',
  warning: '⚠️',
  error: '❌',
};

interface AlertProps extends React.ComponentProps<'div'> {
  title?: string;
  message?: string;
  severity?: AlertSeverity;
}

export function Alert({ title, message, severity = 'info', className, ...props }: AlertProps) {
  return (
    <div className={cn('rounded-lg border p-4', severityStyles[severity], className)} {...props}>
      <div className="flex gap-3">
        <span className="text-base shrink-0">{severityIcons[severity]}</span>
        <div className="space-y-1">
          {title && <div className="font-medium text-sm">{title}</div>}
          {message && <div className="text-sm opacity-90">{message}</div>}
        </div>
      </div>
    </div>
  );
}
