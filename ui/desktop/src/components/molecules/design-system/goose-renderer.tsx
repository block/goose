/**
 * Goose Design System — Unified json-render Renderer
 *
 * Renders AI-generated JSON UI specs using the unified CatalogRenderer
 * that merges shadcn (33 components) + goose custom (11 components).
 *
 * Both goose-ui and json-render code blocks now use the same renderer.
 *
 * Usage:
 *   import { GooseGenerativeUI, isGooseUISpec } from './goose-renderer';
 *   <GooseGenerativeUI spec={jsonSpec} onAction={handleAction} />
 */
'use client';

import type { Spec } from '@json-render/react';
import { CatalogRenderer } from '@/components/organisms/json-render/setup';
import { ElementErrorBoundary } from './element-error-boundary';


export type GooseActionHandler = (
  actionName: string,
  params?: Record<string, unknown>
) => void | Promise<void>;

export function GooseGenerativeUI({
  spec,
  onAction,
  loading,
}: {
  spec: Spec | null;
  onAction?: GooseActionHandler;
  state?: Record<string, unknown>;
  loading?: boolean;
}) {
  if (!spec) return null;

  return (
    <ElementErrorBoundary elementId="GooseGenerativeUI:root">
      <CatalogRenderer
        spec={spec}
        onAction={
          onAction
            ? (action: string, params?: Record<string, unknown>) => onAction(action, params)
            : undefined
        }
        loading={loading}
      />
    </ElementErrorBoundary>
  );
}

export function isGooseUISpec(obj: unknown): obj is Spec {
  if (!obj || typeof obj !== 'object') return false;
  const spec = obj as Record<string, unknown>;
  return (
    typeof spec.root === 'string' && typeof spec.elements === 'object' && spec.elements !== null
  );
}

export function getGooseUIPromptInstructions(): string {
  return `You can generate structured UI using JSON specs. The spec format is:
{
  "root": "main",
  "elements": {
    "main": { "type": "Stack", "props": { "gap": "md" }, "children": ["child1", "child2"] },
    "child1": { "type": "StatCard", "props": { "label": "Sessions", "value": "42" } },
    "child2": { "type": "ListItem", "props": { "title": "Task 1", "status": "active" } }
  }
}

Available components (44 — shadcn + goose custom):

LAYOUT (shadcn):
- Stack: direction ("vertical"|"horizontal"), gap ("sm"|"md"|"lg"), align ("start"|"center"|"end")
- Grid: columns (1-4), gap ("sm"|"md"|"lg")
- Card: title, subtitle, variant, padding — supports children
- Separator: orientation ("horizontal"|"vertical")
- Tabs: items (array of {label, value}), defaultValue
- Accordion: items (array of {trigger, content})
- Dialog: trigger, title, description — supports children
- Drawer: trigger, title, description — supports children

DISPLAY (shadcn):
- Heading: level (1-6), content
- Text: content, variant ("body"|"muted"|"lead"|"small"), color
- Badge: text, variant ("default"|"secondary"|"destructive"|"outline")
- Alert: title, description, variant ("default"|"destructive")
- Image: src, alt, width, height
- Avatar: src, alt, fallback
- Progress: value, max
- Skeleton: width, height
- Spinner: size

DISPLAY (goose custom):
- PageHeader: title (required), description, badge
- StatCard: label (required), value (required), color ("success"|"warning"|"danger"), trend (number)
- DataCard: title, description, variant ("default"|"interactive"|"stat") — supports children
- CodeBlock: code (required), language, title
- Chart: type ("bar"|"line"|"area"|"pie"), data (array of objects), xKey, yKeys (array), height, title, colors (array) — use bar for comparisons, line for trends, pie for proportions

DATA (shadcn):
- Table: columns, rows, caption
- Carousel: items

DATA (goose custom):
- DataTable: columns (array of {key,label,align?,sortable?}), rows (array of objects), caption, striped, hoverable, defaultSortKey, defaultSortDirection — sortable by clicking column headers
- ListItem: title (required), description, status ("active"|"inactive"|"error"|"loading"), indent
- TreeItem: label (required), badge, childCount, defaultExpanded, indent — supports children

INPUT (shadcn):
- Input: label, placeholder, type, value, disabled
- Textarea: label, placeholder, rows, value
- Select: label, placeholder, options (array of {value, label})
- Checkbox: label, checked, disabled
- Switch: label, checked, disabled
- Slider: min, max, step, value
- Button: label, variant ("default"|"secondary"|"destructive"|"outline"|"ghost"), size
- Toggle: label, pressed
- Link: href, label, variant

FEEDBACK (goose custom):
- EmptyState: title, description
- LoadingState: variant ("spinner"|"skeleton"|"pulse"), lines
- ErrorState: title, message

NAVIGATION:
- TabBar: tabs (array of {id, label, group?, badge?}), activeTab, variant ("default"|"pill"|"underline")
- SearchInput: placeholder, value, debounceMs
- Pagination: currentPage, totalPages
- DropdownMenu: trigger, items (array of {label, action})
- Tooltip: content, trigger
- Popover: trigger — supports children
- Collapsible: trigger — supports children

RULES:
- Every element needs a unique string ID in the elements map
- root points to the top-level element ID
- children are arrays of element IDs (strings), not inline objects
- Use Grid with StatCards for dashboards
- Use Stack direction="vertical" as default layout
- Always wrap multiple items in a Stack or Grid
- Use Card to group related content with a title
- Use DataTable for sortable comparisons/rankings; use Table for basic display
- Use Alert for status messages and notifications`;
}
