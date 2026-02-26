import type * as React from 'react';
import { cn } from '@/utils';

interface TableColumn {
  key: string;
  label: string;
  align?: 'left' | 'center' | 'right';
}

interface TableProps extends React.ComponentProps<'div'> {
  columns: TableColumn[];
  rows: Array<Record<string, unknown>>;
  striped?: boolean;
  hoverable?: boolean;
  caption?: string;
}

const alignClass: Record<string, string> = {
  right: 'text-right',
  center: 'text-center',
  left: 'text-left',
};

export function Table({
  columns,
  rows,
  striped = false,
  hoverable = false,
  caption,
  className,
  ...props
}: TableProps) {
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
            {columns.map((col, colIndex) => (
              <th
                key={`${col.key}-${colIndex}`}
                scope="col"
                className={cn(
                  'px-4 py-2.5 font-medium text-text-muted text-left',
                  alignClass[col.align || 'left']
                )}
              >
                {col.label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.length === 0 ? (
            <tr className="border-b border-border-default last:border-0">
              <td colSpan={columns.length} className="px-4 py-3 text-text-muted">
                No rows
              </td>
            </tr>
          ) : (
            rows.map((row, i) => {
              const contentKey = columns.map((c) => String(row[c.key] ?? '')).join('|');
              const rowKey = `${i}-${contentKey}`;
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
