import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import JsonRenderBlock from './JsonRenderBlock';

// ---------------------------------------------------------------------------
// Fixtures — extracted / derived from diagnostics_20260220_28 session.json
// (real AI output from a goose session that produced a component dashboard)
// ---------------------------------------------------------------------------

/**
 * JSONL streaming spec (real format from AI output).
 * Uses RFC 6902 JSON-Patch-style ops processed by createSpecStreamCompiler.
 * Subset of the 129-line dashboard spec, trimmed to a renderable Card+Heading.
 */
const JSONL_CARD_SPEC = [
  '{"op":"add","path":"/root","value":"wrapper"}',
  '{"op":"add","path":"/elements/wrapper","value":{"type":"Stack","props":{"direction":"vertical","gap":"md"},"children":["card"]}}',
  '{"op":"add","path":"/elements/card","value":{"type":"Card","props":{"title":"Total Components","description":"TSX & TS files"},"children":["card-val"]}}',
  '{"op":"add","path":"/elements/card-val","value":{"type":"Heading","props":{"text":"339","level":"h1"},"children":[]}}',
].join('\n');

/**
 * JSONL with multiple component types — mirrors the real dashboard header.
 * Tests Stack, Grid, Card, Heading, and Text components together.
 */
const JSONL_DASHBOARD_HEADER = [
  '{"op":"add","path":"/root","value":"dashboard"}',
  '{"op":"add","path":"/elements/dashboard","value":{"type":"Stack","props":{"direction":"vertical","gap":"lg"},"children":["header","stats-row"]}}',
  '{"op":"add","path":"/elements/header","value":{"type":"Stack","props":{"direction":"vertical","gap":"sm"},"children":["title","subtitle"]}}',
  '{"op":"add","path":"/elements/title","value":{"type":"Heading","props":{"text":"Goose Desktop Dashboard","level":"h1"},"children":[]}}',
  '{"op":"add","path":"/elements/subtitle","value":{"type":"Text","props":{"text":"Mapping component files across domains","variant":"lead"},"children":[]}}',
  '{"op":"add","path":"/elements/stats-row","value":{"type":"Grid","props":{"columns":2,"gap":"md"},"children":["stat-a","stat-b"]}}',
  '{"op":"add","path":"/elements/stat-a","value":{"type":"Card","props":{"title":"Total","description":"All files"},"children":["stat-a-val"]}}',
  '{"op":"add","path":"/elements/stat-a-val","value":{"type":"Heading","props":{"text":"339","level":"h2"},"children":[]}}',
  '{"op":"add","path":"/elements/stat-b","value":{"type":"Card","props":{"title":"Domains","description":"Categories"},"children":["stat-b-val"]}}',
  '{"op":"add","path":"/elements/stat-b-val","value":{"type":"Heading","props":{"text":"30","level":"h2"},"children":[]}}',
].join('\n');

/**
 * Nested JSON tree spec (what an LLM naturally produces before JSONL training).
 * JsonRenderBlock should detect this and call nestedToFlat().
 */
const NESTED_SPEC = JSON.stringify({
  root: {
    type: 'Stack',
    props: { direction: 'vertical', gap: 'md' },
    children: [
      {
        type: 'Heading',
        props: { text: 'Hello from nested spec', level: 'h2' },
        children: [],
      },
      {
        type: 'Text',
        props: { text: 'This was converted from nested tree to flat format' },
        children: [],
      },
    ],
  },
});

/**
 * Already-flat spec — root is a string, elements is a map.
 * This format is what the Renderer natively expects.
 */
const FLAT_SPEC = JSON.stringify({
  root: 'main',
  elements: {
    main: {
      type: 'Card',
      props: { title: 'Pre-flattened', description: 'Already in flat format' },
      children: ['inner'],
    },
    inner: {
      type: 'Text',
      props: { text: 'Content inside a flat-format card' },
      children: [],
    },
  },
});

/**
 * Nested spec with state — tests that state is preserved during nestedToFlat.
 */
const NESTED_WITH_STATE = JSON.stringify({
  root: {
    type: 'Text',
    props: { text: 'Stateful component' },
    children: [],
  },
  state: { activeTab: 'overview' },
});

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('JsonRenderBlock', () => {
  describe('JSONL format (streaming patches from real AI output)', () => {
    it('renders a simple Card from JSONL patches', () => {
      const { container } = render(<JsonRenderBlock spec={JSONL_CARD_SPEC} />);

      expect(container.querySelector('.json-render-block')).toBeInTheDocument();
      expect(screen.getByText('Total Components')).toBeInTheDocument();
      expect(screen.getByText('339')).toBeInTheDocument();
    });

    it('renders a multi-component dashboard header from JSONL', () => {
      const { container } = render(<JsonRenderBlock spec={JSONL_DASHBOARD_HEADER} />);

      expect(container.querySelector('.json-render-block')).toBeInTheDocument();
      expect(screen.getByText('Goose Desktop Dashboard')).toBeInTheDocument();
      expect(screen.getByText('Mapping component files across domains')).toBeInTheDocument();
      expect(screen.getByText('339')).toBeInTheDocument();
      expect(screen.getByText('30')).toBeInTheDocument();
      expect(screen.getByText('Total')).toBeInTheDocument();
      expect(screen.getByText('Domains')).toBeInTheDocument();
    });

    it('renders nothing for empty JSONL', () => {
      const { container } = render(<JsonRenderBlock spec="" />);
      expect(container.querySelector('.json-render-block')).not.toBeInTheDocument();
    });
  });

  describe('nested JSON tree format', () => {
    it('renders a nested tree spec via nestedToFlat conversion', () => {
      const { container } = render(<JsonRenderBlock spec={NESTED_SPEC} />);

      expect(container.querySelector('.json-render-block')).toBeInTheDocument();
      expect(screen.getByText('Hello from nested spec')).toBeInTheDocument();
      expect(
        screen.getByText('This was converted from nested tree to flat format')
      ).toBeInTheDocument();
    });

    it('preserves state from nested spec during conversion', () => {
      const { container } = render(<JsonRenderBlock spec={NESTED_WITH_STATE} />);

      expect(container.querySelector('.json-render-block')).toBeInTheDocument();
      expect(screen.getByText('Stateful component')).toBeInTheDocument();
    });
  });

  describe('already-flat spec format', () => {
    it('renders a pre-flattened spec directly', () => {
      const { container } = render(<JsonRenderBlock spec={FLAT_SPEC} />);

      expect(container.querySelector('.json-render-block')).toBeInTheDocument();
      expect(screen.getByText('Pre-flattened')).toBeInTheDocument();
      expect(screen.getByText('Content inside a flat-format card')).toBeInTheDocument();
    });
  });

  describe('error handling', () => {
    it('shows error for invalid JSON', () => {
      render(<JsonRenderBlock spec="not valid json at all" />);

      expect(screen.getByText(/json-render error/)).toBeInTheDocument();
    });

    it('shows error for JSON that is not a valid spec', () => {
      render(<JsonRenderBlock spec='{"foo": "bar"}' />);

      expect(screen.getByText(/json-render error/)).toBeInTheDocument();
    });

    it('shows error for JSONL with missing root', () => {
      const noRoot = [
        '{"op":"add","path":"/elements/x","value":{"type":"Text","props":{"text":"orphan"},"children":[]}}',
      ].join('\n');

      render(<JsonRenderBlock spec={noRoot} />);

      expect(screen.getByText(/json-render error/)).toBeInTheDocument();
    });
  });

  describe('format detection', () => {
    it('detects JSONL by first-line op field', () => {
      // JSONL starts with {"op": ...}
      const { container } = render(<JsonRenderBlock spec={JSONL_CARD_SPEC} />);
      expect(container.querySelector('.json-render-block')).toBeInTheDocument();
    });

    it('detects nested JSON by root being an object with type', () => {
      const { container } = render(<JsonRenderBlock spec={NESTED_SPEC} />);
      expect(container.querySelector('.json-render-block')).toBeInTheDocument();
    });

    it('detects flat JSON by root being a string', () => {
      const { container } = render(<JsonRenderBlock spec={FLAT_SPEC} />);
      expect(container.querySelector('.json-render-block')).toBeInTheDocument();
    });
  });

  describe('component rendering fidelity (adaptComponents bridge)', () => {
    it('renders Stack with correct direction', () => {
      const spec = [
        '{"op":"add","path":"/root","value":"s"}',
        '{"op":"add","path":"/elements/s","value":{"type":"Stack","props":{"direction":"horizontal","gap":"lg"},"children":["t"]}}',
        '{"op":"add","path":"/elements/t","value":{"type":"Text","props":{"text":"In a horizontal stack"},"children":[]}}',
      ].join('\n');

      render(<JsonRenderBlock spec={spec} />);
      expect(screen.getByText('In a horizontal stack')).toBeInTheDocument();
    });

    it('renders Badge component', () => {
      const spec = [
        '{"op":"add","path":"/root","value":"b"}',
        '{"op":"add","path":"/elements/b","value":{"type":"Badge","props":{"text":"Beta","variant":"default"},"children":[]}}',
      ].join('\n');

      render(<JsonRenderBlock spec={spec} />);
      expect(screen.getByText('Beta')).toBeInTheDocument();
    });

    it('renders Text with variant', () => {
      const spec = JSON.stringify({
        root: {
          type: 'Text',
          props: { text: 'Lead paragraph text', variant: 'lead' },
          children: [],
        },
      });

      render(<JsonRenderBlock spec={spec} />);
      expect(screen.getByText('Lead paragraph text')).toBeInTheDocument();
    });

    it('renders Separator without crashing', () => {
      const spec = [
        '{"op":"add","path":"/root","value":"wrap"}',
        '{"op":"add","path":"/elements/wrap","value":{"type":"Stack","props":{"direction":"vertical","gap":"sm"},"children":["t1","sep","t2"]}}',
        '{"op":"add","path":"/elements/t1","value":{"type":"Text","props":{"text":"Above"},"children":[]}}',
        '{"op":"add","path":"/elements/sep","value":{"type":"Separator","props":{"orientation":"horizontal"},"children":[]}}',
        '{"op":"add","path":"/elements/t2","value":{"type":"Text","props":{"text":"Below"},"children":[]}}',
      ].join('\n');

      render(<JsonRenderBlock spec={spec} />);
      expect(screen.getByText('Above')).toBeInTheDocument();
      expect(screen.getByText('Below')).toBeInTheDocument();
    });

    it('renders Progress bar', () => {
      const spec = [
        '{"op":"add","path":"/root","value":"p"}',
        '{"op":"add","path":"/elements/p","value":{"type":"Progress","props":{"value":75,"max":100,"label":"Loading..."},"children":[]}}',
      ].join('\n');

      render(<JsonRenderBlock spec={spec} />);
      expect(screen.getByText('Loading...')).toBeInTheDocument();
    });

    it('renders Alert component', () => {
      const spec = JSON.stringify({
        root: {
          type: 'Alert',
          props: { title: 'Heads up!', variant: 'default' },
          children: [],
        },
      });

      render(<JsonRenderBlock spec={spec} />);
      expect(screen.getByText('Heads up!')).toBeInTheDocument();
      expect(screen.getByRole('alert')).toBeInTheDocument();
    });
  });

  describe('real-world AI output (diagnostics_20260220_28 derived)', () => {
    it('renders the full dashboard header section from a real session', () => {
      // This is the exact JSONL the AI produced for the dashboard header,
      // extracted from diagnostics_20260220_28/session.json message[14]
      const realJsonl = [
        '{"op":"add","path":"/root","value":"dashboard"}',
        '{"op":"add","path":"/elements/dashboard","value":{"type":"Stack","props":{"direction":"vertical","gap":"lg"},"children":["header","stats-row"]}}',
        '{"op":"add","path":"/elements/header","value":{"type":"Stack","props":{"direction":"vertical","gap":"sm"},"children":["title","subtitle"]}}',
        '{"op":"add","path":"/elements/title","value":{"type":"Heading","props":{"text":"\\ud83e\\udd86 Goose Desktop \\u2014 Component Diversity Dashboard","level":"h1"},"children":[]}}',
        '{"op":"add","path":"/elements/subtitle","value":{"type":"Text","props":{"text":"Mapping 339 component files across 30+ domains in ui/desktop/src/components","variant":"lead"},"children":[]}}',
        '{"op":"add","path":"/elements/stats-row","value":{"type":"Grid","props":{"columns":4,"gap":"md"},"children":["stat-total","stat-domains","stat-ui","stat-settings"]}}',
        '{"op":"add","path":"/elements/stat-total","value":{"type":"Card","props":{"title":"Total Components","description":"TSX & TS files"},"children":["stat-total-val"]}}',
        '{"op":"add","path":"/elements/stat-total-val","value":{"type":"Heading","props":{"text":"339","level":"h1"},"children":[]}}',
        '{"op":"add","path":"/elements/stat-domains","value":{"type":"Card","props":{"title":"Component Domains","description":"Top-level categories"},"children":["stat-domains-val"]}}',
        '{"op":"add","path":"/elements/stat-domains-val","value":{"type":"Heading","props":{"text":"30","level":"h1"},"children":[]}}',
        '{"op":"add","path":"/elements/stat-ui","value":{"type":"Card","props":{"title":"UI Primitives","description":"Atoms + Molecules + Design System"},"children":["stat-ui-val"]}}',
        '{"op":"add","path":"/elements/stat-ui-val","value":{"type":"Heading","props":{"text":"45","level":"h1"},"children":[]}}',
        '{"op":"add","path":"/elements/stat-settings","value":{"type":"Card","props":{"title":"Settings Components","description":"Provider, Model, Extension configs"},"children":["stat-settings-val"]}}',
        '{"op":"add","path":"/elements/stat-settings-val","value":{"type":"Heading","props":{"text":"74","level":"h1"},"children":[]}}',
      ].join('\n');

      const { container } = render(<JsonRenderBlock spec={realJsonl} />);

      expect(container.querySelector('.json-render-block')).toBeInTheDocument();

      // Dashboard title and subtitle
      expect(screen.getByText(/Goose Desktop/)).toBeInTheDocument();
      expect(screen.getByText(/Mapping 339 component files/)).toBeInTheDocument();

      // Stats cards
      expect(screen.getByText('Total Components')).toBeInTheDocument();
      expect(screen.getByText('Component Domains')).toBeInTheDocument();
      expect(screen.getByText('UI Primitives')).toBeInTheDocument();
      expect(screen.getByText('Settings Components')).toBeInTheDocument();

      // Values
      expect(screen.getAllByText('339')).toHaveLength(1);
      expect(screen.getAllByText('30')).toHaveLength(1);
      expect(screen.getAllByText('45')).toHaveLength(1);
      expect(screen.getAllByText('74')).toHaveLength(1);
    });
  });
});
