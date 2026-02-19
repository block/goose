/**
 * Example JSON specs for testing the Generative UI system.
 * These can be used to verify rendering without needing AI generation.
 */

export const exampleDashboardSpec = {
  type: 'Section',
  props: { title: 'System Overview', subtitle: 'Last 24 hours' },
  children: [
    {
      type: 'Grid',
      props: { columns: 4, gap: 'md' },
      children: [
        {
          type: 'MetricCard',
          props: { label: 'Sessions', value: '142', delta: '+12%', deltaType: 'positive' },
        },
        {
          type: 'MetricCard',
          props: { label: 'Tool Calls', value: '1,847', delta: '+5%', deltaType: 'positive' },
        },
        {
          type: 'MetricCard',
          props: { label: 'Success Rate', value: '98.2%', delta: '-0.3%', deltaType: 'negative' },
        },
        {
          type: 'MetricCard',
          props: { label: 'Avg Latency', value: '1.2s', delta: 'stable', deltaType: 'neutral' },
        },
      ],
    },
    {
      type: 'Chart',
      props: {
        type: 'area',
        title: 'Sessions Over Time',
        xKey: 'date',
        yKeys: ['sessions'],
        height: 250,
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
      props: { columns: 2, gap: 'md' },
      children: [
        {
          type: 'Section',
          props: { title: 'Top Tools' },
          children: [
            {
              type: 'List',
              props: {
                items: [
                  { label: 'developer__shell', description: '4,260 calls', status: 'success' },
                  { label: 'developer__text_editor', description: '808 calls', status: 'success' },
                  { label: 'developer__analyze', description: '105 calls', status: 'success' },
                  { label: 'fetch__fetch', description: '42 calls', status: 'warning' },
                ],
              },
            },
          ],
        },
        {
          type: 'Section',
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
  type: 'Section',
  props: { title: 'Evaluation Results', subtitle: 'routing-tests-v2' },
  children: [
    {
      type: 'EvalResult',
      props: {
        datasetName: 'routing-tests-v2',
        accuracy: 87.5,
        agentAccuracy: 92.3,
        modeAccuracy: 85.1,
        testCount: 40,
        passCount: 35,
        failCount: 5,
      },
    },
    {
      type: 'AlertCard',
      props: {
        title: 'Regression Detected',
        message: 'Mode accuracy dropped 3.2% since last run. Check debug mode routing.',
        severity: 'warning',
      },
    },
    {
      type: 'DataTable',
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
        sortable: true,
      },
    },
  ],
};

export const exampleToolResultSpec = {
  type: 'Section',
  props: { title: 'Tool Execution Results' },
  children: [
    {
      type: 'Grid',
      props: { columns: 1, gap: 'sm' },
      children: [
        {
          type: 'ToolResult',
          props: {
            toolName: 'developer__shell',
            status: 'success',
            duration: '0.3s',
            output: 'Build successful',
          },
        },
        {
          type: 'ToolResult',
          props: { toolName: 'developer__text_editor', status: 'success', duration: '0.1s' },
        },
        {
          type: 'ToolResult',
          props: {
            toolName: 'fetch__fetch',
            status: 'error',
            duration: '5.0s',
            output: 'Connection timeout',
          },
        },
      ],
    },
    {
      type: 'ActionButton',
      props: { label: 'Retry Failed', action: 'create_session', variant: 'primary' },
    },
  ],
};
