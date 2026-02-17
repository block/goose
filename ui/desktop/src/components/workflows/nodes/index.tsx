import React, { memo } from 'react';
import { Handle, Position, type NodeProps } from '@xyflow/react';
import type { DagNodeData, NodeKind } from '../types';
import {
  Zap,
  Bot,
  Wrench,
  GitBranch,
  ArrowRightLeft,
  UserCheck,
  Globe,
  CheckCircle2,
  XCircle,
  Loader2,
  SkipForward,
} from 'lucide-react';

const KIND_COLORS: Record<NodeKind, string> = {
  trigger: '#6366f1',
  agent: '#8b5cf6',
  tool: '#0ea5e9',
  condition: '#f59e0b',
  transform: '#10b981',
  human: '#ec4899',
  a2a: '#14b8a6',
};

const KIND_ICONS: Record<NodeKind, React.FC<{ size?: number }>> = {
  trigger: Zap,
  agent: Bot,
  tool: Wrench,
  condition: GitBranch,
  transform: ArrowRightLeft,
  human: UserCheck,
  a2a: Globe,
};

const STATUS_ICONS: Record<string, React.FC<{ size?: number; className?: string }>> = {
  running: Loader2,
  success: CheckCircle2,
  error: XCircle,
  skipped: SkipForward,
};

function BaseNode({ data, selected }: NodeProps & { data: DagNodeData }) {
  const color = KIND_COLORS[data.kind];
  const Icon = KIND_ICONS[data.kind];
  const StatusIcon = data.status ? STATUS_ICONS[data.status] : null;
  const isCondition = data.kind === 'condition';

  return (
    <div
      className={`relative rounded-lg border-2 bg-background-default shadow-md transition-all ${
        selected ? 'ring-2 ring-offset-2 ring-offset-background-default' : ''
      }`}
      style={{
        borderColor: selected ? color : `${color}66`,
        minWidth: 180,
        maxWidth: 260,
      }}
    >
      {/* Input handle — hide for trigger */}
      {data.kind !== 'trigger' && (
        <Handle
          type="target"
          position={Position.Top}
          className="!w-3 !h-3 !border-2 !border-background-default"
          style={{ background: color }}
        />
      )}

      {/* Header */}
      <div
        className="flex items-center gap-2 px-3 py-2 rounded-t-md"
        style={{ background: `${color}15` }}
      >
        <span style={{ color }}>
          <Icon size={16} />
        </span>
        <span className="text-xs font-semibold uppercase tracking-wider" style={{ color }}>
          {data.kind}
        </span>
        {StatusIcon && (
          <StatusIcon
            size={14}
            className={`ml-auto ${
              data.status === 'running'
                ? 'animate-spin text-text-accent'
                : data.status === 'success'
                  ? 'text-text-success'
                  : data.status === 'error'
                    ? 'text-text-danger'
                    : 'text-text-muted'
            }`}
          />
        )}
      </div>

      {/* Body */}
      <div className="px-3 py-2">
        <div className="text-sm font-medium text-text-default truncate">{data.label}</div>
        {data.condition && (
          <div className="mt-1 text-xs text-text-muted truncate">when: {data.condition}</div>
        )}
      </div>

      {/* Output handle */}
      <Handle
        type="source"
        position={Position.Bottom}
        className="!w-3 !h-3 !border-2 !border-background-default"
        style={{ background: color }}
      />

      {/* Condition node: second output for false branch */}
      {isCondition && (
        <Handle
          type="source"
          position={Position.Right}
          id="false"
          className="!w-3 !h-3 !border-2 !border-background-default"
          style={{ background: '#ef4444' }}
        />
      )}
    </div>
  );
}

export const TriggerNode = memo((props: NodeProps) => (
  <BaseNode {...props} data={props.data as DagNodeData} />
));
TriggerNode.displayName = 'TriggerNode';

export const AgentNode = memo((props: NodeProps) => (
  <BaseNode {...props} data={props.data as DagNodeData} />
));
AgentNode.displayName = 'AgentNode';

export const ToolNode = memo((props: NodeProps) => (
  <BaseNode {...props} data={props.data as DagNodeData} />
));
ToolNode.displayName = 'ToolNode';

export const ConditionNode = memo((props: NodeProps) => (
  <BaseNode {...props} data={props.data as DagNodeData} />
));
ConditionNode.displayName = 'ConditionNode';

export const TransformNode = memo((props: NodeProps) => (
  <BaseNode {...props} data={props.data as DagNodeData} />
));
TransformNode.displayName = 'TransformNode';

export const HumanNode = memo((props: NodeProps) => (
  <BaseNode {...props} data={props.data as DagNodeData} />
));
HumanNode.displayName = 'HumanNode';

export const A2aNode = memo((props: NodeProps) => (
  <BaseNode {...props} data={props.data as DagNodeData} />
));
A2aNode.displayName = 'A2aNode';

export const nodeTypes = {
  trigger: TriggerNode,
  agent: AgentNode,
  tool: ToolNode,
  condition: ConditionNode,
  transform: TransformNode,
  human: HumanNode,
  a2a: A2aNode,
};
