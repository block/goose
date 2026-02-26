/**
 * Serialization utilities: Pipeline YAML ↔ React Flow nodes/edges.
 */
import type { Edge, Node } from '@xyflow/react';
import { defaultConfig, type DagNodeData, type NodeKind, type Pipeline, type PipelineMetadata, type PipelineNode } from './types';

const VERTICAL_SPACING = 120;
const HORIZONTAL_SPACING = 280;

/**
 * Convert a Pipeline definition to React Flow nodes and edges.
 */
export function pipelineToFlow(pipeline: Pipeline): {
  nodes: Node<DagNodeData>[];
  edges: Edge[];
} {
  const nodes: Node<DagNodeData>[] = [];
  const edges: Edge[] = [];

  // Build dependency graph for layout
  const nodeMap = new Map<string, PipelineNode>();
  const inDegree = new Map<string, number>();
  const children = new Map<string, string[]>();

  for (const node of pipeline.nodes) {
    nodeMap.set(node.id, node);
    inDegree.set(node.id, 0);
    children.set(node.id, []);
  }

  for (const node of pipeline.nodes) {
    if (node.depends) {
      for (const dep of node.depends) {
        inDegree.set(node.id, (inDegree.get(node.id) ?? 0) + 1);
        children.get(dep)?.push(node.id);
      }
    }
  }

  // Topological sort for layered layout
  const layers: string[][] = [];
  const remaining = new Set(pipeline.nodes.map((n) => n.id));
  const placed = new Set<string>();
  while (remaining.size > 0) {
    const layer: string[] = [];
    for (const id of remaining) {
      const node = nodeMap.get(id);
      if (!node) {
        continue;
      }

      const deps = node.depends ?? [];
      if (deps.every((d) => placed.has(d))) {
        layer.push(id);
      }
    }
    if (layer.length === 0) {
      // Cycle detected — place remaining nodes
      layer.push(...remaining);
      remaining.clear();
    }
    for (const id of layer) {
      remaining.delete(id);
      placed.add(id);
    }
    layers.push(layer);
  }

  // Position nodes in layers
  for (let layerIdx = 0; layerIdx < layers.length; layerIdx++) {
    const layer = layers[layerIdx];
    const layerWidth = layer.length * HORIZONTAL_SPACING;
    const startX = -layerWidth / 2 + HORIZONTAL_SPACING / 2;

    for (let i = 0; i < layer.length; i++) {
      const nodeId = layer[i];
      const pNode = nodeMap.get(nodeId);
      if (!pNode) {
        continue;
      }

      nodes.push({
        id: nodeId,
        type: pNode.type,
        position: {
          x: startX + i * HORIZONTAL_SPACING,
          y: layerIdx * VERTICAL_SPACING,
        },
        data: {
          kind: pNode.type,
          label: pNode.label ?? pNode.id,
          config: pNode.config,
          condition: pNode.condition,
          status: 'idle',
        },
      });

      // Create edges from dependencies
      if (pNode.depends) {
        for (const dep of pNode.depends) {
          edges.push({
            id: `${dep}->${nodeId}`,
            source: dep,
            target: nodeId,
            animated: false,
            style: { strokeWidth: 2 },
          });
        }
      }
    }
  }

  return { nodes, edges };
}

/**
 * Convert React Flow nodes and edges back to a Pipeline definition.
 */
export function flowToPipeline(
  nodes: Node<DagNodeData>[],
  edges: Edge[],
  metadata: PipelineMetadata
): Pipeline {
  // Build reverse edge map: target → sources
  const incomingEdges = new Map<string, string[]>();
  for (const edge of edges) {
    const deps = incomingEdges.get(edge.target) ?? [];
    deps.push(edge.source);
    incomingEdges.set(edge.target, deps);
  }

  const pipelineNodes: PipelineNode[] = nodes.map((node) => {
    const data = node.data as DagNodeData;
    const depends = incomingEdges.get(node.id);

    return {
      id: node.id,
      type: data.kind,
      label: data.label,
      config: data.config,
      depends: depends && depends.length > 0 ? depends : undefined,
      condition: data.condition,
    };
  });

  return {
    apiVersion: 'goose/v1',
    kind: 'Pipeline',
    metadata,
    nodes: pipelineNodes,
  };
}

/**
 * Create a new React Flow node from a palette drop.
 */
export function createNode(kind: NodeKind, position: { x: number; y: number }): Node<DagNodeData> {
  const id = `${kind}_${Date.now().toString(36)}`;
  return {
    id,
    type: kind,
    position,
    data: {
      kind,
      label: `New ${kind}`,
      config: defaultConfig(kind),
      status: 'idle',
    },
  };
}

/**
 * Serialize a pipeline to YAML string (simple implementation).
 */
export function pipelineToYaml(pipeline: Pipeline): string {
  const lines: string[] = [];
  lines.push(`apiVersion: ${pipeline.apiVersion}`);
  lines.push(`kind: ${pipeline.kind}`);
  lines.push('metadata:');
  lines.push(`  name: ${pipeline.metadata.name}`);
  lines.push(`  description: "${pipeline.metadata.description}"`);
  if (pipeline.metadata.tags?.length) {
    lines.push(`  tags: [${pipeline.metadata.tags.join(', ')}]`);
  }
  lines.push('nodes:');

  for (const node of pipeline.nodes) {
    lines.push(`  - id: ${node.id}`);
    lines.push(`    type: ${node.type}`);
    if (node.label) lines.push(`    label: "${node.label}"`);
    if (node.depends?.length) {
      lines.push(`    depends: [${node.depends.join(', ')}]`);
    }
    if (node.condition) {
      lines.push(`    condition: "${node.condition}"`);
    }
    lines.push('    config:');
    for (const [key, value] of Object.entries(node.config)) {
      if (value !== undefined && value !== null && value !== '') {
        if (typeof value === 'object') {
          lines.push(`      ${key}:`);
          for (const [k, v] of Object.entries(value)) {
            lines.push(`        ${k}: "${v}"`);
          }
        } else if (typeof value === 'string' && value.includes('\n')) {
          lines.push(`      ${key}: |`);
          for (const line of value.split('\n')) {
            lines.push(`        ${line}`);
          }
        } else {
          lines.push(`      ${key}: ${typeof value === 'string' ? `"${value}"` : value}`);
        }
      }
    }
  }

  return lines.join('\n');
}

/**
 * Export a pipeline as JSON (for clipboard / API).
 */
export function pipelineToJson(pipeline: Pipeline): string {
  return JSON.stringify(pipeline, null, 2);
}
