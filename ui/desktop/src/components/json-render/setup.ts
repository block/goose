import { defineCatalog } from '@json-render/core';
import { schema, defineRegistry, createRenderer } from '@json-render/react';
import {
  shadcnComponentDefinitions,
  shadcnComponents,
} from '@json-render/shadcn';

const PICKED_COMPONENTS = [
  'Card',
  'Stack',
  'Grid',
  'Separator',
  'Tabs',
  'Accordion',
  'Collapsible',
  'Table',
  'Heading',
  'Text',
  'Image',
  'Avatar',
  'Badge',
  'Alert',
  'Progress',
  'Skeleton',
  'Spinner',
  'Tooltip',
  'Input',
  'Textarea',
  'Select',
  'Checkbox',
  'Radio',
  'Switch',
  'Slider',
  'Button',
  'Link',
  'Toggle',
  'ToggleGroup',
  'ButtonGroup',
] as const;

function pick<T extends Record<string, unknown>>(
  source: T,
  keys: readonly string[]
): Partial<T> {
  const result: Record<string, unknown> = {};
  for (const key of keys) {
    if (key in source) result[key] = source[key];
  }
  return result as Partial<T>;
}

const pickedDefs = pick(shadcnComponentDefinitions, PICKED_COMPONENTS);
const pickedImpls = pick(shadcnComponents, PICKED_COMPONENTS);

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const catalog = defineCatalog(schema, pickedDefs as any);
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const registry = defineRegistry(catalog, pickedImpls as any);
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const CatalogRenderer = createRenderer(catalog, registry as any);
