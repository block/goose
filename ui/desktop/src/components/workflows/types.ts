/**
 * Pipeline DAG types for the visual workflow builder.
 * Maps to the goose/v1 Pipeline YAML format.
 */

export type NodeKind = 'trigger' | 'agent' | 'tool' | 'condition' | 'transform' | 'human' | 'a2a';

export interface TriggerConfig {
  event: 'manual' | 'schedule' | 'webhook' | 'pull_request';
  cron?: string;
  webhook_url?: string;
}

export interface AgentConfig {
  agent: string;
  mode?: string;
  prompt: string;
  max_turns?: number;
  provider?: string;
  model?: string;
}

export interface ToolConfig {
  extension: string;
  tool: string;
  arguments: Record<string, string>;
}

export interface ConditionConfig {
  expression: string;
}

export interface TransformConfig {
  template: string;
}

export interface HumanConfig {
  prompt: string;
  timeout?: number;
  default_action?: 'approve' | 'reject' | 'skip';
}

export interface A2aConfig {
  agent_card_url: string;
  task: string;
  timeout?: number;
}

export type NodeConfig =
  | TriggerConfig
  | AgentConfig
  | ToolConfig
  | ConditionConfig
  | TransformConfig
  | HumanConfig
  | A2aConfig;

export interface PipelineNode {
  id: string;
  type: NodeKind;
  config: NodeConfig;
  depends?: string[];
  condition?: string;
  label?: string;
}

export interface PipelineMetadata {
  name: string;
  description: string;
  tags?: string[];
}

export interface Pipeline {
  apiVersion: 'goose/v1';
  kind: 'Pipeline';
  metadata: PipelineMetadata;
  nodes: PipelineNode[];
}

/** React Flow node data payload */
export interface DagNodeData {
  kind: NodeKind;
  label: string;
  config: NodeConfig;
  condition?: string;
  status?: 'idle' | 'running' | 'success' | 'error' | 'skipped';
  output?: string;
  [key: string]: unknown;
}

/** Palette item for draggable node types */
export interface PaletteItem {
  kind: NodeKind;
  label: string;
  description: string;
  icon: string;
  color: string;
}

export const NODE_PALETTE: PaletteItem[] = [
  {
    kind: 'trigger',
    label: 'Trigger',
    description: 'Entry point for the pipeline',
    icon: 'Zap',
    color: '#6366f1',
  },
  {
    kind: 'agent',
    label: 'Agent',
    description: 'Run a Goose agent with a prompt',
    icon: 'Bot',
    color: '#8b5cf6',
  },
  {
    kind: 'tool',
    label: 'Tool',
    description: 'Call a specific MCP tool',
    icon: 'Wrench',
    color: '#0ea5e9',
  },
  {
    kind: 'condition',
    label: 'Condition',
    description: 'Boolean gate or branch',
    icon: 'GitBranch',
    color: '#f59e0b',
  },
  {
    kind: 'transform',
    label: 'Transform',
    description: 'Data transformation template',
    icon: 'ArrowRightLeft',
    color: '#10b981',
  },
  {
    kind: 'human',
    label: 'Human',
    description: 'Human-in-the-loop approval',
    icon: 'UserCheck',
    color: '#ec4899',
  },
  {
    kind: 'a2a',
    label: 'A2A Agent',
    description: 'Call an external A2A agent',
    icon: 'Globe',
    color: '#14b8a6',
  },
];

/** Default configs for each node kind */
export function defaultConfig(kind: NodeKind): NodeConfig {
  switch (kind) {
    case 'trigger':
      return { event: 'manual' } as TriggerConfig;
    case 'agent':
      return { agent: '', mode: '', prompt: '' } as AgentConfig;
    case 'tool':
      return { extension: '', tool: '', arguments: {} } as ToolConfig;
    case 'condition':
      return { expression: '' } as ConditionConfig;
    case 'transform':
      return { template: '' } as TransformConfig;
    case 'human':
      return {
        prompt: '',
        timeout: 300,
        default_action: 'skip',
      } as HumanConfig;
    case 'a2a':
      return { agent_card_url: '', task: '' } as A2aConfig;
  }
}
