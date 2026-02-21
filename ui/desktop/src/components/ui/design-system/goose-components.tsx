/**
 * Goose Design System — json-render Component Map
 *
 * Maps catalog component types → React implementations.
 * Every wrapper uses AnyComponentRenderProps and casts element.props internally
 * so all components are assignable to the unified registry type.
 *
 * Categories:
 *   - Layout primitives (Stack, Grid, Text) — intentionally raw <div>/<span>
 *   - DS Atom wrappers (Separator, Badge, Button, Input)
 *   - DS Molecule wrappers (Card)
 *   - DS Design-System wrappers (PageHeader, DataCard, StatCard, etc.)
 *   - Custom components (Table, Alert, Select, Progress, CodeBlock) — no DS equivalent yet
 */
import type { ComponentRenderProps } from '@json-render/react';
import type React from 'react';
import { cn } from '../../../utils';
import { Badge } from '../atoms/Badge';
import { Button } from '../atoms/button';
import { Input } from '../atoms/input';
import { Separator } from '../atoms/separator';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../molecules/card';
import { DataCard } from './DataCard';
import { ElementErrorBoundary } from './ElementErrorBoundary';
import { EmptyState } from './EmptyState';
import { ErrorState } from './ErrorState';
import { ListItem } from './ListItem';
import { LoadingState } from './LoadingState';
import { PageHeader } from './PageHeader';
import { SearchInput } from './SearchInput';
import { StatCard } from './StatCard';
import { TabBar } from './TabBar';
import { TreeItem } from './TreeItem';

type AnyComponentRenderProps = ComponentRenderProps<Record<string, unknown>>;

// ─── Layout Primitives (semantic HTML — intentionally minimal) ───

function StackComponent({ element, children }: AnyComponentRenderProps) {
  const p = (element.props || {}) as {
    direction?: 'horizontal' | 'vertical';
    gap?: string;
    align?: string;
  };
  const dir = p.direction === 'horizontal' ? 'flex-row' : 'flex-col';
  const gapMap: Record<string, string> = {
    xs: 'gap-1',
    sm: 'gap-2',
    md: 'gap-4',
    lg: 'gap-6',
    xl: 'gap-8',
  };
  const alignMap: Record<string, string> = {
    start: 'items-start',
    center: 'items-center',
    end: 'items-end',
    stretch: 'items-stretch',
  };
  return (
    <div
      className={cn('flex', dir, gapMap[p.gap || 'md'], p.align ? alignMap[p.align] : undefined)}
    >
      {children}
    </div>
  );
}

function GridComponent({ element, children }: AnyComponentRenderProps) {
  const p = (element.props || {}) as { columns?: number; gap?: string };
  const cols = p.columns || 2;
  const gapMap: Record<string, string> = {
    xs: 'gap-1',
    sm: 'gap-2',
    md: 'gap-4',
    lg: 'gap-6',
    xl: 'gap-8',
  };
  return (
    <div
      className={cn('grid', gapMap[p.gap || 'md'])}
      style={{ gridTemplateColumns: `repeat(${cols}, minmax(0, 1fr))` }}
    >
      {children}
    </div>
  );
}

function TextComponent({ element }: AnyComponentRenderProps) {
  const p = (element.props || {}) as {
    content?: string;
    variant?: string;
    color?: string;
  };
  const variantMap: Record<string, string> = {
    heading: 'text-lg font-semibold text-text-default',
    subheading: 'text-base font-medium text-text-default',
    body: 'text-sm text-text-default',
    caption: 'text-xs text-text-muted',
    label: 'text-sm font-medium text-text-default',
    code: 'font-mono text-sm bg-background-muted px-1.5 py-0.5 rounded',
    lead: 'text-base text-text-muted',
  };
  const colorMap: Record<string, string> = {
    default: 'text-text-default',
    muted: 'text-text-muted',
    success: 'text-text-success',
    warning: 'text-text-warning',
    danger: 'text-text-danger',
    info: 'text-text-info',
    accent: 'text-text-accent',
  };
  return (
    <span className={cn(variantMap[p.variant || 'body'], p.color ? colorMap[p.color] : undefined)}>
      {p.content}
    </span>
  );
}

// ─── DS Atom Wrappers ───

function SeparatorComponent({ element }: AnyComponentRenderProps) {
  const p = (element.props || {}) as { orientation?: 'horizontal' | 'vertical' };
  return <Separator orientation={p.orientation || 'horizontal'} />;
}

function BadgeComponent({ element }: AnyComponentRenderProps) {
  const p = (element.props || {}) as { label?: string; variant?: string };
  const variantMap: Record<string, 'default' | 'secondary' | 'accent' | 'muted' | 'outline'> = {
    default: 'default',
    success: 'default',
    warning: 'accent',
    danger: 'accent',
    info: 'default',
    secondary: 'secondary',
    muted: 'muted',
    outline: 'outline',
  };
  return <Badge variant={variantMap[p.variant || 'default'] || 'default'}>{p.label}</Badge>;
}

function ButtonComponent({ element }: AnyComponentRenderProps) {
  const p = (element.props || {}) as {
    label?: string;
    variant?: string;
    size?: string;
    disabled?: boolean;
  };
  const variantMap: Record<string, 'default' | 'secondary' | 'destructive' | 'ghost' | 'outline'> =
    {
      primary: 'default',
      secondary: 'secondary',
      destructive: 'destructive',
      ghost: 'ghost',
    };
  const sizeMap: Record<string, 'sm' | 'default' | 'lg'> = {
    sm: 'sm',
    md: 'default',
    lg: 'lg',
  };
  return (
    <Button
      variant={variantMap[p.variant || 'primary'] || 'default'}
      size={sizeMap[p.size || 'md'] || 'default'}
      disabled={p.disabled}
    >
      {p.label}
    </Button>
  );
}

function InputComponent({ element }: AnyComponentRenderProps) {
  const p = (element.props || {}) as {
    label?: string;
    placeholder?: string;
    type?: string;
    value?: string;
    disabled?: boolean;
    helperText?: string;
  };
  return (
    <div className="space-y-1.5">
      {p.label && <label className="text-sm font-medium text-text-default">{p.label}</label>}
      <Input
        type={p.type || 'text'}
        placeholder={p.placeholder}
        defaultValue={p.value}
        disabled={p.disabled}
      />
      {p.helperText && <p className="text-xs text-text-muted">{p.helperText}</p>}
    </div>
  );
}

// ─── DS Molecule Wrappers ───

function CardComponent({ element, children }: AnyComponentRenderProps) {
  const p = (element.props || {}) as {
    title?: string;
    subtitle?: string;
    maxWidth?: string;
  };
  const maxWidthMap: Record<string, string> = {
    sm: 'max-w-sm',
    md: 'max-w-md',
    lg: 'max-w-2xl',
    xl: 'max-w-4xl',
    full: 'max-w-full',
  };
  return (
    <Card className={cn(p.maxWidth ? maxWidthMap[p.maxWidth] : undefined)}>
      {(p.title || p.subtitle) && (
        <CardHeader>
          {p.title && <CardTitle>{p.title}</CardTitle>}
          {p.subtitle && <CardDescription>{p.subtitle}</CardDescription>}
        </CardHeader>
      )}
      <CardContent>{children}</CardContent>
    </Card>
  );
}

// ─── DS Design-System Wrappers ───

function PageHeaderComponent({ element }: AnyComponentRenderProps) {
  const p = (element.props || {}) as {
    title?: string;
    description?: string;
    badge?: string;
  };
  return (
    <PageHeader
      title={p.title || ''}
      description={p.description}
      badge={p.badge ? <Badge>{p.badge}</Badge> : undefined}
    />
  );
}

function DataCardComponent({ element, children }: AnyComponentRenderProps) {
  const p = (element.props || {}) as { variant?: 'default' | 'interactive' | 'stat' };
  return <DataCard variant={p.variant || 'default'}>{children}</DataCard>;
}

function StatCardComponent({ element }: AnyComponentRenderProps) {
  const p = (element.props || {}) as {
    label?: string;
    value?: string | number;
    trend?: { direction: 'up' | 'down'; value: string | number };
    variant?: 'default' | 'success' | 'warning' | 'danger';
  };
  const trend = p.trend
    ? { direction: p.trend.direction, value: Number(p.trend.value) || 0 }
    : undefined;
  return (
    <StatCard
      label={p.label || ''}
      value={String(p.value ?? '')}
      trend={trend}
      variant={p.variant}
    />
  );
}

function ListItemComponent({ element }: AnyComponentRenderProps) {
  const p = (element.props || {}) as {
    title?: string;
    description?: string;
    status?: 'active' | 'inactive' | 'error' | 'loading';
  };
  return <ListItem title={p.title || ''} description={p.description} status={p.status} />;
}

function TreeItemComponent({ element, children }: AnyComponentRenderProps) {
  const p = (element.props || {}) as {
    label?: string;
    expanded?: boolean;
    childCount?: number;
  };
  return (
    <TreeItem label={p.label || ''} expanded={p.expanded} childCount={p.childCount}>
      {children}
    </TreeItem>
  );
}

function EmptyStateComponent({ element }: AnyComponentRenderProps) {
  const p = (element.props || {}) as {
    title?: string;
    description?: string;
  };
  return <EmptyState title={p.title || ''} description={p.description} />;
}

function LoadingStateComponent({ element }: AnyComponentRenderProps) {
  const p = (element.props || {}) as {
    variant?: 'spinner' | 'skeleton' | 'pulse';
    lines?: number;
  };
  return <LoadingState variant={p.variant} lines={p.lines} />;
}

function ErrorStateComponent({ element }: AnyComponentRenderProps) {
  const p = (element.props || {}) as { message?: string; title?: string };
  return <ErrorState message={p.message} title={p.title} />;
}

function SearchInputComponent({ element }: AnyComponentRenderProps) {
  const p = (element.props || {}) as { placeholder?: string; value?: string };
  return <SearchInput placeholder={p.placeholder} value={p.value || ''} />;
}

function TabBarComponent({ element, emit }: AnyComponentRenderProps) {
  const p = (element.props || {}) as {
    tabs?: Array<{ id: string; label: string; badge?: string }>;
    activeTab?: string;
    variant?: 'default' | 'pill' | 'underline';
  };
  // TabBar expects TabGroup[] with Tab[] — Tab.icon is LucideIcon, skip string icons
  const groups = [{ tabs: (p.tabs || []).map(({ id, label, badge }) => ({ id, label, badge })) }];
  return (
    <TabBar
      groups={groups}
      activeTab={p.activeTab || ''}
      onTabChange={(tabId) => emit?.(`tabChange:${tabId}`)}
      variant={p.variant}
    />
  );
}

// ─── Components kept as custom (no DS equivalent yet) ───

function TableComponent({ element }: AnyComponentRenderProps) {
  const p = (element.props || {}) as {
    columns?: Array<{ key: string; label: string; align?: string }>;
    rows?: Array<Record<string, unknown>>;
    striped?: boolean;
    hoverable?: boolean;
    caption?: string;
  };
  const columns = p.columns || [];
  const rows = p.rows || [];
  return (
    <div className="overflow-x-auto rounded-lg border border-border-default">
      <table className="w-full text-sm">
        {p.caption && (
          <caption className="px-4 py-2 text-xs text-text-muted text-left">{p.caption}</caption>
        )}
        <thead>
          <tr className="border-b border-border-default bg-background-muted">
            {columns.map((col) => (
              <th
                key={col.key}
                className={cn(
                  'px-4 py-2.5 font-medium text-text-muted text-left',
                  col.align === 'right' && 'text-right',
                  col.align === 'center' && 'text-center'
                )}
              >
                {col.label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row, i) => (
            <tr
              key={i}
              className={cn(
                'border-b border-border-default last:border-0',
                p.striped && i % 2 === 1 && 'bg-background-muted/50',
                p.hoverable && 'hover:bg-background-muted/30'
              )}
            >
              {columns.map((col) => (
                <td
                  key={col.key}
                  className={cn(
                    'px-4 py-2.5 text-text-default',
                    col.align === 'right' && 'text-right',
                    col.align === 'center' && 'text-center'
                  )}
                >
                  {String(row[col.key] ?? '')}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function AlertComponent({ element }: AnyComponentRenderProps) {
  const p = (element.props || {}) as {
    title?: string;
    message?: string;
    severity?: string;
  };
  const severity = p.severity || 'info';
  const styles: Record<string, string> = {
    info: 'border-blue-500/30 bg-blue-500/5 text-blue-600 dark:text-blue-400',
    success: 'border-green-500/30 bg-green-500/5 text-green-600 dark:text-green-400',
    warning: 'border-amber-500/30 bg-amber-500/5 text-amber-600 dark:text-amber-400',
    error: 'border-red-500/30 bg-red-500/5 text-red-600 dark:text-red-400',
  };
  const icons: Record<string, string> = {
    info: 'ℹ️',
    success: '✅',
    warning: '⚠️',
    error: '❌',
  };
  return (
    <div className={cn('rounded-lg border p-4', styles[severity] || styles.info)}>
      <div className="flex gap-3">
        <span className="text-base shrink-0">{icons[severity] || 'ℹ️'}</span>
        <div className="space-y-1">
          {p.title && <div className="font-medium text-sm">{p.title}</div>}
          <div className="text-sm opacity-90">{p.message}</div>
        </div>
      </div>
    </div>
  );
}

function SelectComponent({ element }: AnyComponentRenderProps) {
  const p = (element.props || {}) as {
    label?: string;
    placeholder?: string;
    options?: Array<{ value: string; label: string; disabled?: boolean }>;
    value?: string;
  };
  const options = p.options || [];
  return (
    <div className="space-y-1.5">
      {p.label && <label className="text-sm font-medium text-text-default">{p.label}</label>}
      <select
        className="w-full rounded-md border border-border-default bg-background-default text-text-default px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-border-accent"
        defaultValue={p.value || ''}
      >
        {p.placeholder && (
          <option value="" disabled>
            {p.placeholder}
          </option>
        )}
        {options.map((opt) => (
          <option key={opt.value} value={opt.value} disabled={opt.disabled}>
            {opt.label}
          </option>
        ))}
      </select>
    </div>
  );
}

function ProgressComponent({ element }: AnyComponentRenderProps) {
  const p = (element.props || {}) as {
    label?: string;
    value?: number;
    max?: number;
    color?: string;
    showValue?: boolean;
  };
  const pct = Math.min(100, Math.max(0, ((p.value || 0) / (p.max || 100)) * 100));
  const colors: Record<string, string> = {
    default: 'bg-accent',
    success: 'bg-text-success',
    warning: 'bg-text-warning',
    danger: 'bg-text-danger',
    info: 'bg-text-info',
  };
  return (
    <div className="space-y-1.5">
      {(p.label || p.showValue !== false) && (
        <div className="flex justify-between text-sm">
          {p.label && <span className="text-text-default">{p.label}</span>}
          {p.showValue !== false && <span className="text-text-muted">{Math.round(pct)}%</span>}
        </div>
      )}
      <div className="h-2 bg-background-muted rounded-full overflow-hidden">
        <div
          className={cn(
            'h-full rounded-full transition-all',
            colors[p.color || 'default'] || colors.default
          )}
          style={{ width: `${pct}%` }}
        />
      </div>
    </div>
  );
}

function CodeBlockComponent({ element }: AnyComponentRenderProps) {
  const p = (element.props || {}) as {
    code?: string;
    language?: string;
  };
  return (
    <div className="rounded-lg border border-border-default bg-background-muted overflow-hidden">
      {p.language && (
        <div className="flex items-center justify-between px-4 py-1.5 bg-background-active border-b border-border-default">
          <span className="text-xs text-text-muted font-mono">{p.language}</span>
        </div>
      )}
      <pre className="p-4 overflow-x-auto text-sm">
        <code className="font-mono text-text-default">{p.code}</code>
      </pre>
    </div>
  );
}

// ─── Error Boundary Wrapper ───

function withErrorBoundary(
  Component: React.ComponentType<AnyComponentRenderProps>,
  displayName: string
) {
  const Wrapped = (props: AnyComponentRenderProps) => (
    <ElementErrorBoundary elementId={displayName}>
      <Component {...props} />
    </ElementErrorBoundary>
  );
  Wrapped.displayName = `ErrorBoundary(${displayName})`;
  return Wrapped;
}

// ─── Registry ───

const rawComponents: Record<string, React.ComponentType<AnyComponentRenderProps>> = {
  // Layout primitives
  Stack: StackComponent,
  Grid: GridComponent,
  Text: TextComponent,
  // DS Atoms
  Separator: SeparatorComponent,
  Badge: BadgeComponent,
  Button: ButtonComponent,
  Input: InputComponent,
  // DS Molecules
  Card: CardComponent,
  // DS Design-System
  PageHeader: PageHeaderComponent,
  DataCard: DataCardComponent,
  StatCard: StatCardComponent,
  ListItem: ListItemComponent,
  TreeItem: TreeItemComponent,
  EmptyState: EmptyStateComponent,
  LoadingState: LoadingStateComponent,
  ErrorState: ErrorStateComponent,
  SearchInput: SearchInputComponent,
  TabBar: TabBarComponent,
  // Custom (no DS equivalent yet)
  Table: TableComponent,
  Alert: AlertComponent,
  Select: SelectComponent,
  Progress: ProgressComponent,
  CodeBlock: CodeBlockComponent,
};

export const gooseComponents = Object.fromEntries(
  Object.entries(rawComponents).map(([name, Component]) => [
    name,
    withErrorBoundary(Component, name),
  ])
) as Record<string, React.ComponentType<AnyComponentRenderProps>>;
