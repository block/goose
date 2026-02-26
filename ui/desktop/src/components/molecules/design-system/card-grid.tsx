import type React from 'react';
import { cn } from '../../../utils';

export type CardGridSize = 'xs' | 's' | 'm' | 'l' | 'wl';

export interface CardGridProps {
  children?: React.ReactNode;
  /**
   * Desired column count. In chat surfaces we generally clamp to 2.
   */
  columns?: 1 | 2;
  gap?: 'sm' | 'md' | 'lg';
  /**
   * Per-child sizing tokens. Keys should match the element children keys.
   */
  sizes?: Record<string, CardGridSize>;
  className?: string;
  /**
   * @json-render provides the underlying spec element; we use it to map children
   * to their stable keys for sizing.
   */
  element?: {
    children?: string[];
  };
}

const GAP_CLASS: Record<NonNullable<CardGridProps['gap']>, string> = {
  sm: 'gap-2',
  md: 'gap-3',
  lg: 'gap-4',
};

function isWide(size: CardGridSize | undefined): boolean {
  return size === 'l' || size === 'wl';
}

export function CardGrid({
  children,
  columns = 2,
  gap = 'md',
  sizes = {},
  className,
  element,
}: CardGridProps) {
  const childArray = Array.isArray(children) ? children : [children];
  const keys = element?.children ?? [];

  const effectiveColumns: 1 | 2 = columns === 1 ? 1 : 2;

  return (
    <div
      className={cn(
        'grid w-full min-w-0',
        effectiveColumns === 2 ? 'grid-cols-2' : 'grid-cols-1',
        GAP_CLASS[gap],
        className
      )}
    >
      {childArray.map((child, index) => {
        if (!child) return null;

        const key = keys[index];
        const size = key ? sizes[key] : undefined;

        const colSpan = effectiveColumns === 2 && isWide(size) ? 'col-span-2' : undefined;

        return (
          <div key={key ?? index} className={cn('min-w-0', colSpan)}>
            {child}
          </div>
        );
      })}
    </div>
  );
}
