import type * as React from 'react';
import { cn } from '../../utils';

type BadgeVariant = 'default' | 'secondary' | 'accent' | 'muted' | 'outline';
type BadgeSize = 'sm' | 'md';

const variantStyles: Record<BadgeVariant, string> = {
  default: 'bg-background-accent/10 text-text-accent border-border-accent/20',
  secondary: 'bg-text-muted/10 text-text-muted border-text-muted/20',
  accent: 'bg-background-warning-muted text-text-warning border-border-default',
  muted: 'bg-background-muted text-text-muted border-transparent',
  outline: 'bg-transparent text-text-muted border-border-default',
};

const sizeStyles: Record<BadgeSize, string> = {
  sm: 'text-[10px] px-1.5 py-0',
  md: 'text-xs px-2 py-0.5',
};

interface BadgeProps extends React.ComponentProps<'span'> {
  variant?: BadgeVariant;
  size?: BadgeSize;
}

export function Badge({ variant = 'default', size = 'sm', className, ...props }: BadgeProps) {
  return (
    <span
      className={cn(
        'inline-flex items-center rounded-md border font-medium leading-normal whitespace-nowrap',
        variantStyles[variant],
        sizeStyles[size],
        className
      )}
      {...props}
    />
  );
}
