import React from 'react';
import { cn } from '../../utils';

interface PillProps {
  children: React.ReactNode;
  className?: string;
  variant?: 'default' | 'glass' | 'solid' | 'gradient' | 'glow';
  size?: 'xs' | 'sm' | 'md' | 'lg';
  color?: 'blue' | 'green' | 'amber' | 'red' | 'purple' | 'slate';
  onClick?: () => void;
  disabled?: boolean;
  animated?: boolean;
}

export function Pill({
  children,
  className,
  variant = 'glass',
  size = 'md',
  color = 'blue',
  onClick,
  disabled = false,
  animated = false,
}: PillProps) {
  const baseStyles =
    'inline-flex items-center justify-center rounded-full transition-all duration-300 ease-out font-medium';

  const variants = {
    default: 'bg-background border border-border hover:bg-muted/50',
    glass:
      'bg-white/10 dark:bg-black/10 backdrop-blur-xl border border-white/20 dark:border-white/10 shadow-lg shadow-black/5 dark:shadow-black/20 hover:bg-white/15 dark:hover:bg-black/15 hover:shadow-xl',
    solid: 'bg-background border border-border shadow-md hover:shadow-lg hover:scale-105',
    gradient: 'bg-gradient-to-r shadow-lg hover:shadow-xl hover:scale-105 border-0',
    glow: 'shadow-lg hover:shadow-xl hover:scale-105 border-0',
  };

  const colors = {
    blue: {
      gradient: 'from-blue-500 to-blue-600 hover:from-blue-600 hover:to-blue-700 text-white',
      glow: 'bg-blue-500 hover:bg-blue-600 text-white shadow-blue-500/25 hover:shadow-blue-500/40',
      glass: 'text-blue-700 dark:text-blue-300 hover:text-blue-800 dark:hover:text-blue-200',
    },
    green: {
      gradient: 'from-green-500 to-green-600 hover:from-green-600 hover:to-green-700 text-white',
      glow: 'bg-green-500 hover:bg-green-600 text-white shadow-green-500/25 hover:shadow-green-500/40',
      glass: 'text-green-700 dark:text-green-300 hover:text-green-800 dark:hover:text-green-200',
    },
    amber: {
      gradient: 'from-amber-500 to-amber-600 hover:from-amber-600 hover:to-amber-700 text-white',
      glow: 'bg-amber-500 hover:bg-amber-600 text-white shadow-amber-500/25 hover:shadow-amber-500/40',
      glass: 'text-amber-700 dark:text-amber-300 hover:text-amber-800 dark:hover:text-amber-200',
    },
    red: {
      gradient: 'from-red-500 to-red-600 hover:from-red-600 hover:to-red-700 text-white',
      glow: 'bg-red-500 hover:bg-red-600 text-white shadow-red-500/25 hover:shadow-red-500/40',
      glass: 'text-red-700 dark:text-red-300 hover:text-red-800 dark:hover:text-red-200',
    },
    purple: {
      gradient:
        'from-purple-500 to-purple-600 hover:from-purple-600 hover:to-purple-700 text-white',
      glow: 'bg-purple-500 hover:bg-purple-600 text-white shadow-purple-500/25 hover:shadow-purple-500/40',
      glass:
        'text-purple-700 dark:text-purple-300 hover:text-purple-800 dark:hover:text-purple-200',
    },
    slate: {
      gradient: 'from-slate-500 to-slate-600 hover:from-slate-600 hover:to-slate-700 text-white',
      glow: 'bg-slate-500 hover:bg-slate-600 text-white shadow-slate-500/25 hover:shadow-slate-500/40',
      glass: 'text-slate-700 dark:text-slate-300 hover:text-slate-800 dark:hover:text-slate-200',
    },
  };

  const sizes = {
    xs: 'px-2 py-1 text-xs gap-1',
    sm: 'px-3 py-1.5 text-sm gap-1.5',
    md: 'px-4 py-2 text-sm gap-2',
    lg: 'px-6 py-3 text-base gap-3',
  };

  const animatedStyles = animated ? 'animate-pulse' : '';

  const disabledStyles = disabled
    ? 'opacity-50 cursor-not-allowed pointer-events-none'
    : onClick
      ? 'cursor-pointer hover:scale-105 active:scale-95'
      : '';

  const colorStyles =
    variant === 'gradient'
      ? colors[color].gradient
      : variant === 'glow'
        ? colors[color].glow
        : variant === 'glass'
          ? colors[color].glass
          : '';

  return (
    <div
      className={cn(
        baseStyles,
        variants[variant],
        sizes[size],
        colorStyles,
        disabledStyles,
        animatedStyles,
        className
      )}
      onClick={onClick && !disabled ? onClick : undefined}
    >
      {children}
    </div>
  );
}

export default Pill;
