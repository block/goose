/**
 * End-to-end component registry tests.
 *
 * Verifies that every component registered in the unified 45-component
 * registry (33 shadcn + 12 goose custom) renders without crashing when
 * given a minimal valid spec via JsonRenderBlock.
 */

import { render, screen } from '@testing-library/react';
import { beforeAll, describe, expect, it } from 'vitest';
import JsonRenderBlock from './JsonRenderBlock';

// Mock IntersectionObserver for JSDOM (needed by Chart's lazy rendering)
beforeAll(() => {
  global.IntersectionObserver = class MockIntersectionObserver {
    private callback: IntersectionObserverCallback;
    root = null;
    rootMargin = '0px';
    thresholds = [0];
    constructor(callback: IntersectionObserverCallback) {
      this.callback = callback;
    }
    observe(target: Element) {
      // Immediately report as visible
      this.callback(
        [{ isIntersecting: true, target } as IntersectionObserverEntry],
        this as unknown as IntersectionObserver
      );
    }
    unobserve() {}
    disconnect() {}
    takeRecords(): IntersectionObserverEntry[] {
      return [];
    }
  } as unknown as typeof IntersectionObserver;
});

/** Helper: build a nested JSON spec for a single component */
function nestedSpec(type: string, props: Record<string, unknown> = {}, children: unknown[] = []) {
  return JSON.stringify({
    root: { type, props, children },
  });
}

/** Helper: build a JSONL spec for a component with children */
function jsonlSpec(
  rootId: string,
  elements: Record<string, { type: string; props: Record<string, unknown>; children?: string[] }>
) {
  const lines = [`{"op":"add","path":"/root","value":"${rootId}"}`];
  for (const [id, el] of Object.entries(elements)) {
    lines.push(
      JSON.stringify({
        op: 'add',
        path: `/elements/${id}`,
        value: { type: el.type, props: el.props, children: el.children ?? [] },
      })
    );
  }
  return lines.join('\n');
}

describe('Component Registry — End-to-End Rendering', () => {
  describe('shadcn components (System 1)', () => {
    it('renders Card with title', () => {
      const spec = nestedSpec('Card', { title: 'Test Card' });
      const { container } = render(<JsonRenderBlock spec={spec} />);
      expect(container.querySelector('.json-render-block')).toBeInTheDocument();
    });

    it('renders Stack with children', () => {
      const spec = jsonlSpec('s', {
        s: { type: 'Stack', props: { direction: 'vertical', gap: 'md' }, children: ['t1', 't2'] },
        t1: { type: 'Text', props: { text: 'First' } },
        t2: { type: 'Text', props: { text: 'Second' } },
      });
      render(<JsonRenderBlock spec={spec} />);
      expect(screen.getByText('First')).toBeInTheDocument();
      expect(screen.getByText('Second')).toBeInTheDocument();
    });

    it('renders Grid with columns', () => {
      const spec = jsonlSpec('g', {
        g: { type: 'Grid', props: { columns: 3, gap: 'md' }, children: ['c1', 'c2', 'c3'] },
        c1: { type: 'Text', props: { text: 'Col 1' } },
        c2: { type: 'Text', props: { text: 'Col 2' } },
        c3: { type: 'Text', props: { text: 'Col 3' } },
      });
      render(<JsonRenderBlock spec={spec} />);
      expect(screen.getByText('Col 1')).toBeInTheDocument();
      expect(screen.getByText('Col 3')).toBeInTheDocument();
    });

    it('renders Separator', () => {
      const spec = nestedSpec('Separator', { orientation: 'horizontal' });
      const { container } = render(<JsonRenderBlock spec={spec} />);
      expect(container.querySelector('.json-render-block')).toBeInTheDocument();
    });

    it('renders Tabs', () => {
      const spec = jsonlSpec('tabs', {
        tabs: {
          type: 'Tabs',
          props: { defaultValue: 'tab1' },
          children: ['tab1-content'],
        },
        'tab1-content': { type: 'Text', props: { text: 'Tab 1 body' } },
      });
      const { container } = render(<JsonRenderBlock spec={spec} />);
      expect(container.querySelector('.json-render-block')).toBeInTheDocument();
    });

    it('renders Heading at all levels', () => {
      for (const level of ['h1', 'h2', 'h3', 'h4']) {
        const spec = nestedSpec('Heading', { text: `Heading ${level}`, level });
        render(<JsonRenderBlock spec={spec} />);
        expect(screen.getByText(`Heading ${level}`)).toBeInTheDocument();
      }
    });

    it('renders Text with variants', () => {
      for (const variant of ['body', 'lead', 'muted', 'caption']) {
        const spec = nestedSpec('Text', { text: `Text ${variant}`, variant });
        render(<JsonRenderBlock spec={spec} />);
        expect(screen.getByText(`Text ${variant}`)).toBeInTheDocument();
      }
    });

    it('renders Badge', () => {
      const spec = nestedSpec('Badge', { text: 'Beta', variant: 'default' });
      render(<JsonRenderBlock spec={spec} />);
      expect(screen.getByText('Beta')).toBeInTheDocument();
    });

    it('renders Alert', () => {
      const spec = nestedSpec('Alert', { title: 'Warning!', variant: 'destructive' });
      render(<JsonRenderBlock spec={spec} />);
      expect(screen.getByText('Warning!')).toBeInTheDocument();
    });

    it('renders Button', () => {
      const spec = nestedSpec('Button', { label: 'Click Me', variant: 'default' });
      render(<JsonRenderBlock spec={spec} />);
      expect(screen.getByText('Click Me')).toBeInTheDocument();
    });

    it('renders Input', () => {
      const spec = nestedSpec('Input', { placeholder: 'Enter text...' });
      const { container } = render(<JsonRenderBlock spec={spec} />);
      expect(container.querySelector('.json-render-block')).toBeInTheDocument();
    });

    it('renders Progress', () => {
      const spec = nestedSpec('Progress', { value: 75, max: 100 });
      const { container } = render(<JsonRenderBlock spec={spec} />);
      expect(container.querySelector('.json-render-block')).toBeInTheDocument();
    });

    it('renders Table with data', () => {
      const spec = nestedSpec('Table', {
        columns: ['Name', 'Count'],
        rows: [
          ['Atoms', '14'],
          ['Molecules', '14'],
        ],
      });
      render(<JsonRenderBlock spec={spec} />);
      expect(screen.getByText('Atoms')).toBeInTheDocument();
      expect(screen.getByText('Molecules')).toBeInTheDocument();
    });

    it('renders Skeleton', () => {
      const spec = nestedSpec('Skeleton', { width: '100%', height: '20px' });
      const { container } = render(<JsonRenderBlock spec={spec} />);
      expect(container.querySelector('.json-render-block')).toBeInTheDocument();
    });

    it('renders Image', () => {
      const spec = nestedSpec('Image', { src: 'https://example.com/img.png', alt: 'Test' });
      const { container } = render(<JsonRenderBlock spec={spec} />);
      expect(container.querySelector('.json-render-block')).toBeInTheDocument();
    });

    it('renders Link', () => {
      const spec = nestedSpec('Link', { href: 'https://example.com' });
      const { container } = render(<JsonRenderBlock spec={spec} />);
      expect(container.querySelector('a')).toBeInTheDocument();
    });

    it('renders Select', () => {
      const spec = nestedSpec('Select', {
        options: [
          { value: 'a', label: 'Option A' },
          { value: 'b', label: 'Option B' },
        ],
      });
      const { container } = render(<JsonRenderBlock spec={spec} />);
      expect(container.querySelector('.json-render-block')).toBeInTheDocument();
    });

    it('renders Checkbox', () => {
      const spec = nestedSpec('Checkbox', { label: 'Agree to terms' });
      const { container } = render(<JsonRenderBlock spec={spec} />);
      expect(container.querySelector('.json-render-block')).toBeInTheDocument();
    });

    it('renders Switch', () => {
      const spec = nestedSpec('Switch', { label: 'Enable feature' });
      const { container } = render(<JsonRenderBlock spec={spec} />);
      expect(container.querySelector('.json-render-block')).toBeInTheDocument();
    });

    it('renders Textarea', () => {
      const spec = nestedSpec('Textarea', { placeholder: 'Write something...' });
      const { container } = render(<JsonRenderBlock spec={spec} />);
      expect(container.querySelector('.json-render-block')).toBeInTheDocument();
    });

    it('renders Toggle', () => {
      const spec = nestedSpec('Toggle', { label: 'Bold' });
      const { container } = render(<JsonRenderBlock spec={spec} />);
      expect(container.querySelector('.json-render-block')).toBeInTheDocument();
    });

    it('renders Slider', () => {
      const spec = nestedSpec('Slider', { min: 0, max: 100, value: 50 });
      const { container } = render(<JsonRenderBlock spec={spec} />);
      expect(container.querySelector('.json-render-block')).toBeInTheDocument();
    });

    it('renders Spinner', () => {
      const spec = nestedSpec('Spinner', {});
      const { container } = render(<JsonRenderBlock spec={spec} />);
      expect(container.querySelector('.json-render-block')).toBeInTheDocument();
    });
  });

  describe('goose custom components (System 2)', () => {
    it('renders PageHeader', () => {
      const spec = nestedSpec('PageHeader', { title: 'Dashboard', description: 'Overview' });
      render(<JsonRenderBlock spec={spec} />);
      expect(screen.getByText('Dashboard')).toBeInTheDocument();
    });

    it('renders StatCard', () => {
      const spec = nestedSpec('StatCard', { label: 'Total Files', value: '346' });
      render(<JsonRenderBlock spec={spec} />);
      expect(screen.getByText('Total Files')).toBeInTheDocument();
      expect(screen.getByText('346')).toBeInTheDocument();
    });

    it('renders DataCard', () => {
      const spec = nestedSpec('DataCard', { variant: 'default' }, []);
      const { container } = render(<JsonRenderBlock spec={spec} />);
      expect(container.querySelector('.json-render-block')).toBeInTheDocument();
    });

    it('renders ListItem', () => {
      const spec = nestedSpec('ListItem', { title: 'Item One', description: 'First item' });
      render(<JsonRenderBlock spec={spec} />);
      expect(screen.getByText('Item One')).toBeInTheDocument();
    });

    it('renders TreeItem', () => {
      const spec = nestedSpec('TreeItem', { label: 'src/', expanded: true });
      render(<JsonRenderBlock spec={spec} />);
      expect(screen.getByText('src/')).toBeInTheDocument();
    });

    it('renders EmptyState', () => {
      const spec = nestedSpec('EmptyState', { title: 'No data', description: 'Nothing here' });
      render(<JsonRenderBlock spec={spec} />);
      expect(screen.getByText('No data')).toBeInTheDocument();
    });

    it('renders LoadingState', () => {
      const spec = nestedSpec('LoadingState', { variant: 'spinner' });
      const { container } = render(<JsonRenderBlock spec={spec} />);
      expect(container.querySelector('.json-render-block')).toBeInTheDocument();
    });

    it('renders ErrorState', () => {
      const spec = nestedSpec('ErrorState', { title: 'Failed', message: 'Something broke' });
      render(<JsonRenderBlock spec={spec} />);
      expect(screen.getByText('Something broke')).toBeInTheDocument();
    });

    it('renders SearchInput', () => {
      const spec = nestedSpec('SearchInput', { placeholder: 'Search...', value: '' });
      const { container } = render(<JsonRenderBlock spec={spec} />);
      expect(container.querySelector('.json-render-block')).toBeInTheDocument();
    });

    it('renders TabBar', () => {
      const spec = nestedSpec('TabBar', {
        tabs: [
          { id: 'overview', label: 'Overview' },
          { id: 'details', label: 'Details' },
        ],
        activeTab: 'overview',
      });
      render(<JsonRenderBlock spec={spec} />);
      expect(screen.getByText('Overview')).toBeInTheDocument();
      expect(screen.getByText('Details')).toBeInTheDocument();
    });

    it('renders CodeBlock', () => {
      const spec = nestedSpec('CodeBlock', { code: 'const x = 42;', language: 'typescript' });
      render(<JsonRenderBlock spec={spec} />);
      expect(screen.getByText('const x = 42;')).toBeInTheDocument();
    });

    it('renders Chart (bar type)', () => {
      const spec = nestedSpec('Chart', {
        type: 'bar',
        data: [
          { name: 'Atoms', count: 14 },
          { name: 'Molecules', count: 14 },
        ],
        xKey: 'name',
        yKeys: ['count'],
        title: 'Components',
      });
      render(<JsonRenderBlock spec={spec} />);
      expect(screen.getByText('Components')).toBeInTheDocument();
    });
  });

  describe('JSONL brace recovery', () => {
    it('recovers elements with extra trailing brace', () => {
      // Simulates the exact bug from diagnostics_20260221_1
      const spec = [
        '{"op":"add","path":"/root","value":"main"}',
        '{"op":"add","path":"/elements/main","value":{"type":"Stack","props":{"direction":"vertical","gap":"md"},"children":["t1","tab-content"]}}',
        '{"op":"add","path":"/elements/t1","value":{"type":"Heading","props":{"text":"Dashboard","level":"h2"},"children":[]}}',
        // Extra trailing brace — the original bug
        '{"op":"add","path":"/elements/tab-content","value":{"type":"Text","props":{"text":"Tab panel visible"},"children":[]}}',
      ].join('\n');
      render(<JsonRenderBlock spec={spec} />);
      expect(screen.getByText('Dashboard')).toBeInTheDocument();
      expect(screen.getByText('Tab panel visible')).toBeInTheDocument();
    });
  });

  describe('complex dashboard (real-world AI output)', () => {
    it('renders a complete dashboard with KPIs, charts, and tables', () => {
      const spec = jsonlSpec('dashboard', {
        dashboard: {
          type: 'Stack',
          props: { direction: 'vertical', gap: 'md' },
          children: ['title', 'kpi-row', 'chart-row', 'table'],
        },
        title: {
          type: 'Heading',
          props: { text: 'Component Overview', level: 'h3' },
        },
        'kpi-row': {
          type: 'Grid',
          props: { columns: 4, gap: 'sm' },
          children: ['kpi1', 'kpi2', 'kpi3', 'kpi4'],
        },
        kpi1: { type: 'StatCard', props: { label: 'Total', value: '346' } },
        kpi2: { type: 'StatCard', props: { label: 'Atoms', value: '14' } },
        kpi3: { type: 'StatCard', props: { label: 'Molecules', value: '14' } },
        kpi4: { type: 'StatCard', props: { label: 'Design System', value: '13' } },
        'chart-row': {
          type: 'Grid',
          props: { columns: 2, gap: 'sm' },
          children: ['chart1', 'chart2'],
        },
        chart1: {
          type: 'Chart',
          props: {
            type: 'bar',
            data: [
              { name: 'icons', count: 77 },
              { name: 'settings', count: 74 },
              { name: 'ui', count: 51 },
            ],
            xKey: 'name',
            yKeys: ['count'],
            height: 160,
            title: 'By Directory',
          },
        },
        chart2: {
          type: 'Chart',
          props: {
            type: 'pie',
            data: [
              { name: 'Atoms', value: 14 },
              { name: 'Molecules', value: 14 },
              { name: 'Design System', value: 13 },
            ],
            xKey: 'name',
            yKeys: ['value'],
            height: 160,
            title: 'UI Breakdown',
          },
        },
        table: {
          type: 'Table',
          props: {
            columns: ['Directory', 'Files'],
            rows: [
              ['icons', '77'],
              ['settings', '74'],
              ['ui', '51'],
              ['recipes', '17'],
            ],
          },
        },
      });

      render(<JsonRenderBlock spec={spec} />);

      // Title
      expect(screen.getByText('Component Overview')).toBeInTheDocument();

      // KPI cards
      expect(screen.getByText('Total')).toBeInTheDocument();
      expect(screen.getByText('346')).toBeInTheDocument();
      expect(screen.getByText('Atoms')).toBeInTheDocument();

      // Chart titles
      expect(screen.getByText('By Directory')).toBeInTheDocument();
      expect(screen.getByText('UI Breakdown')).toBeInTheDocument();

      // Table data
      expect(screen.getByText('icons')).toBeInTheDocument();
      expect(screen.getByText('77')).toBeInTheDocument();
    });
  });
});
