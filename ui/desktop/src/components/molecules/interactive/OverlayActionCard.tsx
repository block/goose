import * as React from 'react';
import { cn } from '@/utils';

type OverlayActionCardProps = {
  ariaLabel: string;
  onActivate: () => void;
  /** Root container classes (padding, rounded, etc). */
  className?: string;
  /** Used for both the root container and the overlay button (default: rounded-xl). */
  radiusClassName?: string;
  /** Extra classes for the overlay <button>. */
  overlayClassName?: string;
  children: React.ReactNode;
};

function OverlayActionCardActions({ className, children }: { className?: string; children: React.ReactNode }) {
  return <div className={cn('pointer-events-auto', className)}>{children}</div>;
}

/**
 * Pattern:
 * - Full-card primary action via an overlay <button>
 * - Non-interactive content should live under pointer-events-none
 * - Nested controls must be wrapped with <OverlayActionCard.Actions>
 */
export const OverlayActionCard = Object.assign(
  React.forwardRef<HTMLDivElement, OverlayActionCardProps>(function OverlayActionCard(
    { ariaLabel, onActivate, className, radiusClassName = 'rounded-xl', overlayClassName, children },
    ref
  ) {
    return (
      <div
        ref={ref}
        className={cn(
          'group relative bg-background-default border border-border-default hover:border-border-accent hover:shadow-lg transition-all',
          radiusClassName,
          className
        )}
      >
        <button
          type="button"
          className={cn(
            'absolute inset-0 focus:outline-none focus:ring-2 focus:ring-border-accent',
            radiusClassName,
            overlayClassName
          )}
          aria-label={ariaLabel}
          onClick={onActivate}
        />

        <div className="relative z-10 pointer-events-none">{children}</div>
      </div>
    );
  }),
  { Actions: OverlayActionCardActions }
);
