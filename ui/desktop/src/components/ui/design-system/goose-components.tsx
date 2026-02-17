/**
 * Goose Design System — json-render Component Map
 *
 * Maps catalog component types to React implementations.
 * Uses ComponentRenderProps from @json-render/react.
 */
'use client';

import React from 'react';
import type { ComponentRenderProps } from '@json-render/react';
import { ElementErrorBoundary } from './ElementErrorBoundary';
import { PageHeader } from './PageHeader';
import { DataCard } from './DataCard';
import { StatCard } from './StatCard';
import { ListItem } from './ListItem';
import { TreeItem } from './TreeItem';
import { EmptyState } from './EmptyState';
import { LoadingState } from './LoadingState';
import { ErrorState } from './ErrorState';
import { SearchInput } from './SearchInput';
import { TabBar } from './TabBar';
import { cn } from '../../../utils';

// ─── Layout Primitives (inline) ─────────────────────────────────

function StackComponent({ element, children }: ComponentRenderProps<{
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

function GridComponent({ element, children }: ComponentRenderProps<{
  columns?: number;
  gap?: 'sm' | 'md' | 'lg';
}>) {
  const p = element.props || {};
  const cols = p.columns || 2;
  const gapMap = { sm: 'gap-2', md: 'gap-4', lg: 'gap-6' };
  const colClass = cols === 1 ? 'grid-cols-1' : cols === 2 ? 'grid-cols-2' : cols === 3 ? 'grid-cols-3' : 'grid-cols-4';
  return <div className={cn('grid', colClass, gapMap[p.gap || 'md'])}>{children}</div>;
}

function TextComponent({ element }: ComponentRenderProps<{
  content?: string;
  variant?: 'body' | 'heading' | 'label' | 'caption' | 'code';
  color?: 'default' | 'muted' | 'accent' | 'success' | 'warning' | 'danger';
}>) {
  const p = element.props || {};
  const variantClass: Record<string, string> = {
    body: 'text-sm', heading: 'text-lg font-semibold', label: 'text-xs font-medium uppercase tracking-wide',
    caption: 'text-xs', code: 'font-mono text-sm bg-background-muted px-1.5 py-0.5 rounded',
  };
  const colorClass: Record<string, string> = {
    default: 'text-text-default', muted: 'text-text-muted', accent: 'text-text-accent',
    success: 'text-text-success', warning: 'text-text-warning', danger: 'text-text-danger',
  };
  return <span className={cn(variantClass[p.variant || 'body'], colorClass[p.color || 'default'])}>{p.content}</span>;
}

function SeparatorComponent({ element }: ComponentRenderProps<{ orientation?: 'horizontal' | 'vertical' }>) {
  const p = element.props || {};
  return p.orientation === 'vertical'
    ? <div className="w-px bg-border-default self-stretch" />
    : <hr className="border-border-default" />;
}

function BadgeComponent({ element }: ComponentRenderProps<{
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
    <span className={cn('inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium', variantClass[p.variant || 'info'])}>
      {p.text}
    </span>
  );
}

// ─── Design System Component Wrappers ───────────────────────────

function PageHeaderComponent({ element }: ComponentRenderProps<{
  title?: string; description?: string;
}>) {
  const p = element.props || {};
  return <PageHeader title={p.title || ''} description={p.description} />;
}

function DataCardComponent({ element, children }: ComponentRenderProps<{
  variant?: 'default' | 'interactive' | 'stat';
}>) {
  const p = element.props || {};
  return <DataCard variant={p.variant}>{children}</DataCard>;
}

function StatCardComponent({ element }: ComponentRenderProps<{
  label?: string; value?: string | number;
  color?: 'default' | 'success' | 'warning' | 'danger';
  trend?: number; trendDirection?: 'up' | 'down';
}>) {
  const p = element.props || {};
  const trend = p.trend != null ? { value: p.trend, direction: (p.trendDirection || 'up') as 'up' | 'down' } : undefined;
  return <StatCard label={p.label || ''} value={p.value ?? ''} variant={p.color} trend={trend} />;
}

function ListItemComponent({ element }: ComponentRenderProps<{
  title?: string; description?: string;
  status?: 'active' | 'inactive' | 'error' | 'loading';
  indent?: number;
}>) {
  const p = element.props || {};
  return <ListItem title={p.title || ''} description={p.description} status={p.status} indent={p.indent} />;
}

function TreeItemComponent({ element, children }: ComponentRenderProps<{
  label?: string; badge?: string; childCount?: number;
  defaultExpanded?: boolean; indent?: number;
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

function EmptyStateComponent({ element }: ComponentRenderProps<{
  title?: string; description?: string;
}>) {
  const p = element.props || {};
  return <EmptyState title={p.title} description={p.description} />;
}

function LoadingStateComponent({ element }: ComponentRenderProps<{
  variant?: 'spinner' | 'skeleton' | 'pulse'; lines?: number;
}>) {
  const p = element.props || {};
  return <LoadingState variant={p.variant} lines={p.lines} />;
}

function ErrorStateComponent({ element }: ComponentRenderProps<{
  title?: string; message?: string;
}>) {
  const p = element.props || {};
  return <ErrorState title={p.title} message={p.message} />;
}

function SearchInputComponent({ element, emit }: ComponentRenderProps<{
  placeholder?: string; value?: string; debounceMs?: number;
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

function TabBarComponent({ element, emit }: ComponentRenderProps<{
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
    groupMap.get(g)!.push(tab);
  }

  const groups = Array.from(groupMap.entries()).map(([label, tabs]) => ({
    label: label || undefined,
    tabs: tabs.map(t => ({ id: t.id, label: t.label, badge: t.badge })),
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

// ─── Error-Isolated Registry ────────────────────────────────────

type AnyComponentRenderProps = ComponentRenderProps<Record<string, unknown>>;

function withErrorBoundary(
  name: string,
  Component: React.ComponentType<AnyComponentRenderProps>,
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
};

export const gooseComponents: Record<string, React.ComponentType<AnyComponentRenderProps>> =
  Object.fromEntries(
    Object.entries(rawComponents).map(([name, Component]) => [
      name,
      withErrorBoundary(name, Component),
    ]),
  );
