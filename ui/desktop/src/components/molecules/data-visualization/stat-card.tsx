import type { LucideIcon } from 'lucide-react';
import { cn } from '@/utils';

interface StatCardProps {
  label: string;
  value: string | number;
  icon?: LucideIcon;
  trend?: { value: number; direction: 'up' | 'down' };
  variant?: 'default' | 'success' | 'warning' | 'danger';
  className?: string;
}

const variantTextColor: Record<string, string> = {
  default: 'text-text-default',
  success: 'text-text-success',
  warning: 'text-text-warning',
  danger: 'text-text-danger',
};

export function StatCard({
  label,
  value,
  icon: Icon,
  trend,
  variant = 'default',
  className,
}: StatCardProps) {
  const displayValue =
    typeof value === 'string' && value.trim().length === 0
      ? '—'
      : typeof value === 'number'
        ? value.toLocaleString()
        : value;

  const ariaLabel = `${label}: ${displayValue}`;

  return (
    <section
      aria-label={ariaLabel}
      className={cn(
        'bg-background-default border border-border-default rounded-lg p-4 flex flex-col gap-2',
        className
      )}
    >
      <div className="flex items-center justify-between">
        <span className="text-xs font-medium text-text-muted uppercase tracking-wider">
          {label}
        </span>
        {Icon && <Icon className="h-4 w-4 text-text-muted" />}
      </div>
      <div className="flex items-baseline gap-2">
        <span className={cn('text-2xl font-semibold', variantTextColor[variant])}>
          {displayValue}
        </span>
        {trend && (
          <span
            className={cn(
              'text-xs font-medium',
              trend.direction === 'up' ? 'text-text-success' : 'text-text-danger'
            )}
          >
            {trend.direction === 'up' ? '↑' : '↓'} {Math.abs(trend.value)}%
          </span>
        )}
      </div>
    </section>
  );
}
