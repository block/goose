import React from 'react';
import { cn } from '../../lib/utils';

interface PillProps {
  children: React.ReactNode;
  className?: string;
  variant?: 'default' | 'glass' | 'solid';
  size?: 'sm' | 'md' | 'lg';
  onClick?: () => void;
  disabled?: boolean;
}

export function Pill({
  children,
  className,
  variant = 'glass',
  size = 'md',
  onClick,
  disabled = false,
}: PillProps) {
  const baseStyles = 'inline-flex items-center justify-center rounded-full transition-all duration-200 ease-out';
  
  const variants = {
    default: 'bg-background-muted border border-borderSubtle hover:bg-background-default',
    glass: 'bg-white/10 dark:bg-black/10 backdrop-blur-2xl border border-white/20 dark:border-white/10 shadow-lg shadow-black/10 dark:shadow-black/30 hover:bg-white/15 dark:hover:bg-black/15',
    solid: 'bg-background-default border border-borderSubtle shadow-md hover:shadow-lg',
  };

  const sizes = {
    sm: 'px-3 py-1.5 text-sm gap-1.5',
    md: 'px-4 py-2 text-sm gap-2',
    lg: 'px-6 py-3 text-base gap-3',
  };

  const disabledStyles = disabled 
    ? 'opacity-50 cursor-not-allowed pointer-events-none' 
    : onClick 
      ? 'cursor-pointer' 
      : '';

  return (
    <div
      className={cn(
        baseStyles,
        variants[variant],
        sizes[size],
        disabledStyles,
        className
      )}
      onClick={onClick && !disabled ? onClick : undefined}
    >
      {children}
    </div>
  );
}

export default Pill;
