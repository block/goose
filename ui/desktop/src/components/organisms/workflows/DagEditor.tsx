import {
  addEdge,
  Background,
  type Connection,
  Controls,
  type Edge,
  MiniMap,
  type Node,
  type OnConnect,
  Panel,
  ReactFlow,
  ReactFlowProvider,
  useEdgesState,
  useNodesState,
  useReactFlow,
} from '@xyflow/react';
import type React from 'react';
import { useCallback, useMemo, useRef, useState } from 'react';
import '@xyflow/react/dist/style.css';
import { FileJson, FileText, Redo2, Save, Undo2 } from 'lucide-react';
import { nodeTypes } from './nodes';
import { NodePalette } from './panels/NodePalette';
import { PropertiesPanel } from './panels/PropertiesPanel';
import { createNode, flowToPipeline, pipelineToJson, pipelineToYaml } from './serialization';
import type { DagNodeData, NodeKind, PipelineMetadata } from './types';

interface DagEditorProps {
  initialNodes?: Node<DagNodeData>[];
  initialEdges?: Edge[];
  metadata?: PipelineMetadata;
  onSave?: (yaml: string, json: string) => void;
}

function DagEditorInner({
  initialNodes = [],
  initialEdges = [],
  metadata = { name: 'New Pipeline', description: '' },
  onSave,
}: DagEditorProps) {
  const reactFlowWrapper = useRef<HTMLDivElement>(null);
  const { screenToFlowPosition } = useReactFlow();

  const [nodes, setNodes, onNodesChange] = useNodesState(initialNodes);
  const [edges, setEdges, onEdgesChange] = useEdgesState(initialEdges);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [pipelineMeta, setPipelineMeta] = useState<PipelineMetadata>(metadata);
  const [showExport, setShowExport] = useState<'yaml' | 'json' | null>(null);
  const [draggingKind, setDraggingKind] = useState<NodeKind | null>(null);

  // History for undo/redo
  const [history, setHistory] = useState<{ nodes: Node<DagNodeData>[]; edges: Edge[] }[]>([]);
  const [historyIdx, setHistoryIdx] = useState(-1);

  const pushHistory = useCallback(() => {
    setHistory((prev) => {
      const next = [...prev.slice(0, historyIdx + 1), { nodes: [...nodes], edges: [...edges] }];
      return next.slice(-50); // Cap at 50 entries
    });
    setHistoryIdx((prev) => Math.min(prev + 1, 49));
  }, [nodes, edges, historyIdx]);

  const undo = useCallback(() => {
    if (historyIdx > 0) {
      const state = history[historyIdx - 1];
      setNodes(state.nodes);
      setEdges(state.edges);
      setHistoryIdx((prev) => prev - 1);
    }
  }, [history, historyIdx, setNodes, setEdges]);

  const redo = useCallback(() => {
    if (historyIdx < history.length - 1) {
      const state = history[historyIdx + 1];
      setNodes(state.nodes);
      setEdges(state.edges);
      setHistoryIdx((prev) => prev + 1);
    }
  }, [history, historyIdx, setNodes, setEdges]);

  // Connect nodes
  const onConnect: OnConnect = useCallback(
    (params: Connection) => {
      pushHistory();
      setEdges((eds) =>
        addEdge(
          {
            ...params,
            animated: false,
            style: { strokeWidth: 2, stroke: '#6366f1' },
          },
          eds
        )
      );
    },
    [setEdges, pushHistory]
  );

  // Drop node from palette
  const onDragOver = useCallback((event: React.DragEvent) => {
    event.preventDefault();
    event.dataTransfer.dropEffect = 'move';
  }, []);

  const onDrop = useCallback(
    (event: React.DragEvent) => {
      event.preventDefault();
      const kind = event.dataTransfer.getData('application/dagnode') as NodeKind;
      if (!kind) return;

      const position = screenToFlowPosition({
        x: event.clientX,
        y: event.clientY,
      });

      pushHistory();
      const newNode = createNode(kind, position);
      setNodes((nds) => [...nds, newNode]);
      setSelectedNodeId(newNode.id);
      setDraggingKind(null);
    },
    [screenToFlowPosition, setNodes, pushHistory]
  );

  // Node selection
  const onNodeClick = useCallback((_: React.MouseEvent, node: Node) => {
    setSelectedNodeId(node.id);
  }, []);

  const onPaneClick = useCallback(() => {
    setSelectedNodeId(null);
  }, []);

  // Update node data from properties panel
  const onUpdateNode = useCallback(
    (nodeId: string, data: Partial<DagNodeData>) => {
      pushHistory();
      setNodes((nds) =>
        nds.map((n) => (n.id === nodeId ? { ...n, data: { ...n.data, ...data } } : n))
      );
    },
    [setNodes, pushHistory]
  );

  // Delete node
  const onDeleteNode = useCallback(
    (nodeId: string) => {
      pushHistory();
      setNodes((nds) => nds.filter((n) => n.id !== nodeId));
      setEdges((eds) => eds.filter((e) => e.source !== nodeId && e.target !== nodeId));
      setSelectedNodeId(null);
    },
    [setNodes, setEdges, pushHistory]
  );

  // Selected node data
  const selectedNode = useMemo(
    () => nodes.find((n) => n.id === selectedNodeId),
    [nodes, selectedNodeId]
  );

  // Save / Export
  const handleSave = useCallback(() => {
    const pipeline = flowToPipeline(nodes, edges, pipelineMeta);
    const yaml = pipelineToYaml(pipeline);
    const json = pipelineToJson(pipeline);
    onSave?.(yaml, json);
  }, [nodes, edges, pipelineMeta, onSave]);

  const handleExport = useCallback(
    (format: 'yaml' | 'json') => {
      const pipeline = flowToPipeline(nodes, edges, pipelineMeta);
      const content = format === 'yaml' ? pipelineToYaml(pipeline) : pipelineToJson(pipeline);
      navigator.clipboard.writeText(content);
      setShowExport(format);
      setTimeout(() => setShowExport(null), 2000);
    },
    [nodes, edges, pipelineMeta]
  );

  return (
    <div className="flex h-full w-full bg-background-default">
      {/* Node Palette */}
      <NodePalette onDragStart={setDraggingKind} />

      {/* Canvas */}
      <div className="flex-1 relative" ref={reactFlowWrapper}>
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onConnect={onConnect}
          onDragOver={onDragOver}
          onDrop={onDrop}
          onNodeClick={onNodeClick}
          onPaneClick={onPaneClick}
          nodeTypes={nodeTypes}
          fitView
          snapToGrid
          snapGrid={[20, 20]}
          className={draggingKind ? 'cursor-copy' : ''}
          proOptions={{ hideAttribution: true }}
        >
          <Background gap={20} size={1} />
          <Controls
            className="!bg-background-default !border-border-default !shadow-md"
            showInteractive={false}
          />
          <MiniMap
            className="!bg-background-muted !border-border-default"
            nodeColor={(node) => {
              const data = node.data as DagNodeData;
              const colors: Record<NodeKind, string> = {
                trigger: '#6366f1',
                agent: '#8b5cf6',
                tool: '#0ea5e9',
                condition: '#f59e0b',
                transform: '#10b981',
                human: '#ec4899',
                a2a: '#14b8a6',
              };
              return colors[data.kind] ?? '#6b7280';
            }}
          />

          {/* Toolbar */}
          <Panel position="top-center">
            <div className="flex items-center gap-1 bg-background-default border border-border-default rounded-lg shadow-md px-2 py-1">
              {/* Pipeline name */}
              <input
                type="text"
                value={pipelineMeta.name}
                onChange={(e) => setPipelineMeta((m) => ({ ...m, name: e.target.value }))}
                className="text-sm font-medium text-text-default bg-transparent border-none
                           focus:outline-none focus:ring-0 w-40 text-center"
                placeholder="Pipeline name"
              />

              <div className="w-px h-5 bg-border-muted mx-1" />

              <button type="button"
                onClick={undo}
                disabled={historyIdx <= 0}
                className="p-1.5 rounded-md hover:bg-background-muted disabled:opacity-30 text-text-muted"
                title="Undo"
              >
                <Undo2 size={14} />
              </button>
              <button type="button"
                onClick={redo}
                disabled={historyIdx >= history.length - 1}
                className="p-1.5 rounded-md hover:bg-background-muted disabled:opacity-30 text-text-muted"
                title="Redo"
              >
                <Redo2 size={14} />
              </button>

              <div className="w-px h-5 bg-border-muted mx-1" />

              <button type="button"
                onClick={() => handleExport('yaml')}
                className="p-1.5 rounded-md hover:bg-background-muted text-text-muted"
                title="Copy as YAML"
              >
                <FileText size={14} />
              </button>
              <button type="button"
                onClick={() => handleExport('json')}
                className="p-1.5 rounded-md hover:bg-background-muted text-text-muted"
                title="Copy as JSON"
              >
                <FileJson size={14} />
              </button>

              <div className="w-px h-5 bg-border-muted mx-1" />

              <button type="button"
                onClick={handleSave}
                className="flex items-center gap-1 px-2 py-1 rounded-md
                           bg-background-accent text-text-on-accent text-xs font-medium
                           hover:opacity-90 transition-opacity"
              >
                <Save size={12} />
                Save
              </button>

              {showExport && (
                <span className="text-xs text-text-success ml-1">
                  Copied {showExport.toUpperCase()}!
                </span>
              )}
            </div>
          </Panel>

          {/* Empty state */}
          {nodes.length === 0 && (
            <Panel position="top-center" className="mt-20">
              <div className="text-center text-text-muted">
                <p className="text-sm font-medium">Drag nodes from the palette</p>
                <p className="text-xs mt-1">Start with a Trigger node</p>
              </div>
            </Panel>
          )}
        </ReactFlow>
      </div>

      {/* Properties Panel */}
      {selectedNode && (
        <PropertiesPanel
          nodeId={selectedNode.id}
          data={selectedNode.data as DagNodeData}
          onUpdate={onUpdateNode}
          onDelete={onDeleteNode}
          onClose={() => setSelectedNodeId(null)}
        />
      )}
    </div>
  );
}

export function DagEditor(props: DagEditorProps) {
  return (
    <ReactFlowProvider>
      <DagEditorInner {...props} />
    </ReactFlowProvider>
  );
}
