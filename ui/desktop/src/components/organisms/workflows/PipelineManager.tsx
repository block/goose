import type { Edge, Node } from '@xyflow/react';
import { Clock, FileText, FolderOpen, Plus, Trash2 } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { deletePipeline, getPipeline, listPipelines, savePipeline, type Pipeline } from '@/api';
import { DagEditor } from './DagEditor';
import { pipelineToFlow } from './serialization';
import type { DagNodeData, PipelineMetadata } from './types';

interface PipelineListItem {
  id: string;
  name: string;
  description: string;
  node_count: number;
  updated_at: string;
}

export function PipelineManager() {
  const [pipelines, setPipelines] = useState<PipelineListItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [activePipeline, setActivePipeline] = useState<{
    id: string | null;
    nodes: Node<DagNodeData>[];
    edges: Edge[];
    metadata: PipelineMetadata;
  } | null>(null);

  const fetchPipelines = useCallback(async () => {
    try {
      setLoading(true);
      setError(null);
      const response = await listPipelines();
      if (response.data) {
        const items = (response.data as { pipelines?: PipelineListItem[] })?.pipelines;
        setPipelines(items ?? []);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load pipelines');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchPipelines();
  }, [fetchPipelines]);

  const handleNewPipeline = useCallback(() => {
    setActivePipeline({
      id: null,
      nodes: [],
      edges: [],
      metadata: { name: 'New Pipeline', description: '' },
    });
  }, []);

  const handleOpenPipeline = useCallback(async (id: string) => {
    try {
      const response = await getPipeline({ path: { id } });
      if (response.data) {
        const pipeline = response.data as {
          name: string;
          description?: string;
          nodes: Array<{
            id: string;
            kind: string;
            label: string;
            config: Record<string, unknown>;
            position?: { x: number; y: number };
          }>;
          edges: Array<{
            id: string;
            source: string;
            target: string;
            label?: string;
          }>;
        };
        const { nodes, edges } = pipelineToFlow(pipeline as never);
        setActivePipeline({
          id,
          nodes,
          edges,
          metadata: {
            name: pipeline.name,
            description: pipeline.description ?? '',
          },
        });
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load pipeline');
    }
  }, []);

  const handleSave = useCallback(
    async (_yaml: string, json: string) => {
      try {
        const parsed = JSON.parse(json);
        const pipeline: Pipeline = {
          name: parsed.name ?? activePipeline?.metadata.name ?? 'Untitled',
          description: parsed.description ?? activePipeline?.metadata.description ?? '',
          nodes: parsed.nodes ?? [],
          edges: parsed.edges ?? [],
          api_version: 'goose/v1',
          kind: 'Pipeline',
          tags: parsed.tags ?? [],
          version: parsed.version ?? '1.0.0',
          layout: parsed.layout ?? null,
        };
        const response = await savePipeline({
          body: {
            pipeline,
            id: activePipeline?.id ?? undefined,
          },
        });
        if (response.data) {
          const saved = response.data as { id?: string };
          if (saved.id && activePipeline) {
            setActivePipeline((prev) => (prev ? { ...prev, id: saved.id ?? prev.id } : prev));
          }
          await fetchPipelines();
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to save');
      }
    },
    [activePipeline, fetchPipelines]
  );

  const handleDeletePipeline = useCallback(
    async (id: string) => {
      if (!window.confirm('Delete this pipeline? This cannot be undone.')) {
        return;
      }
      try {
        await deletePipeline({ path: { id } });
        setPipelines((prev) => prev.filter((p) => p.id !== id));
        if (activePipeline?.id === id) {
          setActivePipeline(null);
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to delete');
      }
    },
    [activePipeline]
  );

  const handleBack = useCallback(() => {
    setActivePipeline(null);
  }, []);

  // Active pipeline editor
  if (activePipeline) {
    return (
      <div className="flex flex-col h-full">
        <div className="flex items-center gap-2 px-4 py-2 border-b border-border-default bg-background-default">
          <button type="button"
            onClick={handleBack}
            className="text-xs text-text-muted hover:text-text-default transition-colors"
          >
            ← Back to Pipelines
          </button>
          <span className="text-text-subtle">|</span>
          <span className="text-sm font-medium text-text-default">
            {activePipeline.metadata.name}
          </span>
          {activePipeline.id && (
            <span className="text-xs text-text-subtle">({activePipeline.id.slice(0, 8)})</span>
          )}
        </div>
        <div className="flex-1">
          <DagEditor
            initialNodes={activePipeline.nodes}
            initialEdges={activePipeline.edges}
            metadata={activePipeline.metadata}
            onSave={handleSave}
          />
        </div>
      </div>
    );
  }

  // Pipeline list view
  return (
    <div className="flex flex-col h-full bg-background-app">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b border-border-default">
        <div>
          <h1 className="text-xl font-semibold text-text-default">Pipelines</h1>
          <p className="text-sm text-text-muted mt-0.5">
            Visual DAG workflows for multi-agent orchestration
          </p>
        </div>
        <button type="button"
          onClick={handleNewPipeline}
          className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg
                     bg-background-accent text-text-on-accent text-sm font-medium
                     hover:opacity-90 transition-opacity"
        >
          <Plus size={16} />
          New Pipeline
        </button>
      </div>

      {/* Error banner */}
      {error && (
        <div className="mx-6 mt-4 p-3 bg-background-danger-muted border border-border-default rounded-lg text-sm text-text-danger">
          {error}
          <button type="button" onClick={() => setError(null)} className="ml-2 underline">
            Dismiss
          </button>
        </div>
      )}

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-6">
        {loading ? (
          <div className="flex items-center justify-center h-48 text-text-muted">
            Loading pipelines...
          </div>
        ) : pipelines.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-64 text-center">
            <FolderOpen size={48} className="text-text-subtle mb-4" />
            <h2 className="text-lg font-medium text-text-default mb-1">No pipelines yet</h2>
            <p className="text-sm text-text-muted mb-4 max-w-md">
              Create your first visual workflow by connecting agents, tools, and conditions in a
              DAG.
            </p>
            <button type="button"
              onClick={handleNewPipeline}
              className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg
                         bg-background-accent text-text-on-accent text-sm font-medium
                         hover:opacity-90 transition-opacity"
            >
              <Plus size={16} />
              Create Pipeline
            </button>
          </div>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {pipelines.map((pipeline) => (
              <div
                key={pipeline.id}
                className="group relative border border-border-default rounded-lg p-4
                           bg-background-default hover:border-border-accent
                           cursor-pointer transition-colors"
                onClick={() => handleOpenPipeline(pipeline.id)}
              >
                <div className="flex items-start justify-between">
                  <div className="flex items-center gap-2">
                    <FileText size={18} className="text-text-accent" />
                    <h3 className="text-sm font-medium text-text-default">{pipeline.name}</h3>
                  </div>
                  <button type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      handleDeletePipeline(pipeline.id);
                    }}
                    className="opacity-0 group-hover:opacity-100 p-1 rounded
                               hover:bg-background-danger-muted text-text-muted
                               hover:text-text-danger transition-all"
                    title="Delete pipeline"
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
                {pipeline.description && (
                  <p className="text-xs text-text-muted mt-1.5 line-clamp-2">
                    {pipeline.description}
                  </p>
                )}
                <div className="flex items-center gap-3 mt-3 text-xs text-text-subtle">
                  <span>{pipeline.node_count} nodes</span>
                  <span className="flex items-center gap-1">
                    <Clock size={10} />
                    {new Date(pipeline.updated_at).toLocaleDateString()}
                  </span>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
