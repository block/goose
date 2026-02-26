import type { ReactNode } from 'react';
import { cn } from '@/utils';

type ListItemStatus = 'active' | 'inactive' | 'error' | 'loading';

interface ListItemProps {
  title: string;
  description?: string;
  icon?: ReactNode;
  status?: ListItemStatus;
  actions?: ReactNode;
  onClick?: () => void;
  indent?: number;
  className?: string;
}

const statusDot: Record<ListItemStatus, string> = {
  active: 'bg-text-success',
  inactive: 'bg-text-muted',
  error: 'bg-text-danger',
  loading: 'bg-text-warning animate-pulse',
};

export function ListItem({
  title,
  description,
  icon,
  status,
  actions,
  onClick,
  indent = 0,
  className,
}: ListItemProps) {
  const Component = onClick ? 'button' : 'div';
  return (
    <Component
      onClick={onClick}
      className={cn(
        'flex items-center gap-3 px-3 py-2.5 rounded-md w-full text-left',
        onClick && 'hover:bg-background-muted cursor-pointer transition-colors',
        className
      )}
      style={indent > 0 ? { paddingLeft: `${indent * 1.5 + 0.75}rem` } : undefined}
    >
      {icon && <div className="shrink-0 text-text-muted">{icon}</div>}
      {status && <div className={cn('h-2 w-2 rounded-full shrink-0', statusDot[status])} />}
      <div className="flex-1 min-w-0">
        <div className="text-sm font-medium text-text-default truncate">{title}</div>
        {description && <div className="text-xs text-text-muted truncate">{description}</div>}
      </div>
      {actions && <div className="shrink-0 flex items-center gap-1">{actions}</div>}
    </Component>
  );
}
