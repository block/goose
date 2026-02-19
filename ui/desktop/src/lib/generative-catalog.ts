/**
 * Goose Generative UI Component Catalog
 *
 * Defines the set of components that AI can generate as structured JSON.
 * Uses json-render with Zod schemas to constrain AI output to valid UI specs.
 *
 * Note: json-render uses zod 4 internally while project uses zod 3.
 * We use `as any` casts for schema definitions since they're validated at runtime.
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type AnySchema = any;

// We define schemas as plain objects to avoid zod 3/4 type conflicts.
// json-render validates these at runtime using its bundled zod 4.
import { defineCatalog } from '@json-render/core';
import { schema } from '@json-render/react';
import { z } from 'zod';

const catalogDef = {
  components: {
    Grid: {
      props: z.object({
        columns: z.number().min(1).max(4).default(2),
        gap: z.enum(['sm', 'md', 'lg']).default('md'),
      }) as AnySchema,
      slots: ['default'] as const,
      description: 'Grid layout container for arranging child components',
    },
    Section: {
      props: z.object({
        title: z.string(),
        subtitle: z.string().optional(),
        collapsible: z.boolean().default(false),
      }) as AnySchema,
      slots: ['default'] as const,
      description: 'Titled section container with optional subtitle',
    },
    MetricCard: {
      props: z.object({
        label: z.string(),
        value: z.string(),
        delta: z.string().optional(),
        deltaType: z.enum(['positive', 'negative', 'neutral']).optional(),
        icon: z.string().optional(),
        description: z.string().optional(),
      }) as AnySchema,
      description: 'KPI metric card showing a value with optional trend delta',
    },
    DataTable: {
      props: z.object({
        columns: z.array(
          z.object({
            key: z.string(),
            label: z.string(),
            align: z.enum(['left', 'center', 'right']).default('left'),
          })
        ),
        rows: z.array(z.record(z.string(), z.union([z.string(), z.number(), z.boolean()]))),
        sortable: z.boolean().default(false),
        maxRows: z.number().default(10),
      }) as AnySchema,
      description: 'Data table with columns and rows, optional sorting',
    },
    Chart: {
      props: z.object({
        type: z.enum(['bar', 'line', 'area', 'pie']),
        data: z.array(z.record(z.string(), z.union([z.string(), z.number()]))),
        xKey: z.string(),
        yKeys: z.array(z.string()),
        height: z.number().default(300),
        title: z.string().optional(),
        colors: z.array(z.string()).optional(),
      }) as AnySchema,
      description: 'Chart visualization — bar, line, area, or pie chart',
    },
    ProgressBar: {
      props: z.object({
        label: z.string(),
        value: z.number().min(0).max(100),
        color: z.enum(['green', 'yellow', 'red', 'blue']).default('blue'),
      }) as AnySchema,
      description: 'Horizontal progress bar with label and percentage',
    },
    StatusBadge: {
      props: z.object({
        label: z.string(),
        status: z.enum(['success', 'warning', 'error', 'info', 'neutral']),
      }) as AnySchema,
      description: 'Colored badge indicating status',
    },
    AlertCard: {
      props: z.object({
        title: z.string(),
        message: z.string(),
        severity: z.enum(['info', 'warning', 'error', 'success']),
        dismissible: z.boolean().default(false),
      }) as AnySchema,
      description: 'Alert notification card with severity level',
    },
    Text: {
      props: z.object({
        content: z.string(),
        variant: z.enum(['body', 'heading', 'caption', 'code']).default('body'),
      }) as AnySchema,
      description: 'Text content with style variant',
    },
    CodeBlock: {
      props: z.object({
        code: z.string(),
        language: z.string().default('text'),
        title: z.string().optional(),
      }) as AnySchema,
      description: 'Formatted code block with syntax highlighting',
    },
    List: {
      props: z.object({
        items: z.array(
          z.object({
            label: z.string(),
            description: z.string().optional(),
            icon: z.string().optional(),
            status: z.enum(['success', 'warning', 'error', 'info', 'neutral']).optional(),
          })
        ),
        ordered: z.boolean().default(false),
      }) as AnySchema,
      description: 'List of items with optional descriptions and status icons',
    },
    ActionButton: {
      props: z.object({
        label: z.string(),
        action: z.string(),
        variant: z.enum(['primary', 'secondary', 'destructive', 'ghost']).default('primary'),
        icon: z.string().optional(),
      }) as AnySchema,
      description: 'Button that triggers a named action',
    },
    SessionCard: {
      props: z.object({
        sessionId: z.string(),
        name: z.string(),
        provider: z.string().optional(),
        messageCount: z.number().optional(),
        tokenCount: z.number().optional(),
        createdAt: z.string().optional(),
      }) as AnySchema,
      description: 'Summary card for a Goose session',
    },
    ToolResult: {
      props: z.object({
        toolName: z.string(),
        status: z.enum(['success', 'error']),
        duration: z.string().optional(),
        output: z.string().optional(),
      }) as AnySchema,
      description: 'Display result of a tool execution',
    },
    EvalResult: {
      props: z.object({
        datasetName: z.string(),
        accuracy: z.number(),
        agentAccuracy: z.number().optional(),
        modeAccuracy: z.number().optional(),
        testCount: z.number(),
        passCount: z.number(),
        failCount: z.number(),
      }) as AnySchema,
      description: 'Evaluation run result summary',
    },
  },
  actions: {
    navigate: {
      params: z.object({ path: z.string() }) as AnySchema,
      description: 'Navigate to an app page',
    },
    create_session: {
      params: z.object({ message: z.string().optional() }) as AnySchema,
      description: 'Create a new chat session',
    },
    run_eval: {
      params: z.object({ datasetId: z.string() }) as AnySchema,
      description: 'Run an evaluation on a dataset',
    },
    open_session: {
      params: z.object({ sessionId: z.string() }) as AnySchema,
      description: 'Open an existing session',
    },
    install_extension: {
      params: z.object({ name: z.string() }) as AnySchema,
      description: 'Install a tool extension',
    },
    run_recipe: {
      params: z.object({ recipeId: z.string() }) as AnySchema,
      description: 'Run a workflow recipe',
    },
    dismiss: {
      description: 'Dismiss the generative panel',
    },
  },
};

// @ts-expect-error — zod 3/4 type compatibility at the defineCatalog call level
export const gooseCatalog = defineCatalog(schema, catalogDef);

export const catalogPrompt = gooseCatalog.prompt();
