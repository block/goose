import type * as React from 'react';
import { cn } from '../../../utils';

type BadgeVariant = 'default' | 'secondary' | 'accent' | 'muted' | 'outline';
type BadgeSize = 'sm' | 'md';

const variantStyles: Record<BadgeVariant, string> = {
  default: 'bg-blue-500/10 text-blue-500 border-blue-500/20',
  secondary: 'bg-text-muted/10 text-text-muted border-text-muted/20',
  accent: 'bg-amber-500/10 text-amber-500 border-amber-500/20',
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
