import { defineCatalog } from '@json-render/core';
import { createRenderer, defineRegistry } from '@json-render/react';
import { schema } from '@json-render/react/schema';
import { shadcnComponents } from '@json-render/shadcn';
import { shadcnComponentDefinitions as catalogDefs } from '@json-render/shadcn/catalog';
import React from 'react';

// Pick a subset of components for the catalog
const COMPONENT_KEYS = [
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

function pick<T extends Record<string, unknown>>(
  obj: T,
  keys: readonly string[]
): Partial<T> {
  const result: Partial<T> = {};
  for (const key of keys) {
    if (key in obj) {
      (result as Record<string, unknown>)[key] = obj[key];
    }
  }
  return result;
}

const pickedDefs = pick(catalogDefs, COMPONENT_KEYS);

// @ts-expect-error — zod v3/v4 type mismatch between project root and @json-render internals
const catalog = defineCatalog(schema, { components: pickedDefs });

/**
 * Adapt shadcn components to work with @json-render/react's Renderer.
 *
 * The Renderer (ElementRenderer) renders each component as:
 *   <Component element={resolvedElement} emit={emit} on={on} bindings={...} loading={loading}>
 *     {children}
 *   </Component>
 *
 * So components receive ComponentRenderProps: { element, emit, on, bindings, loading, children }
 *
 * But @json-render/shadcn components expect BaseComponentProps:
 *   Component({ props: element.props, children, emit, on, bindings })
 *
 * This adapter bridges the two interfaces — identical to what defineRegistry() does internally.
 */
interface ElementLike {
  props: Record<string, unknown>;
  [key: string]: unknown;
}

interface ComponentRenderInput {
  element: ElementLike;
  children?: React.ReactNode;
  emit: (event: string) => void;
  on: (event: string) => unknown;
  bindings?: Record<string, string>;
  loading?: boolean;
}

function adaptComponents(
  components: Record<string, React.FC<{ props: Record<string, unknown>; children?: React.ReactNode }>>
): Record<string, React.FC<ComponentRenderInput>> {
  const adapted: Record<string, React.FC<ComponentRenderInput>> = {};
  for (const [name, Component] of Object.entries(components)) {
    adapted[name] = ({ element, children, emit, on, bindings, loading }: ComponentRenderInput) => {
      return React.createElement(Component, {
        props: element.props,
        children,
        emit,
        on,
        bindings,
        loading,
      });
    };
    adapted[name].displayName = `Adapted${name}`;
  }
  return adapted;
}

const pickedComponents = pick(shadcnComponents, COMPONENT_KEYS) as Record<
  string,
  React.FC<{ props: Record<string, unknown>; children?: React.ReactNode }>
>;
const adaptedComponents = adaptComponents(pickedComponents);

// @ts-expect-error — zod v3/v4 type mismatch
const registry = defineRegistry(schema, { components: adaptedComponents });

// @ts-expect-error — zod v3/v4 type mismatch
export const CatalogRenderer = createRenderer(catalog, adaptedComponents);

export const catalogPrompt = catalog.prompt();
export { catalog, registry };
