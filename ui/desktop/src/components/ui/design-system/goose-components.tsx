/**
 * Goose Design System — json-render Component Map
 *
 * Maps catalog component types to React implementations.
 * Uses ComponentRenderProps from @json-render/react.
 */
'use client';

import type { ComponentRenderProps } from '@json-render/react';
import type React from 'react';
import { cn } from '../../../utils';
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

// ─── Layout Primitives (inline) ─────────────────────────────────

function StackComponent({
  element,
  children,
}: ComponentRenderProps<{
  direction?: 'vertical' | 'horizontal';
  gap?: 'sm' | 'md' | 'lg';
  align?: 'start' | 'center' | 'end';
}>) {
  const p = element.props || {};
  const gapMap = { sm: 'gap-2', md: 'gap-4', lg: 'gap-6' };
  const dir = p.direction === 'horizontal' ? 'flex-row' : 'flex-col';
  const alignMap = { start: 'items-start', center: 'items-center', end: 'items-end' };
  return (
    <div className={cn('flex', dir, gapMap[p.gap || 'md'], alignMap[p.align || 'start'])}>
      {children}
    </div>
  );
}

function GridComponent({
  element,
  children,
}: ComponentRenderProps<{
  columns?: number;
  gap?: 'sm' | 'md' | 'lg';
}>) {
  const p = element.props || {};
  const cols = p.columns || 2;
  const gapMap = { sm: 'gap-2', md: 'gap-4', lg: 'gap-6' };
  const colClass =
    cols === 1
      ? 'grid-cols-1'
      : cols === 2
        ? 'grid-cols-2'
        : cols === 3
          ? 'grid-cols-3'
          : 'grid-cols-4';
  return <div className={cn('grid', colClass, gapMap[p.gap || 'md'])}>{children}</div>;
}

function TextComponent({
  element,
}: ComponentRenderProps<{
  content?: string;
  variant?: 'body' | 'heading' | 'label' | 'caption' | 'code';
  color?: 'default' | 'muted' | 'accent' | 'success' | 'warning' | 'danger';
}>) {
  const p = element.props || {};
  const variantClass: Record<string, string> = {
    body: 'text-sm',
    heading: 'text-lg font-semibold',
    label: 'text-xs font-medium uppercase tracking-wide',
    caption: 'text-xs',
    code: 'font-mono text-sm bg-background-muted px-1.5 py-0.5 rounded',
  };
  const colorClass: Record<string, string> = {
    default: 'text-text-default',
    muted: 'text-text-muted',
    accent: 'text-text-accent',
    success: 'text-text-success',
    warning: 'text-text-warning',
    danger: 'text-text-danger',
  };
  return (
    <span className={cn(variantClass[p.variant || 'body'], colorClass[p.color || 'default'])}>
      {p.content}
    </span>
  );
}

function SeparatorComponent({
  element,
}: ComponentRenderProps<{ orientation?: 'horizontal' | 'vertical' }>) {
  const p = element.props || {};
  return p.orientation === 'vertical' ? (
    <div className="w-px bg-border-default self-stretch" />
  ) : (
    <hr className="border-border-default" />
  );
}

function BadgeComponent({
  element,
}: ComponentRenderProps<{
  text?: string;
  variant?: 'success' | 'warning' | 'danger' | 'info';
}>) {
  const p = element.props || {};
  const variantClass: Record<string, string> = {
    success: 'bg-background-success-muted text-text-success',
    warning: 'bg-background-warning-muted text-text-warning',
    danger: 'bg-background-danger-muted text-text-danger',
    info: 'bg-background-muted text-text-accent',
  };
  return (
    <span
      className={cn(
        'inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium',
        variantClass[p.variant || 'info']
      )}
    >
      {p.text}
    </span>
  );
}

// ─── Design System Component Wrappers ───────────────────────────

function PageHeaderComponent({
  element,
}: ComponentRenderProps<{
  title?: string;
  description?: string;
}>) {
  const p = element.props || {};
  return <PageHeader title={p.title || ''} description={p.description} />;
}

function DataCardComponent({
  element,
  children,
}: ComponentRenderProps<{
  variant?: 'default' | 'interactive' | 'stat';
}>) {
  const p = element.props || {};
  return <DataCard variant={p.variant}>{children}</DataCard>;
}

function StatCardComponent({
  element,
}: ComponentRenderProps<{
  label?: string;
  value?: string | number;
  color?: 'default' | 'success' | 'warning' | 'danger';
  trend?: number;
  trendDirection?: 'up' | 'down';
}>) {
  const p = element.props || {};
  const trend =
    p.trend != null
      ? { value: p.trend, direction: (p.trendDirection || 'up') as 'up' | 'down' }
      : undefined;
  return <StatCard label={p.label || ''} value={p.value ?? ''} variant={p.color} trend={trend} />;
}

function ListItemComponent({
  element,
}: ComponentRenderProps<{
  title?: string;
  description?: string;
  status?: 'active' | 'inactive' | 'error' | 'loading';
  indent?: number;
}>) {
  const p = element.props || {};
  return (
    <ListItem
      title={p.title || ''}
      description={p.description}
      status={p.status}
      indent={p.indent}
    />
  );
}

function TreeItemComponent({
  element,
  children,
}: ComponentRenderProps<{
  label?: string;
  badge?: string;
  childCount?: number;
  defaultExpanded?: boolean;
  indent?: number;
}>) {
  const p = element.props || {};
  return (
    <TreeItem
      label={p.label || ''}
      badge={p.badge ? <span className="text-xs text-text-muted">{p.badge}</span> : undefined}
      childCount={p.childCount}
      defaultExpanded={p.defaultExpanded}
      indent={p.indent}
    >
      {children}
    </TreeItem>
  );
}

function EmptyStateComponent({
  element,
}: ComponentRenderProps<{
  title?: string;
  description?: string;
}>) {
  const p = element.props || {};
  return <EmptyState title={p.title} description={p.description} />;
}

function LoadingStateComponent({
  element,
}: ComponentRenderProps<{
  variant?: 'spinner' | 'skeleton' | 'pulse';
  lines?: number;
}>) {
  const p = element.props || {};
  return <LoadingState variant={p.variant} lines={p.lines} />;
}

function ErrorStateComponent({
  element,
}: ComponentRenderProps<{
  title?: string;
  message?: string;
}>) {
  const p = element.props || {};
  return <ErrorState title={p.title} message={p.message} />;
}

function SearchInputComponent({
  element,
  emit,
}: ComponentRenderProps<{
  placeholder?: string;
  value?: string;
  debounceMs?: number;
}>) {
  const p = element.props || {};
  return (
    <SearchInput
      placeholder={p.placeholder}
      value={p.value}
      debounceMs={p.debounceMs}
      onChange={() => emit?.('change')}
    />
  );
}

function TabBarComponent({
  element,
  emit,
}: ComponentRenderProps<{
  tabs?: Array<{ id: string; label: string; group?: string; badge?: string }>;
  activeTab?: string;
  variant?: 'default' | 'pill' | 'underline';
}>) {
  const p = element.props || {};
  const rawTabs = p.tabs || [];

  // Group tabs
  const groupMap = new Map<string, typeof rawTabs>();
  for (const tab of rawTabs) {
    const g = tab.group || '';
    if (!groupMap.has(g)) groupMap.set(g, []);
    groupMap.get(g)?.push(tab);
  }

  const groups = Array.from(groupMap.entries()).map(([label, tabs]) => ({
    label: label || undefined,
    tabs: tabs.map((t) => ({ id: t.id, label: t.label, badge: t.badge })),
  }));

  return (
    <TabBar
      groups={groups}
      activeTab={p.activeTab || ''}
      onTabChange={() => emit?.('tabChange')}
      variant={p.variant}
    />
  );
}

// ─── Button ─────────────────────────────────────────────────────

function ButtonComponent({ element }: AnyComponentRenderProps) {
  const p = (element.props || {}) as {
    label: string;
    variant?: 'primary' | 'secondary' | 'destructive' | 'ghost';
    size?: 'sm' | 'md' | 'lg';
    disabled?: boolean;
  };
  const variants: Record<string, string> = {
    primary: 'bg-accent text-text-on-accent hover:opacity-90',
    secondary:
      'bg-background-muted text-text-default border border-border-default hover:bg-background-active',
    destructive: 'bg-red-600 text-white hover:opacity-90',
    ghost: 'text-text-muted hover:bg-background-active hover:text-text-default',
  };
  const sizes: Record<string, string> = {
    sm: 'px-2.5 py-1 text-xs',
    md: 'px-4 py-2 text-sm',
    lg: 'px-6 py-3 text-base',
  };
  return (
    <button
      className={`rounded-lg font-medium transition-colors ${variants[p.variant || 'primary']} ${sizes[p.size || 'md']} ${p.disabled ? 'opacity-50 cursor-not-allowed' : ''}`}
      disabled={p.disabled}
    >
      {p.label}
    </button>
  );
}

// ─── Table ──────────────────────────────────────────────────────

function TableComponent({ element }: AnyComponentRenderProps) {
  const p = (element.props || {}) as {
    columns: Array<{ key: string; label: string; align?: 'left' | 'center' | 'right' }>;
    rows: Array<Record<string, string | number | boolean>>;
    maxRows?: number;
    striped?: boolean;
  };
  const columns = p.columns || [];
  const rows = (p.rows || []).slice(0, p.maxRows || 50);
  return (
    <div className="overflow-x-auto rounded-lg border border-border-default">
      <table className="w-full text-sm">
        <thead>
          <tr className="bg-background-muted">
            {columns.map((col) => (
              <th
                key={col.key}
                className={`px-3 py-2 font-medium text-text-default text-${col.align || 'left'}`}
              >
                {col.label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody className="divide-y divide-border-default">
          {rows.map((row, i) => (
            <tr
              key={i}
              className={`${p.striped && i % 2 === 1 ? 'bg-background-muted/50' : ''} hover:bg-background-active`}
            >
              {columns.map((col) => (
                <td
                  key={col.key}
                  className={`px-3 py-2 text-text-muted text-${col.align || 'left'}`}
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

// ─── Alert ──────────────────────────────────────────────────────

function AlertComponent({ element }: AnyComponentRenderProps) {
  const p = (element.props || {}) as {
    title: string;
    message?: string;
    severity?: 'info' | 'success' | 'warning' | 'error';
  };
  const severity = p.severity || 'info';
  const styles: Record<string, { border: string; bg: string; icon: string }> = {
    info: { border: 'border-border-info', bg: 'bg-background-info/10', icon: 'ℹ️' },
    success: { border: 'border-text-success', bg: 'bg-text-success/10', icon: '✅' },
    warning: { border: 'border-text-warning', bg: 'bg-text-warning/10', icon: '⚠️' },
    error: { border: 'border-border-danger', bg: 'bg-background-danger/10', icon: '❌' },
  };
  const s = styles[severity] || styles.info;
  return (
    <div className={`p-4 rounded-xl border ${s.border} ${s.bg}`}>
      <div className="flex items-start gap-3">
        <span className="text-lg">{s.icon}</span>
        <div>
          <div className="font-medium text-text-default">{p.title}</div>
          {p.message && <div className="text-sm text-text-muted mt-1">{p.message}</div>}
        </div>
      </div>
    </div>
  );
}

// ─── Card ───────────────────────────────────────────────────────

function CardComponent({ element, children }: AnyComponentRenderProps) {
  const p = (element.props || {}) as {
    title?: string;
    subtitle?: string;
    variant?: 'default' | 'outlined' | 'elevated';
    padding?: 'none' | 'sm' | 'md' | 'lg';
  };
  const paddings: Record<string, string> = {
    none: 'p-0',
    sm: 'p-3',
    md: 'p-4',
    lg: 'p-6',
  };
  const variants: Record<string, string> = {
    default: 'bg-background-default border border-border-default',
    outlined: 'border-2 border-border-strong',
    elevated: 'bg-background-default border border-border-default shadow-md',
  };
  return (
    <div
      className={`rounded-xl ${variants[p.variant || 'default']} ${paddings[p.padding || 'md']}`}
    >
      {(p.title || p.subtitle) && (
        <div className={`${p.padding === 'none' ? 'px-4 pt-4' : ''} mb-3`}>
          {p.title && <h3 className="font-semibold text-text-default">{p.title}</h3>}
          {p.subtitle && <p className="text-sm text-text-muted mt-0.5">{p.subtitle}</p>}
        </div>
      )}
      <div>{children}</div>
    </div>
  );
}

// ─── Select ─────────────────────────────────────────────────────

function SelectComponent({ element }: AnyComponentRenderProps) {
  const p = (element.props || {}) as {
    label?: string;
    placeholder?: string;
    options: Array<{ value: string; label: string; disabled?: boolean }>;
    value?: string;
  };
  return (
    <div className="space-y-1.5">
      {p.label && <label className="text-sm font-medium text-text-default">{p.label}</label>}
      <select
        className="w-full rounded-lg border border-border-default bg-background-default text-text-default px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-border-accent"
        defaultValue={p.value || ''}
      >
        {p.placeholder && (
          <option value="" disabled>
            {p.placeholder}
          </option>
        )}
        {(p.options || []).map((opt) => (
          <option key={opt.value} value={opt.value} disabled={opt.disabled}>
            {opt.label}
          </option>
        ))}
      </select>
    </div>
  );
}

// ─── Progress ───────────────────────────────────────────────────

function ProgressComponent({ element }: AnyComponentRenderProps) {
  const p = (element.props || {}) as {
    label?: string;
    value: number;
    max?: number;
    color?: 'default' | 'success' | 'warning' | 'danger' | 'info';
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
          className={`h-full rounded-full transition-all ${colors[p.color || 'default']}`}
          style={{ width: `${pct}%` }}
        />
      </div>
    </div>
  );
}

// ─── CodeBlock ──────────────────────────────────────────────────

function CodeBlockComponent({ element }: AnyComponentRenderProps) {
  const p = (element.props || {}) as {
    code: string;
    language?: string;
    title?: string;
  };
  return (
    <div className="rounded-lg overflow-hidden border border-border-default">
      {(p.title || p.language) && (
        <div className="bg-background-muted px-3 py-1.5 text-xs text-text-muted border-b border-border-default flex justify-between">
          {p.title && <span>{p.title}</span>}
          {p.language && <span className="text-text-muted/60">{p.language}</span>}
        </div>
      )}
      <pre className="bg-background-default p-3 text-sm text-text-default overflow-x-auto">
        <code>{p.code}</code>
      </pre>
    </div>
  );
}

// ─── Input ──────────────────────────────────────────────────────

function InputComponent({ element }: AnyComponentRenderProps) {
  const p = (element.props || {}) as {
    label?: string;
    placeholder?: string;
    type?: 'text' | 'number' | 'email' | 'password' | 'url';
    value?: string;
    disabled?: boolean;
    helperText?: string;
  };
  return (
    <div className="space-y-1.5">
      {p.label && <label className="text-sm font-medium text-text-default">{p.label}</label>}
      <input
        type={p.type || 'text'}
        placeholder={p.placeholder}
        defaultValue={p.value}
        disabled={p.disabled}
        className={`w-full rounded-lg border border-border-default bg-background-default text-text-default px-3 py-2 text-sm placeholder:text-text-muted focus:outline-none focus:ring-2 focus:ring-border-accent ${p.disabled ? 'opacity-50 cursor-not-allowed' : ''}`}
      />
      {p.helperText && <p className="text-xs text-text-muted">{p.helperText}</p>}
    </div>
  );
}

// ─── Error-Isolated Registry ────────────────────────────────────

type AnyComponentRenderProps = ComponentRenderProps<Record<string, unknown>>;

function withErrorBoundary(
  name: string,
  Component: React.ComponentType<AnyComponentRenderProps>
): React.ComponentType<AnyComponentRenderProps> {
  const Wrapped = (props: AnyComponentRenderProps) => (
    <ElementErrorBoundary elementId={name}>
      <Component {...props} />
    </ElementErrorBoundary>
  );
  Wrapped.displayName = `ErrorBoundary(${name})`;
  return Wrapped;
}

const rawComponents: Record<string, React.ComponentType<AnyComponentRenderProps>> = {
  Stack: StackComponent as React.ComponentType<AnyComponentRenderProps>,
  Grid: GridComponent as React.ComponentType<AnyComponentRenderProps>,
  Text: TextComponent as React.ComponentType<AnyComponentRenderProps>,
  Separator: SeparatorComponent as React.ComponentType<AnyComponentRenderProps>,
  Badge: BadgeComponent as React.ComponentType<AnyComponentRenderProps>,
  PageHeader: PageHeaderComponent as React.ComponentType<AnyComponentRenderProps>,
  DataCard: DataCardComponent as React.ComponentType<AnyComponentRenderProps>,
  StatCard: StatCardComponent as React.ComponentType<AnyComponentRenderProps>,
  ListItem: ListItemComponent as React.ComponentType<AnyComponentRenderProps>,
  TreeItem: TreeItemComponent as React.ComponentType<AnyComponentRenderProps>,
  EmptyState: EmptyStateComponent as React.ComponentType<AnyComponentRenderProps>,
  LoadingState: LoadingStateComponent as React.ComponentType<AnyComponentRenderProps>,
  ErrorState: ErrorStateComponent as React.ComponentType<AnyComponentRenderProps>,
  SearchInput: SearchInputComponent as React.ComponentType<AnyComponentRenderProps>,
  TabBar: TabBarComponent as React.ComponentType<AnyComponentRenderProps>,
  Button: ButtonComponent as React.ComponentType<AnyComponentRenderProps>,
  Table: TableComponent as React.ComponentType<AnyComponentRenderProps>,
  Alert: AlertComponent as React.ComponentType<AnyComponentRenderProps>,
  Card: CardComponent as React.ComponentType<AnyComponentRenderProps>,
  Select: SelectComponent as React.ComponentType<AnyComponentRenderProps>,
  Progress: ProgressComponent as React.ComponentType<AnyComponentRenderProps>,
  CodeBlock: CodeBlockComponent as React.ComponentType<AnyComponentRenderProps>,
  Input: InputComponent as React.ComponentType<AnyComponentRenderProps>,
};

export const gooseComponents: Record<
  string,
  React.ComponentType<AnyComponentRenderProps>
> = Object.fromEntries(
  Object.entries(rawComponents).map(([name, Component]) => [
    name,
    withErrorBoundary(name, Component),
  ])
);
