/**
 * Goose Design System — json-render Renderer
 *
 * Renders AI-generated JSON UI specs using the Goose design system.
 * Uses JSONUIProvider for state/action management + Renderer for element tree.
 *
 * Usage:
 *   import { GooseGenerativeUI, isGooseUISpec } from './goose-renderer';
 *   <GooseGenerativeUI spec={jsonSpec} onAction={handleAction} />
 */
'use client';

import type { Spec } from '@json-render/react';
import { JSONUIProvider, Renderer } from '@json-render/react';
import { ElementErrorBoundary } from './ElementErrorBoundary';
import { gooseComponents } from './goose-components';

export type GooseActionHandler = (
  actionName: string,
  params?: Record<string, unknown>
) => void | Promise<void>;

export function GooseGenerativeUI({
  spec,
  onAction,
  state,
  loading,
}: {
  spec: Spec | null;
  onAction?: GooseActionHandler;
  state?: Record<string, unknown>;
  loading?: boolean;
}) {
  const handlers = onAction
    ? new Proxy({} as Record<string, (params: Record<string, unknown>) => unknown>, {
        get: (_target, prop: string) => {
          return (params: Record<string, unknown>) => onAction(prop, params);
        },
      })
    : undefined;

  return (
    <ElementErrorBoundary elementId="GooseGenerativeUI:root">
      <JSONUIProvider registry={gooseComponents} initialState={state} handlers={handlers}>
        <Renderer spec={spec} registry={gooseComponents} loading={loading} />
      </JSONUIProvider>
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

Available components (23):

LAYOUT:
- Stack: direction ("vertical"|"horizontal"), gap ("sm"|"md"|"lg"), align ("start"|"center"|"end")
- Grid: columns (1-4), gap ("sm"|"md"|"lg")
- Card: title, subtitle, variant ("default"|"outlined"|"elevated"), padding ("none"|"sm"|"md"|"lg") — supports children

DISPLAY:
- PageHeader: title (required), description, badge
- StatCard: label (required), value (required), color ("success"|"warning"|"danger"), trend (number)
- DataCard: title, description, variant ("default"|"interactive"|"stat") — supports children
- Text: content (required), variant ("body"|"heading"|"label"|"caption"|"code"), color ("default"|"muted"|"accent"|"success"|"warning"|"danger")
- Badge: text (required), variant ("success"|"warning"|"danger"|"info")
- Separator: orientation ("horizontal"|"vertical")
- CodeBlock: code (required), language, title
- Progress: value (required), max (default 100), label, color ("default"|"success"|"warning"|"danger"|"info"), showValue (boolean)

DATA:
- Table: columns (array of {key, label, align?}), rows (array of records), maxRows, striped (boolean)

LISTS:
- ListItem: title (required), description, status ("active"|"inactive"|"error"|"loading"), indent (number)
- TreeItem: label (required), badge, childCount, defaultExpanded (boolean), indent — supports children

FEEDBACK:
- Alert: title (required), message, severity ("info"|"success"|"warning"|"error")

STATES:
- EmptyState: title, description
- LoadingState: variant ("spinner"|"skeleton"|"pulse"), lines (number)
- ErrorState: title, message

INPUT:
- Button: label (required), variant ("primary"|"secondary"|"destructive"|"ghost"), size ("sm"|"md"|"lg"), disabled (boolean)
- Input: label, placeholder, type ("text"|"number"|"email"|"password"|"url"), value, disabled, helperText
- Select: label, placeholder, options (array of {value, label, disabled?}), value
- SearchInput: placeholder, value, debounceMs
- TabBar: tabs (array of {id, label, group?, badge?}), activeTab, variant ("default"|"pill"|"underline")

RULES:
- Every element needs a unique string ID in the elements map
- root points to the top-level element ID
- children are arrays of element IDs (strings), not inline objects
- Use Grid with StatCards for dashboards
- Use Stack direction="vertical" as default layout
- Always wrap multiple items in a Stack or Grid
- Use Card to group related content with a title
- Use Table for structured data display
- Use Alert for status messages and notifications`;
}
