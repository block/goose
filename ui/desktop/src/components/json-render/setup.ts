/**
 * Unified json-render setup — merges shadcn components (33) with goose
 * custom components (13) into a single registry.
 *
 * Both goose-ui and json-render code blocks use this CatalogRenderer.
 */
import { defineCatalog } from '@json-render/core';
import type { ComponentRenderProps } from '@json-render/react';
import { createRenderer, defineRegistry } from '@json-render/react';
import { schema } from '@json-render/react/schema';
import { shadcnComponents } from '@json-render/shadcn';
import { shadcnComponentDefinitions as catalogDefs } from '@json-render/shadcn/catalog';
import React from 'react';

import { gooseComponents } from '../ui/design-system/goose-components';

// ─── shadcn component keys ───────────────────────────────────
export const SHADCN_COMPONENT_KEYS = [
  'Card',
  'Stack',
  'Grid',
  'Separator',
  'Tabs',
  'Accordion',
  'Collapsible',
  'Dialog',
  'Drawer',
  'Popover',
  'Tooltip',
  'DropdownMenu',
  'Heading',
  'Text',
  'Image',
  'Avatar',
  'Badge',
  'Alert',
  'Table',
  'Carousel',
  'Progress',
  'Skeleton',
  'Spinner',
  'Input',
  'Textarea',
  'Select',
  'Checkbox',
  'Switch',
  'Slider',
  'Button',
  'Toggle',
  'Link',
  'Pagination',
] as const;

// ─── Goose DS overlay keys (must not overlap shadcn) ──────────
export const GOOSE_CUSTOM_COMPONENT_KEYS = [
  'PageHeader',
  'DataCard',
  'StatCard',
  'DataTable',
  'ListItem',
  'TreeItem',
  'EmptyState',
  'LoadingState',
  'ErrorState',
  'SearchInput',
  'TabBar',
  'CodeBlock',
  'Chart',
  'CardGrid',
] as const;

const SHADCN_KEYS = SHADCN_COMPONENT_KEYS;
const GOOSE_CUSTOM_KEYS = GOOSE_CUSTOM_COMPONENT_KEYS;

function pick<T extends Record<string, unknown>>(obj: T, keys: readonly string[]): Partial<T> {
  const result: Partial<T> = {};
  for (const key of keys) {
    if (key in obj) {
      (result as Record<string, unknown>)[key] = obj[key];
    }
  }
  return result;
}

const pickedDefs = pick(catalogDefs, SHADCN_KEYS);

// @ts-expect-error — zod v3/v4 type mismatch between project root and @json-render internals
const catalog = defineCatalog(schema, { components: pickedDefs });

/**
 * Adapt shadcn components to work with @json-render/react's Renderer.
 *
 * shadcn components expect: ({ props, children }) => JSX
 * Renderer passes: ({ element, children, emit, on, bindings, loading }) => JSX
 *
 * This adapter extracts element.props and forwards it.
 */
function mergeClassName(existing: unknown, add: string) {
  if (typeof existing === 'string' && existing.trim().length > 0) return `${existing} ${add}`;
  return add;
}

const FORCE_FULL_WIDTH = new Set<string>(['Tabs', 'Card', 'Table', 'Grid', 'Stack']);

function adaptShadcnComponents(
  components: Record<
    string,
    React.ComponentType<{ props: Record<string, unknown>; children?: React.ReactNode }>
  >
): Record<string, React.ComponentType<ComponentRenderProps>> {
  const adapted: Record<string, React.ComponentType<ComponentRenderProps>> = {};
  for (const [name, ShadcnComponent] of Object.entries(components)) {
    const AdaptedComponent = ({ element, children }: ComponentRenderProps) => {
      const baseProps = FORCE_FULL_WIDTH.has(name)
        ? { ...element.props, className: mergeClassName(element.props.className, 'w-full min-w-0') }
        : element.props;

      const props = (() => {
        if (name !== 'Card') return baseProps;

        // GenUI runs inside a chat-width surface. The shadcn Card "maxWidth" defaults
        // commonly produce cramped dashboards (e.g. max-w-md + mx-auto). Make the
        // default *and* the common "lg" choice expand to full width.
        const maxWidth = (element.props as { maxWidth?: unknown }).maxWidth;
        const centered = (element.props as { centered?: unknown }).centered;

        const effectiveMaxWidth = maxWidth === 'lg' || maxWidth == null ? 'full' : maxWidth;
        const effectiveCentered = effectiveMaxWidth === 'full' ? false : centered;

        return {
          ...baseProps,
          maxWidth: effectiveMaxWidth,
          ...(effectiveCentered === undefined ? {} : { centered: effectiveCentered }),
        };
      })();

      return React.createElement(ShadcnComponent, { props, children });
    };
    AdaptedComponent.displayName = `Adapted${name}`;
    adapted[name] = AdaptedComponent;
  }
  return adapted;
}

const pickedShadcn = pick(shadcnComponents, SHADCN_KEYS) as Record<
  string,
  React.ComponentType<{ props: Record<string, unknown>; children?: React.ReactNode }>
>;
const adaptedShadcn = adaptShadcnComponents(pickedShadcn);

function assertRegistryInvariant() {
  const shadcn = new Set<string>(SHADCN_KEYS);
  const overlap = GOOSE_CUSTOM_KEYS.filter((k) => shadcn.has(k));
  if (overlap.length > 0) {
    throw new Error(
      `json-render registry invariant violated: Goose DS overrides shadcn component(s): ${overlap.join(
        ', '
      )}`
    );
  }
}

if (import.meta.env.MODE !== 'production') {
  assertRegistryInvariant();
}

// Pick goose custom components — already implement ComponentRenderProps ({ element, children, emit })
const pickedGoose = pick(gooseComponents, GOOSE_CUSTOM_KEYS) as Record<
  string,
  React.ComponentType<ComponentRenderProps>
>;

// Merge: shadcn (adapted) + goose custom (native)
const mergedComponents: Record<string, React.ComponentType<ComponentRenderProps>> = {
  ...adaptedShadcn,
  ...pickedGoose,
};

// @ts-expect-error — zod v3/v4 type mismatch
const registry = defineRegistry(schema, { components: mergedComponents });

// @ts-expect-error — zod v3/v4 type mismatch
export const CatalogRenderer = createRenderer(catalog, mergedComponents);

export const catalogPrompt = catalog.prompt();
export { catalog, registry };
