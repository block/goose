import { Settings2, X } from 'lucide-react';
import type React from 'react';
import type {
  A2aConfig,
  AgentConfig,
  ConditionConfig,
  DagNodeData,
  HumanConfig,
  NodeKind,
  ToolConfig,
  TransformConfig,
  TriggerConfig,
} from '../types';

interface PropertiesPanelProps {
  nodeId: string;
  data: DagNodeData;
  onUpdate: (nodeId: string, data: Partial<DagNodeData>) => void;
  onDelete: (nodeId: string) => void;
  onClose: () => void;
}

function FieldLabel({ children }: { children: React.ReactNode }) {
  return <label className="block text-xs font-medium text-text-muted mb-1">{children}</label>;
}

function TextInput({
  value,
  onChange,
  placeholder,
}: {
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
}) {
  return (
    <input
      type="text"
      value={value}
      onChange={(e) => onChange(e.target.value)}
      placeholder={placeholder}
      className="w-full px-2 py-1.5 text-sm rounded-md border border-border-default
                 bg-background-default text-text-default placeholder-text-subtle
                 focus:outline-none focus:ring-1 focus:ring-border-accent"
    />
  );
}

function TextArea({
  value,
  onChange,
  placeholder,
  rows = 3,
}: {
  value: string;
  onChange: (v: string) => void;
  placeholder?: string;
  rows?: number;
}) {
  return (
    <textarea
      value={value}
      onChange={(e) => onChange(e.target.value)}
      placeholder={placeholder}
      rows={rows}
      className="w-full px-2 py-1.5 text-sm rounded-md border border-border-default
                 bg-background-default text-text-default placeholder-text-subtle
                 focus:outline-none focus:ring-1 focus:ring-border-accent resize-y"
    />
  );
}

function SelectInput({
  value,
  onChange,
  options,
}: {
  value: string;
  onChange: (v: string) => void;
  options: { value: string; label: string }[];
}) {
  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className="w-full px-2 py-1.5 text-sm rounded-md border border-border-default
                 bg-background-default text-text-default
                 focus:outline-none focus:ring-1 focus:ring-border-accent"
    >
      {options.map((opt) => (
        <option key={opt.value} value={opt.value}>
          {opt.label}
        </option>
      ))}
    </select>
  );
}

function NumberInput({
  value,
  onChange,
  min,
  max,
}: {
  value: number | undefined;
  onChange: (v: number) => void;
  min?: number;
  max?: number;
}) {
  return (
    <input
      type="number"
      value={value ?? ''}
      onChange={(e) => onChange(parseInt(e.target.value, 10))}
      min={min}
      max={max}
      className="w-full px-2 py-1.5 text-sm rounded-md border border-border-default
                 bg-background-default text-text-default
                 focus:outline-none focus:ring-1 focus:ring-border-accent"
    />
  );
}

function updateConfig<T>(
  data: DagNodeData,
  onUpdate: (id: string, d: Partial<DagNodeData>) => void,
  nodeId: string,
  patch: Partial<T>
) {
  onUpdate(nodeId, { config: { ...data.config, ...patch } });
}

function TriggerFields({
  nodeId,
  data,
  onUpdate,
}: {
  nodeId: string;
  data: DagNodeData;
  onUpdate: PropertiesPanelProps['onUpdate'];
}) {
  const config = data.config as TriggerConfig;
  return (
    <>
      <FieldLabel>Event Type</FieldLabel>
      <SelectInput
        value={config.event}
        onChange={(v) =>
          updateConfig<TriggerConfig>(data, onUpdate, nodeId, {
            event: v as TriggerConfig['event'],
          })
        }
        options={[
          { value: 'manual', label: 'Manual' },
          { value: 'schedule', label: 'Schedule (cron)' },
          { value: 'webhook', label: 'Webhook' },
          { value: 'pull_request', label: 'Pull Request' },
        ]}
      />
      {config.event === 'schedule' && (
        <div className="mt-2">
          <FieldLabel>Cron Expression</FieldLabel>
          <TextInput
            value={config.cron ?? ''}
            onChange={(v) => updateConfig<TriggerConfig>(data, onUpdate, nodeId, { cron: v })}
            placeholder="0 9 * * *"
          />
        </div>
      )}
    </>
  );
}

function AgentFields({
  nodeId,
  data,
  onUpdate,
}: {
  nodeId: string;
  data: DagNodeData;
  onUpdate: PropertiesPanelProps['onUpdate'];
}) {
  const config = data.config as AgentConfig;
  return (
    <>
      <FieldLabel>Agent Name</FieldLabel>
      <TextInput
        value={config.agent}
        onChange={(v) => updateConfig<AgentConfig>(data, onUpdate, nodeId, { agent: v })}
        placeholder="e.g., developer"
      />
      <div className="mt-2">
        <FieldLabel>Prompt</FieldLabel>
        <TextArea
          value={config.prompt}
          onChange={(v) => updateConfig<AgentConfig>(data, onUpdate, nodeId, { prompt: v })}
          placeholder="What should this agent do?"
        />
      </div>
      <div className="mt-2">
        <FieldLabel>Max Turns</FieldLabel>
        <NumberInput
          value={config.max_turns}
          onChange={(v) => updateConfig<AgentConfig>(data, onUpdate, nodeId, { max_turns: v })}
          min={1}
          max={100}
        />
      </div>
      <div className="mt-2">
        <FieldLabel>Provider</FieldLabel>
        <TextInput
          value={config.provider ?? ''}
          onChange={(v) => updateConfig<AgentConfig>(data, onUpdate, nodeId, { provider: v })}
          placeholder="e.g., anthropic"
        />
      </div>
      <div className="mt-2">
        <FieldLabel>Model</FieldLabel>
        <TextInput
          value={config.model ?? ''}
          onChange={(v) => updateConfig<AgentConfig>(data, onUpdate, nodeId, { model: v })}
          placeholder="e.g., claude-sonnet-4-20250514"
        />
      </div>
    </>
  );
}

function ToolFields({
  nodeId,
  data,
  onUpdate,
}: {
  nodeId: string;
  data: DagNodeData;
  onUpdate: PropertiesPanelProps['onUpdate'];
}) {
  const config = data.config as ToolConfig;
  return (
    <>
      <FieldLabel>Extension</FieldLabel>
      <TextInput
        value={config.extension}
        onChange={(v) => updateConfig<ToolConfig>(data, onUpdate, nodeId, { extension: v })}
        placeholder="e.g., developer"
      />
      <div className="mt-2">
        <FieldLabel>Tool</FieldLabel>
        <TextInput
          value={config.tool}
          onChange={(v) => updateConfig<ToolConfig>(data, onUpdate, nodeId, { tool: v })}
          placeholder="e.g., shell"
        />
      </div>
    </>
  );
}

function ConditionFields({
  nodeId,
  data,
  onUpdate,
}: {
  nodeId: string;
  data: DagNodeData;
  onUpdate: PropertiesPanelProps['onUpdate'];
}) {
  const config = data.config as ConditionConfig;
  return (
    <>
      <FieldLabel>Expression</FieldLabel>
      <TextArea
        value={config.expression}
        onChange={(v) => updateConfig<ConditionConfig>(data, onUpdate, nodeId, { expression: v })}
        placeholder="e.g., steps.lint.exit_code == 0"
        rows={2}
      />
    </>
  );
}

function TransformFields({
  nodeId,
  data,
  onUpdate,
}: {
  nodeId: string;
  data: DagNodeData;
  onUpdate: PropertiesPanelProps['onUpdate'];
}) {
  const config = data.config as TransformConfig;
  return (
    <>
      <FieldLabel>Template</FieldLabel>
      <TextArea
        value={config.template}
        onChange={(v) => updateConfig<TransformConfig>(data, onUpdate, nodeId, { template: v })}
        placeholder="Transform template using {{ step.output }}"
        rows={4}
      />
    </>
  );
}

function HumanFields({
  nodeId,
  data,
  onUpdate,
}: {
  nodeId: string;
  data: DagNodeData;
  onUpdate: PropertiesPanelProps['onUpdate'];
}) {
  const config = data.config as HumanConfig;
  return (
    <>
      <FieldLabel>Approval Prompt</FieldLabel>
      <TextArea
        value={config.prompt}
        onChange={(v) => updateConfig<HumanConfig>(data, onUpdate, nodeId, { prompt: v })}
        placeholder="What should the human review?"
      />
      <div className="mt-2">
        <FieldLabel>Timeout (seconds)</FieldLabel>
        <NumberInput
          value={config.timeout}
          onChange={(v) => updateConfig<HumanConfig>(data, onUpdate, nodeId, { timeout: v })}
          min={0}
        />
      </div>
      <div className="mt-2">
        <FieldLabel>Default Action</FieldLabel>
        <SelectInput
          value={config.default_action ?? 'skip'}
          onChange={(v) =>
            updateConfig<HumanConfig>(data, onUpdate, nodeId, {
              default_action: v as HumanConfig['default_action'],
            })
          }
          options={[
            { value: 'approve', label: 'Auto-approve' },
            { value: 'reject', label: 'Auto-reject' },
            { value: 'skip', label: 'Skip' },
          ]}
        />
      </div>
    </>
  );
}

function A2aFields({
  nodeId,
  data,
  onUpdate,
}: {
  nodeId: string;
  data: DagNodeData;
  onUpdate: PropertiesPanelProps['onUpdate'];
}) {
  const config = data.config as A2aConfig;
  return (
    <>
      <FieldLabel>Agent Card URL</FieldLabel>
      <TextInput
        value={config.agent_card_url}
        onChange={(v) => updateConfig<A2aConfig>(data, onUpdate, nodeId, { agent_card_url: v })}
        placeholder="https://agent.example.com/.well-known/agent.json"
      />
      <div className="mt-2">
        <FieldLabel>Task Prompt</FieldLabel>
        <TextArea
          value={config.task}
          onChange={(v) => updateConfig<A2aConfig>(data, onUpdate, nodeId, { task: v })}
          placeholder="Task to send to the A2A agent"
        />
      </div>
      <div className="mt-2">
        <FieldLabel>Timeout (seconds)</FieldLabel>
        <NumberInput
          value={config.timeout}
          onChange={(v) => updateConfig<A2aConfig>(data, onUpdate, nodeId, { timeout: v })}
          min={0}
        />
      </div>
    </>
  );
}

const FIELD_COMPONENTS: Record<
  NodeKind,
  React.FC<{ nodeId: string; data: DagNodeData; onUpdate: PropertiesPanelProps['onUpdate'] }>
> = {
  trigger: TriggerFields,
  agent: AgentFields,
  tool: ToolFields,
  condition: ConditionFields,
  transform: TransformFields,
  human: HumanFields,
  a2a: A2aFields,
};

export function PropertiesPanel({
  nodeId,
  data,
  onUpdate,
  onDelete,
  onClose,
}: PropertiesPanelProps) {
  const FieldsComponent = FIELD_COMPONENTS[data.kind];

  return (
    <div className="w-72 border-l border-border-default bg-background-default overflow-y-auto flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between p-3 border-b border-border-muted">
        <div className="flex items-center gap-2">
          <Settings2 size={14} className="text-text-muted" />
          <h3 className="text-sm font-semibold text-text-default">Properties</h3>
        </div>
        <button type="button"
          onClick={onClose}
          className="p-1 rounded-md hover:bg-background-muted text-text-muted"
        >
          <X size={14} />
        </button>
      </div>

      {/* Label */}
      <div className="p-3 border-b border-border-muted">
        <FieldLabel>Label</FieldLabel>
        <TextInput
          value={data.label}
          onChange={(v) => onUpdate(nodeId, { label: v })}
          placeholder="Node name"
        />
      </div>

      {/* Type-specific fields */}
      <div className="p-3 flex-1 space-y-2">
        {FieldsComponent && <FieldsComponent nodeId={nodeId} data={data} onUpdate={onUpdate} />}

        {/* Condition guard (for non-condition nodes) */}
        {data.kind !== 'condition' && data.kind !== 'trigger' && (
          <div className="mt-4 pt-3 border-t border-border-muted">
            <FieldLabel>Run Condition (optional)</FieldLabel>
            <TextInput
              value={data.condition ?? ''}
              onChange={(v) => onUpdate(nodeId, { condition: v || undefined })}
              placeholder="e.g., steps.lint.exit_code == 0"
            />
          </div>
        )}
      </div>

      {/* Delete */}
      <div className="p-3 border-t border-border-muted">
        <button type="button"
          onClick={() => onDelete(nodeId)}
          className="w-full px-3 py-2 text-sm rounded-md
                     bg-background-danger-muted text-text-danger
                     hover:bg-red-500/20 transition-colors"
        >
          Delete Node
        </button>
      </div>
    </div>
  );
}
