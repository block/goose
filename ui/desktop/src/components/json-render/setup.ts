import { defineCatalog } from '@json-render/core';
import { createRenderer, defineRegistry } from '@json-render/react';
import { schema } from '@json-render/react/schema';
import {
  shadcnComponents,
} from '@json-render/shadcn';
import { shadcnComponentDefinitions as catalogDefs } from '@json-render/shadcn/catalog';

// Pick a subset of components for the catalog
type PickedKey =
  | 'Card'
  | 'Stack'
  | 'Grid'
  | 'Separator'
  | 'Tabs'
  | 'Accordion'
  | 'Collapsible'
  | 'Dialog'
  | 'Drawer'
  | 'Popover'
  | 'Tooltip'
  | 'DropdownMenu'
  | 'Heading'
  | 'Text'
  | 'Image'
  | 'Avatar'
  | 'Badge'
  | 'Alert'
  | 'Table'
  | 'Carousel'
  | 'Progress'
  | 'Skeleton'
  | 'Spinner'
  | 'Input'
  | 'Textarea'
  | 'Select'
  | 'Checkbox'
  | 'Switch'
  | 'Slider'
  | 'Button'
  | 'Toggle'
  | 'Link'
  | 'Pagination';

function pick<T extends Record<string, unknown>, K extends keyof T>(
  obj: T,
  keys: K[],
): Pick<T, K> {
  const result = {} as Pick<T, K>;
  for (const key of keys) {
    if (key in obj) result[key] = obj[key];
  }
  return result;
}

const COMPONENT_KEYS: PickedKey[] = [
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
];

const pickedDefs = pick(catalogDefs, COMPONENT_KEYS);
// @ts-expect-error — zod v3/v4 type mismatch between project root and @json-render internals
const catalog = defineCatalog(schema, { components: pickedDefs });

const components = pick(shadcnComponents, COMPONENT_KEYS);

// @ts-expect-error — zod v3/v4 type mismatch
const registry = defineRegistry(catalog, components);

// Create the renderer from catalog + components
// @ts-expect-error — zod v3/v4 type mismatch
export const CatalogRenderer = createRenderer(catalog, components);

// Export the catalog prompt for system prompt injection
export const catalogPrompt: string = catalog.prompt();

export { catalog, registry };
