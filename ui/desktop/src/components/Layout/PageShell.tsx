import React from 'react';
import { cn } from '../../utils';

type PageWidth = 'narrow' | 'default' | 'wide' | 'full';

const WIDTH_MAP: Record<PageWidth, string> = {
  narrow: 'max-w-3xl',
  default: 'max-w-5xl',
  wide: 'max-w-7xl',
  full: '',
};

interface PageShellProps {
  children: React.ReactNode;
  /** Page title shown at the top */
  title?: string;
  /** Optional subtitle below the title */
  subtitle?: string;
  /** Right-aligned actions next to the title */
  actions?: React.ReactNode;
  /** Max width of the content area */
  width?: PageWidth;
  /** Additional className on the outer container */
  className?: string;
  /** Whether to center content vertically (for empty/welcome states) */
  centerContent?: boolean;
}

export function PageShell({
  children,
  title,
  subtitle,
  actions,
  width = 'default',
  className,
  centerContent = false,
}: PageShellProps) {
  return (
    <div className={cn('h-full overflow-y-auto', className)}>
      <div
        className={cn(
          'mx-auto w-full px-8 py-6',
          WIDTH_MAP[width],
          centerContent && 'min-h-full flex flex-col'
        )}
      >
        {(title || actions) && (
          <div className="flex items-start justify-between gap-4 mb-6">
            <div>
              {title && (
                <h1 className="text-2xl font-semibold tracking-tight text-text-default">
                  {title}
                </h1>
              )}
              {subtitle && (
                <p className="mt-1 text-sm text-text-muted">{subtitle}</p>
              )}
            </div>
            {actions && <div className="flex items-center gap-2 shrink-0">{actions}</div>}
          </div>
        )}
        {centerContent ? (
          <div className="flex-1 flex flex-col items-center justify-center">
            {children}
          </div>
        ) : (
          children
        )}
      </div>
    </div>
  );
}
