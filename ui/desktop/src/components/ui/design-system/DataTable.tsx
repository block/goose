import { useMemo, useState } from 'react';
import type React from 'react';

import ArrowDown from '../../icons/ArrowDown';
import ArrowUp from '../../icons/ArrowUp';
import { cn } from '../../../utils';

export interface DataTableColumn {
  key: string;
  label: string;
  align?: 'left' | 'center' | 'right';
  sortable?: boolean;
}

export interface DataTableProps extends React.ComponentProps<'div'> {
  columns: DataTableColumn[];
  rows: Array<Record<string, unknown>>;
  striped?: boolean;
  hoverable?: boolean;
  caption?: string;
  defaultSortKey?: string;
  defaultSortDirection?: 'asc' | 'desc';
}

const alignClass: Record<string, string> = {
  right: 'text-right',
  center: 'text-center',
  left: 'text-left',
};

function compareValues(a: unknown, b: unknown): number {
  if (typeof a === 'number' && typeof b === 'number') return a - b;

  // Prefer numeric comparison for strings like "12" vs "9".
  return String(a ?? '').localeCompare(String(b ?? ''), undefined, {
    numeric: true,
    sensitivity: 'base',
  });
}

export function DataTable({
  columns,
  rows,
  striped = false,
  hoverable = false,
  caption,
  defaultSortKey,
  defaultSortDirection = 'desc',
  className,
  ...props
}: DataTableProps) {
  const firstSortableKey = columns.find((c) => c.sortable !== false)?.key;
  const [sortKey, setSortKey] = useState<string | undefined>(defaultSortKey || firstSortableKey);
  const [sortDirection, setSortDirection] = useState<'asc' | 'desc'>(defaultSortDirection);

  const ariaLabel = caption || 'Data table';

  if (columns.length === 0) {
    return (
      <div
        className={cn(
          'rounded-lg border border-border-default bg-background-muted/30 px-4 py-3 text-sm text-text-muted',
          className
        )}
        {...props}
      >
        No columns
      </div>
    );
  }

  const sortedRows = useMemo(() => {
    if (!sortKey) return rows;

    const sorted = [...rows].sort((ra, rb) => {
      const cmp = compareValues(ra[sortKey], rb[sortKey]);
      return sortDirection === 'asc' ? cmp : -cmp;
    });

    return sorted;
  }, [rows, sortKey, sortDirection]);

  return (
    <div
      className={cn('overflow-x-auto rounded-lg border border-border-default', className)}
      {...props}
    >
      <table className="w-full text-sm" aria-label={ariaLabel}>
        {caption && (
          <caption className="px-4 py-2 text-xs text-text-muted text-left">{caption}</caption>
        )}
        <thead>
          <tr className="border-b border-border-default bg-background-muted">
            {columns.map((col, colIndex) => {
              const isSorted = sortKey === col.key;
              const ariaSort: React.AriaAttributes['aria-sort'] = isSorted
                ? sortDirection === 'asc'
                  ? 'ascending'
                  : 'descending'
                : 'none';

              const sortable = col.sortable !== false;

              return (
                <th
                  key={`${col.key}-${colIndex}`}
                  scope="col"
                  aria-sort={sortable ? ariaSort : undefined}
                  className={cn(
                    'px-4 py-2.5 font-medium text-text-muted',
                    alignClass[col.align || 'left']
                  )}
                >
                  {sortable ? (
                    <button
                      type="button"
                      className={cn(
                        'inline-flex items-center gap-1 text-left',
                        'hover:text-text-default',
                        'focus:outline-none focus-visible:ring-2 focus-visible:ring-blue-400 focus-visible:ring-opacity-50 rounded'
                      )}
                      onClick={() => {
                        if (sortKey === col.key) {
                          setSortDirection((d) => (d === 'asc' ? 'desc' : 'asc'));
                        } else {
                          setSortKey(col.key);
                          setSortDirection('desc');
                        }
                      }}
                    >
                      <span>{col.label}</span>
                      {isSorted ? (
                        sortDirection === 'asc' ? (
                          <ArrowUp className="h-3 w-3 text-text-muted" />
                        ) : (
                          <ArrowDown className="h-3 w-3 text-text-muted" />
                        )
                      ) : null}
                      <span className="sr-only">
                        {isSorted
                          ? sortDirection === 'asc'
                            ? 'Sorted ascending'
                            : 'Sorted descending'
                          : 'Not sorted'}
                      </span>
                    </button>
                  ) : (
                    col.label
                  )}
                </th>
              );
            })}
          </tr>
        </thead>
        <tbody>
          {sortedRows.length === 0 ? (
            <tr className="border-b border-border-default last:border-0">
              <td colSpan={columns.length} className="px-4 py-3 text-text-muted">
                No rows
              </td>
            </tr>
          ) : (
            sortedRows.map((row, i) => {
              const contentKey = columns.map((c) => String(row[c.key] ?? '')).join('|');
              const rowKey = contentKey.length > 0 ? contentKey : String(i);
              return (
                <tr
                  key={rowKey}
                  className={cn(
                    'border-b border-border-default last:border-0',
                    striped && i % 2 === 1 && 'bg-background-muted/50',
                    hoverable && 'hover:bg-background-muted/30'
                  )}
                >
                  {columns.map((col, colIndex) => (
                    <td
                      key={`${col.key}-${colIndex}`}
                      className={cn(
                        'px-4 py-2.5 text-text-default',
                        alignClass[col.align || 'left']
                      )}
                    >
                      {String(row[col.key] ?? '')}
                    </td>
                  ))}
                </tr>
              );
            })
          )}
        </tbody>
      </table>
    </div>
  );
}
