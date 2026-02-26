import type { ReactNode } from 'react';
import { cn } from '../../../utils';

type DataCardVariant = 'default' | 'interactive' | 'stat';

interface DataCardProps {
  children: ReactNode;
  onClick?: () => void;
  className?: string;
  variant?: DataCardVariant;
}

const variantClasses: Record<DataCardVariant, string> = {
  default: '',
  interactive: 'hover:bg-background-muted cursor-pointer transition-colors',
  stat: 'text-center',
};

export function DataCard({ children, onClick, className, variant = 'default' }: DataCardProps) {
  const Component = onClick ? 'button' : 'div';
  return (
    <Component
      onClick={onClick}
      className={cn(
        'bg-background-default border border-border-default rounded-lg p-4',
        variantClasses[variant],
        onClick && variantClasses.interactive,
        className
      )}
    >
      {children}
    </Component>
  );
}
