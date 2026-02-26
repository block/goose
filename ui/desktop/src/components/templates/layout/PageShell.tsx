import type React from 'react';
import { cn } from '../../../utils';

type PageWidth = 'narrow' | 'default' | 'wide' | 'full';

const WIDTH_MAP: Record<PageWidth, string> = {
  narrow: 'max-w-3xl',
  default: 'max-w-5xl',
  wide: 'max-w-7xl',
  full: '',
};

interface PageShellProps {
  children: React.ReactNode;
  title?: string;
  subtitle?: string;
  actions?: React.ReactNode;
  /** Content rendered between the header and the scrollable body (e.g. tab bar). */
  headerExtra?: React.ReactNode;
  width?: PageWidth;
  className?: string;
  centerContent?: boolean;
  stickyHeader?: boolean;
  bodyProps?: React.HTMLAttributes<HTMLDivElement>;
}

function Header({
  title,
  subtitle,
  actions,
}: Pick<PageShellProps, 'title' | 'subtitle' | 'actions'>) {
  if (!title && !actions) return null;
  return (
    <div className="flex items-start justify-between gap-4 mb-6">
      <div>
        {title && (
          <h1 className="text-2xl font-semibold tracking-tight text-text-default">{title}</h1>
        )}
        {subtitle && <p className="mt-1 text-sm text-text-muted">{subtitle}</p>}
      </div>
      {actions && <div className="flex items-center gap-2 shrink-0">{actions}</div>}
    </div>
  );
}

export function PageShell({
  children,
  title,
  subtitle,
  actions,
  headerExtra,
  width = 'default',
  className,
  centerContent = false,
  stickyHeader = false,
  bodyProps,
}: PageShellProps) {
  const widthClass = WIDTH_MAP[width];

  if (stickyHeader) {
    return (
      <div className={cn('h-full flex flex-col', className)}>
        <div className={cn('mx-auto w-full px-8 pt-6 shrink-0', widthClass)}>
          <Header title={title} subtitle={subtitle} actions={actions} />
          {headerExtra}
        </div>
        <div {...bodyProps} className={cn('flex-1 min-h-0 overflow-y-auto', bodyProps?.className)}>
          <div className={cn('mx-auto w-full px-8 pb-6', widthClass)}>{children}</div>
        </div>
      </div>
    );
  }

  return (
    <div className={cn('h-full overflow-y-auto', className)}>
      <div
        className={cn(
          'mx-auto w-full px-8 py-6',
          widthClass,
          centerContent && 'min-h-full flex flex-col'
        )}
      >
        <Header title={title} subtitle={subtitle} actions={actions} />
        {centerContent ? (
          <div className="flex-1 flex flex-col items-center justify-center">{children}</div>
        ) : (
          children
        )}
      </div>
    </div>
  );
}
