import { ChevronDown, ChevronRight } from 'lucide-react';
import { type ReactNode, useState } from 'react';
import { cn } from '@/utils';

interface TreeItemProps {
  label: string;
  icon?: ReactNode;
  expanded?: boolean;
  defaultExpanded?: boolean;
  onToggle?: () => void;
  childCount?: number;
  badge?: ReactNode;
  children?: ReactNode;
  indent?: number;
  className?: string;
}

export function TreeItem({
  label,
  icon,
  expanded: controlledExpanded,
  defaultExpanded = false,
  onToggle,
  childCount,
  badge,
  children,
  indent = 0,
  className,
}: TreeItemProps) {
  const [internalExpanded, setInternalExpanded] = useState(defaultExpanded);
  const isExpanded = controlledExpanded ?? internalExpanded;
  const hasChildren = !!children;

  const handleToggle = () => {
    if (onToggle) {
      onToggle();
    } else {
      setInternalExpanded((prev) => !prev);
    }
  };

  return (
    <div className={className}>
      <button
        onClick={hasChildren ? handleToggle : undefined}
        className={cn(
          'flex items-center gap-2 w-full px-3 py-2 rounded-md text-left text-sm',
          hasChildren && 'hover:bg-background-muted cursor-pointer transition-colors',
          !hasChildren && 'cursor-default'
        )}
        style={indent > 0 ? { paddingLeft: `${indent * 1.5 + 0.75}rem` } : undefined}
      >
        {hasChildren ? (
          isExpanded ? (
            <ChevronDown className="h-4 w-4 text-text-muted shrink-0" />
          ) : (
            <ChevronRight className="h-4 w-4 text-text-muted shrink-0" />
          )
        ) : (
          <div className="w-4 shrink-0" />
        )}
        {icon && <div className="shrink-0 text-text-muted">{icon}</div>}
        <span className="font-medium text-text-default truncate flex-1">{label}</span>
        {childCount !== undefined && childCount > 0 && (
          <span className="text-xs text-text-muted bg-background-muted rounded-full px-2 py-0.5">
            {childCount}
          </span>
        )}
        {badge}
      </button>
      {hasChildren && isExpanded && (
        <div className="border-l border-border-default ml-5">{children}</div>
      )}
    </div>
  );
}
