/**
 * Goose Generative UI Component Catalog
 *
 * Defines the set of components that AI can generate as structured JSON.
 * Uses json-render with Zod schemas to constrain AI output to valid UI specs.
 *
 * Note: json-render uses zod 4 internally, while the project uses zod 3.
 * The runtime is compatible but TypeScript types differ. We use ts-ignore
 * for the catalog definition since it's validated at runtime by json-render.
 */
import { defineCatalog } from '@json-render/core';
import { schema } from '@json-render/react';
import { z } from 'zod';

// @ts-ignore — zod 3/4 type mismatch (runtime compatible, TS types differ)
export const gooseCatalog = defineCatalog(schema, {
  components: {
    // --- Layout ---
    Grid: {
      // @ts-ignore
      props: z.object({
        columns: z.number().min(1).max(4).default(2),
        gap: z.enum(['sm', 'md', 'lg']).default('md'),
      }),
      slots: ['default'],
      description: 'Grid layout container for arranging child components',
    },

    Section: {
      // @ts-ignore
      props: z.object({
        title: z.string(),
        subtitle: z.string().optional(),
        collapsible: z.boolean().default(false),
      }),
      slots: ['default'],
      description: 'Titled section container with optional subtitle',
    },

    // --- Data Display ---
    MetricCard: {
      // @ts-ignore
      props: z.object({
        label: z.string(),
        value: z.string(),
        delta: z.string().optional(),
        deltaType: z.enum(['positive', 'negative', 'neutral']).optional(),
        icon: z.string().optional(),
        description: z.string().optional(),
      }),
      description: 'KPI metric card showing a value with optional trend delta',
    },

    DataTable: {
      // @ts-ignore
      props: z.object({
        columns: z.array(z.object({
          key: z.string(),
          label: z.string(),
          align: z.enum(['left', 'center', 'right']).default('left'),
        })),
        rows: z.array(z.record(z.string(), z.union([z.string(), z.number(), z.boolean()]))),
        sortable: z.boolean().default(false),
        maxRows: z.number().default(10),
      }),
      description: 'Data table with columns and rows, optional sorting',
    },

    Chart: {
      // @ts-ignore
      props: z.object({
        type: z.enum(['bar', 'line', 'area', 'pie']),
        data: z.array(z.record(z.string(), z.union([z.string(), z.number()]))),
        xKey: z.string(),
        yKeys: z.array(z.string()),
        height: z.number().default(300),
        title: z.string().optional(),
        colors: z.array(z.string()).optional(),
      }),
      description: 'Chart visualization — bar, line, area, or pie chart',
    },

    ProgressBar: {
      // @ts-ignore
      props: z.object({
        label: z.string(),
        value: z.number().min(0).max(100),
        color: z.enum(['green', 'yellow', 'red', 'blue']).default('blue'),
      }),
      description: 'Horizontal progress bar with label and percentage',
    },

    // --- Status & Alerts ---
    StatusBadge: {
      // @ts-ignore
      props: z.object({
        label: z.string(),
        status: z.enum(['success', 'warning', 'error', 'info', 'neutral']),
      }),
      description: 'Colored badge indicating status',
    },

    AlertCard: {
      // @ts-ignore
      props: z.object({
        title: z.string(),
        message: z.string(),
        severity: z.enum(['info', 'warning', 'error', 'success']),
        dismissible: z.boolean().default(false),
      }),
      description: 'Alert notification card with severity level',
    },

    // --- Content ---
    Text: {
      // @ts-ignore
      props: z.object({
        content: z.string(),
        variant: z.enum(['body', 'heading', 'caption', 'code']).default('body'),
      }),
      description: 'Text content with style variant',
    },

    CodeBlock: {
      // @ts-ignore
      props: z.object({
        code: z.string(),
        language: z.string().default('text'),
        title: z.string().optional(),
      }),
      description: 'Formatted code block with syntax highlighting',
    },

    List: {
      // @ts-ignore
      props: z.object({
        items: z.array(z.object({
          label: z.string(),
          description: z.string().optional(),
          icon: z.string().optional(),
          status: z.enum(['success', 'warning', 'error', 'info', 'neutral']).optional(),
        })),
        ordered: z.boolean().default(false),
      }),
      description: 'List of items with optional descriptions and status icons',
    },

    // --- Interactive ---
    ActionButton: {
      // @ts-ignore
      props: z.object({
        label: z.string(),
        action: z.string(),
        variant: z.enum(['primary', 'secondary', 'destructive', 'ghost']).default('primary'),
        icon: z.string().optional(),
      }),
      description: 'Button that triggers a named action',
    },

    // --- Goose-Specific ---
    SessionCard: {
      // @ts-ignore
      props: z.object({
        sessionId: z.string(),
        name: z.string(),
        provider: z.string().optional(),
        messageCount: z.number().optional(),
        tokenCount: z.number().optional(),
        createdAt: z.string().optional(),
      }),
      description: 'Summary card for a Goose session',
    },

    ToolResult: {
      // @ts-ignore
      props: z.object({
        toolName: z.string(),
        status: z.enum(['success', 'error']),
        duration: z.string().optional(),
        output: z.string().optional(),
      }),
      description: 'Display result of a tool execution',
    },

    EvalResult: {
      // @ts-ignore
      props: z.object({
        datasetName: z.string(),
        accuracy: z.number(),
        agentAccuracy: z.number().optional(),
        modeAccuracy: z.number().optional(),
        testCount: z.number(),
        passCount: z.number(),
        failCount: z.number(),
      }),
      description: 'Evaluation run result summary',
    },
  },

  actions: {
    navigate: {
      // @ts-ignore
      params: z.object({ path: z.string() }),
      description: 'Navigate to an app page',
    },
    create_session: {
      // @ts-ignore
      params: z.object({ message: z.string().optional() }),
      description: 'Create a new chat session',
    },
    run_eval: {
      // @ts-ignore
      params: z.object({ datasetId: z.string() }),
      description: 'Run an evaluation on a dataset',
    },
    open_session: {
      // @ts-ignore
      params: z.object({ sessionId: z.string() }),
      description: 'Open an existing session',
    },
    install_extension: {
      // @ts-ignore
      params: z.object({ name: z.string() }),
      description: 'Install a tool extension',
    },
    run_recipe: {
      // @ts-ignore
      params: z.object({ recipeId: z.string() }),
      description: 'Run a workflow recipe',
    },
    dismiss: {
      description: 'Dismiss the generative panel',
    },
  },
});

// Export the catalog prompt for AI system prompts
export const catalogPrompt = gooseCatalog.prompt();
