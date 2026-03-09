/**
 * Example JSON specs for testing the Generative UI system.
 * These can be used to verify rendering without needing AI generation.
 */

export const exampleDashboardSpec = {
  type: 'Stack',
  props: { direction: 'vertical', gap: 'md' },
  children: [
    {
      type: 'PageHeader',
      props: { title: 'System Overview', description: 'Last 24 hours' },
    },
    {
      type: 'Grid',
      props: { columns: 4, gap: 'sm' },
      children: [
        { type: 'StatCard', props: { label: 'Sessions', value: '142', trend: 12 } },
        { type: 'StatCard', props: { label: 'Tool Calls', value: '1,847', trend: 5 } },
        { type: 'StatCard', props: { label: 'Success Rate', value: '98.2%', trend: -0.3 } },
        { type: 'StatCard', props: { label: 'Avg Latency', value: '1.2s', trend: 0 } },
      ],
    },
    {
      type: 'Chart',
      props: {
        type: 'area',
        title: 'Sessions Over Time',
        xKey: 'date',
        yKeys: ['sessions'],
        height: 200,
        data: [
          { date: 'Mon', sessions: 18 },
          { date: 'Tue', sessions: 24 },
          { date: 'Wed', sessions: 31 },
          { date: 'Thu', sessions: 22 },
          { date: 'Fri', sessions: 35 },
          { date: 'Sat', sessions: 12 },
          { date: 'Sun', sessions: 8 },
        ],
      },
    },
    {
      type: 'Grid',
      props: { columns: 2, gap: 'sm' },
      children: [
        {
          type: 'Card',
          props: { title: 'Top Tools' },
          children: [
            { type: 'ListItem', props: { title: 'developer__shell', description: '4,260 calls' } },
            {
              type: 'ListItem',
              props: { title: 'developer__text_editor', description: '808 calls' },
            },
            { type: 'ListItem', props: { title: 'developer__analyze', description: '105 calls' } },
            { type: 'ListItem', props: { title: 'fetch__fetch', description: '42 calls' } },
          ],
        },
        {
          type: 'Card',
          props: { title: 'Provider Usage' },
          children: [
            {
              type: 'Chart',
              props: {
                type: 'pie',
                xKey: 'provider',
                yKeys: ['sessions'],
                height: 200,
                data: [
                  { provider: 'Anthropic', sessions: 85 },
                  { provider: 'OpenAI', sessions: 42 },
                  { provider: 'Ollama', sessions: 15 },
                ],
              },
            },
          ],
        },
      ],
    },
  ],
};

export const exampleEvalSpec = {
  type: 'Stack',
  props: { direction: 'vertical', gap: 'md' },
  children: [
    {
      type: 'PageHeader',
      props: { title: 'Evaluation Results', description: 'routing-tests-v2' },
    },
    {
      type: 'Grid',
      props: { columns: 3, gap: 'sm' },
      children: [
        { type: 'StatCard', props: { label: 'Accuracy', value: '87.5%' } },
        { type: 'StatCard', props: { label: 'Agent Accuracy', value: '92.3%' } },
        { type: 'StatCard', props: { label: 'Mode Accuracy', value: '85.1%' } },
      ],
    },
    {
      type: 'Alert',
      props: {
        title: 'Regression Detected',
        description: 'Mode accuracy dropped 3.2% since last run. Check debug mode routing.',
        variant: 'destructive',
      },
    },
    {
      type: 'Table',
      props: {
        columns: [
          { key: 'input', label: 'Input', align: 'left' },
          { key: 'expected', label: 'Expected', align: 'left' },
          { key: 'actual', label: 'Actual', align: 'left' },
          { key: 'status', label: 'Status', align: 'center' },
        ],
        rows: [
          {
            input: 'Fix the auth bug',
            expected: 'coding/debug',
            actual: 'coding/code',
            status: '✗',
          },
          { input: 'Write unit tests', expected: 'qa/test', actual: 'coding/code', status: '✗' },
          {
            input: 'Deploy to staging',
            expected: 'coding/devops',
            actual: 'coding/devops',
            status: '✓',
          },
          { input: 'Review the PR', expected: 'pm/review', actual: 'pm/review', status: '✓' },
          { input: 'Check security', expected: 'qa/security', actual: 'qa/audit', status: '✗' },
        ],
        caption: 'Routing assertions',
      },
    },
  ],
};

export const exampleToolResultSpec = {
  type: 'Stack',
  props: { direction: 'vertical', gap: 'md' },
  children: [
    {
      type: 'PageHeader',
      props: { title: 'Tool Execution Results' },
    },
    {
      type: 'Table',
      props: {
        columns: [
          { key: 'tool', label: 'Tool', align: 'left' },
          { key: 'status', label: 'Status', align: 'left' },
          { key: 'duration', label: 'Duration', align: 'right' },
          { key: 'output', label: 'Output', align: 'left' },
        ],
        rows: [
          {
            tool: 'developer__shell',
            status: 'success',
            duration: '0.3s',
            output: 'Build successful',
          },
          {
            tool: 'developer__text_editor',
            status: 'success',
            duration: '0.1s',
            output: '',
          },
          {
            tool: 'fetch__fetch',
            status: 'error',
            duration: '5.0s',
            output: 'Connection timeout',
          },
        ],
      },
    },
    {
      type: 'Button',
      props: { label: 'Retry Failed', variant: 'destructive' },
    },
  ],
};
