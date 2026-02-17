/**
 * Goose Design System — json-render Catalog
 *
 * Component definitions compatible with @vercel-labs/json-render.
 * Allows generative UI to render design system components from JSON specs.
 *
 * Usage:
 *   import { gooseCatalog } from './catalog';
 *   import { Render } from '@vercel-labs/json-render/react';
 *   <Render spec={jsonSpec} catalog={gooseCatalog} />
 *
 * Reference: https://json-render.dev/docs
 * Local clone: /home/jmercier/codes/json-render
 */

import { z } from 'zod';

export const gooseComponentDefinitions = {
  PageHeader: {
    props: z.object({
      title: z.string(),
      description: z.string().optional(),
    }),
    slots: { actions: true, badge: true },
    description: 'Page header with title, optional description, and action buttons.',
    example: { title: 'Dashboard', description: 'Monitor your agent activity' },
  },

  DataCard: {
    props: z.object({
      variant: z.enum(['default', 'interactive', 'stat']).default('default'),
    }),
    slots: { children: true },
    events: { onClick: z.function() },
    description: 'Generic container card for data display.',
    example: { variant: 'default' },
  },

  StatCard: {
    props: z.object({
      label: z.string(),
      value: z.union([z.string(), z.number()]),
      variant: z.enum(['default', 'success', 'warning', 'danger']).default('default'),
      trend: z.object({
        value: z.number(),
        direction: z.enum(['up', 'down']),
      }).optional(),
    }),
    description: 'Metric card showing a KPI with optional trend indicator.',
    example: { label: 'Total Sessions', value: 1234, variant: 'default', trend: { value: 12, direction: 'up' } },
  },

  ListItem: {
    props: z.object({
      title: z.string(),
      description: z.string().optional(),
      status: z.enum(['active', 'inactive', 'error', 'loading']).optional(),
      indent: z.number().default(0),
    }),
    slots: { icon: true, actions: true },
    events: { onClick: z.function() },
    description: 'Row item for lists with optional status dot, icon, and actions.',
    example: { title: 'Developer Extension', description: 'Code editing tools', status: 'active' },
  },

  TreeItem: {
    props: z.object({
      label: z.string(),
      childCount: z.number().optional(),
      defaultExpanded: z.boolean().default(false),
      indent: z.number().default(0),
    }),
    slots: { icon: true, badge: true, children: true },
    events: { onToggle: z.function() },
    description: 'Expandable tree node with chevron, optional child count badge, and nested children.',
    example: { label: 'QA Pipeline', childCount: 4, defaultExpanded: true },
  },

  EmptyState: {
    props: z.object({
      title: z.string().default('No items yet'),
      description: z.string().optional(),
    }),
    slots: { action: true },
    description: 'Centered empty state with icon, title, description, and optional action button.',
    example: { title: 'No recipes found', description: 'Create your first recipe to get started.' },
  },

  LoadingState: {
    props: z.object({
      variant: z.enum(['spinner', 'skeleton', 'pulse']).default('skeleton'),
      lines: z.number().default(3),
    }),
    description: 'Loading placeholder with spinner, skeleton, or pulse animation.',
    example: { variant: 'skeleton', lines: 3 },
  },

  ErrorState: {
    props: z.object({
      title: z.string().default('Something went wrong'),
      message: z.string().optional(),
    }),
    events: { onRetry: z.function() },
    description: 'Error display with retry button.',
    example: { title: 'Failed to load', message: 'Check your connection and try again.' },
  },

  SearchInput: {
    props: z.object({
      placeholder: z.string().default('Search...'),
      debounceMs: z.number().default(300),
    }),
    events: { onChange: z.function() },
    description: 'Search input with icon, clear button, and optional debounce.',
    example: { placeholder: 'Search recipes...', debounceMs: 300 },
  },

  TabBar: {
    props: z.object({
      activeTab: z.string(),
      variant: z.enum(['default', 'pill', 'underline']).default('default'),
      groups: z.array(z.object({
        label: z.string().optional(),
        tabs: z.array(z.object({
          id: z.string(),
          label: z.string(),
          badge: z.union([z.string(), z.number()]).optional(),
        })),
      })),
    }),
    events: { onTabChange: z.function() },
    description: 'Grouped tab bar with multiple variants (default, pill, underline).',
    example: {
      activeTab: 'dashboard',
      variant: 'default',
      groups: [
        { label: 'Observe', tabs: [{ id: 'dashboard', label: 'Dashboard' }, { id: 'live', label: 'Live' }] },
        { label: 'Evaluate', tabs: [{ id: 'overview', label: 'Overview' }] },
      ],
    },
  },
} as const;
