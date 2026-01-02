import { useEffect, useRef, useState, useCallback } from 'react';
import ForceGraph3D, {
  GraphNode as ForceGraphNode,
  GraphLink as ForceGraphLink,
} from '3d-force-graph';
import { ArrowLeft, GitBranch, Folder } from 'lucide-react';
import { Button } from '../ui/button';
import { getGraphInsights } from '../../api';
import type { GraphInsights, GraphNode as ApiGraphNode } from '../../api/types.gen';

interface ForceNode extends ForceGraphNode {
  name: string;
  nodeType: string;
  val: number;
  color: string;
  metadata?: ApiGraphNode['metadata'];
  x?: number;
  y?: number;
  z?: number;
}

interface GraphInsightsViewProps {
  onBack?: () => void;
}

export default function GraphInsightsView({ onBack }: GraphInsightsViewProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const graphRef = useRef<ReturnType<ReturnType<typeof ForceGraph3D>> | null>(null);
  const [data, setData] = useState<GraphInsights | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [hoveredNode, setHoveredNode] = useState<ForceNode | null>(null);

  useEffect(() => {
    const fetchData = async () => {
      try {
        setLoading(true);
        const response = await getGraphInsights();
        if (response.data) {
          setData(response.data);
        } else {
          setError('Failed to load graph data');
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Unknown error');
      } finally {
        setLoading(false);
      }
    };
    fetchData();
  }, []);

  const handleResize = useCallback(() => {
    if (graphRef.current && containerRef.current) {
      graphRef.current
        .width(containerRef.current.clientWidth)
        .height(containerRef.current.clientHeight);
    }
  }, []);

  useEffect(() => {
    if (!data || !containerRef.current || loading) return;

    if (graphRef.current) {
      if (graphRef.current._destructor) {
        graphRef.current._destructor();
      }
      graphRef.current = null;
    }

    containerRef.current.innerHTML = '';

    const graphData = {
      nodes: data.nodes.map((n) => ({
        id: n.id,
        name: n.name,
        nodeType: n.nodeType,
        val: Math.max(n.val || 1, 1),
        color: n.color || '#888888',
        metadata: n.metadata,
      })),
      links: data.links.map((l) => ({
        source: l.source,
        target: l.target,
        value: l.value,
      })),
    };

    const graph = ForceGraph3D()(containerRef.current)
      .graphData(graphData)
      .nodeId('id')
      .nodeVal('val')
      .nodeLabel((node) => {
        const n = node as ForceNode;
        const meta = n.metadata;
        if (!meta) return n.name;

        const typeIcon = meta.projectType === 'git' ? '🔀' : '📁';
        const dirList =
          meta.directories && meta.directories.length > 1
            ? `<br/><span style="color: #666; font-size: 10px;">${meta.directories.length} directories</span>`
            : '';

        return `<div style="text-align: center; padding: 8px; max-width: 300px;">
          <strong>${typeIcon} ${n.name}</strong>${dirList}
          ${meta.sessionCount ? `<br/>Sessions: ${meta.sessionCount}` : ''}
          ${meta.messageCount ? `<br/>Messages: ${meta.messageCount.toLocaleString()}` : ''}
          ${meta.tokenCount ? `<br/>Tokens: ${meta.tokenCount.toLocaleString()}` : ''}
        </div>`;
      })
      .nodeColor((node) => (node as ForceNode).color || '#888888')
      .nodeOpacity(0.9)
      .linkWidth((link: ForceGraphLink) => Math.sqrt(link.value || 1) * 0.5)
      .linkColor(() => 'rgba(255, 255, 255, 0.15)')
      .linkOpacity(0.4)
      .backgroundColor('#0a0a0a')
      .width(containerRef.current.clientWidth)
      .height(containerRef.current.clientHeight)
      .onNodeHover((node) => {
        setHoveredNode(node as ForceNode | null);
        if (containerRef.current) {
          containerRef.current.style.cursor = node ? 'pointer' : 'default';
        }
      })
      .onNodeClick((node) => {
        const n = node as ForceNode;
        const distance = 150;
        const distRatio = 1 + distance / Math.hypot(n.val || 1, n.val || 1);
        graph.cameraPosition(
          {
            x: (n.x || 0) * distRatio,
            y: (n.y || 0) * distRatio,
            z: (n.z || 0) * distRatio,
          },
          node,
          1000
        );
      });

    graphRef.current = graph;
    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      if (graphRef.current && graphRef.current._destructor) {
        graphRef.current._destructor();
      }
    };
  }, [data, loading, handleResize]);

  if (loading) {
    return (
      <div className="flex items-center justify-center h-screen bg-[#0a0a0a]">
        <div className="text-white text-lg">Loading project graph...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex flex-col items-center justify-center h-screen bg-[#0a0a0a] gap-4">
        <div className="text-red-400 text-lg">Error: {error}</div>
        {onBack && (
          <Button variant="outline" onClick={onBack}>
            <ArrowLeft className="w-4 h-4 mr-2" />
            Go Back
          </Button>
        )}
      </div>
    );
  }

  return (
    <div className="relative w-full h-screen bg-[#0a0a0a]">
      <div ref={containerRef} className="w-full h-full" />

      {onBack && (
        <div className="absolute top-4 left-4">
          <Button
            variant="outline"
            onClick={onBack}
            className="bg-black/50 border-white/20 text-white hover:bg-white/10"
          >
            <ArrowLeft className="w-4 h-4 mr-2" />
            Back
          </Button>
        </div>
      )}

      {data && (
        <div className="absolute top-4 right-4 bg-black/70 border border-white/20 rounded-lg p-4 text-white max-w-xs">
          <h3 className="text-lg font-semibold mb-2">Project Insights</h3>
          <div className="space-y-1 text-sm">
            <p>
              <span className="text-gray-400">Projects:</span>{' '}
              {data.summary.uniqueProjects?.toLocaleString() ?? 'N/A'}
            </p>
            <p>
              <span className="text-gray-400">Sessions (30d):</span>{' '}
              {data.nodes.filter((n) => n.nodeType === 'session').length}
            </p>
            <p>
              <span className="text-gray-400">Total Sessions:</span>{' '}
              {data.summary.totalSessions.toLocaleString()}
            </p>
            <p>
              <span className="text-gray-400">Messages:</span>{' '}
              {data.summary.totalMessages.toLocaleString()}
            </p>
          </div>
        </div>
      )}

      <div className="absolute bottom-4 left-4 bg-black/70 border border-white/20 rounded-lg p-4 text-white">
        <h4 className="text-sm font-semibold mb-2">Legend</h4>
        <div className="space-y-2 text-xs">
          <div className="flex items-center gap-2">
            <GitBranch className="w-3 h-3 text-[#6366F1]" />
            <div className="w-3 h-3 rounded-full bg-[#6366F1]" />
            <span>Git Project</span>
          </div>
          <div className="flex items-center gap-2">
            <Folder className="w-3 h-3 text-[#10B981]" />
            <div className="w-3 h-3 rounded-full bg-[#10B981]" />
            <span>Directory</span>
          </div>
          <div className="flex items-center gap-2">
            <div className="w-3 h-3 rounded-full bg-[#9CA3AF]" />
            <span>Session</span>
          </div>
        </div>
        <p className="text-[10px] text-gray-500 mt-2">Sessions from the last 30 days</p>
      </div>

      {hoveredNode && hoveredNode.nodeType === 'project' && (
        <div className="absolute bottom-4 right-4 bg-black/70 border border-white/20 rounded-lg p-4 text-white max-w-sm">
          <div className="flex items-center gap-2 mb-2">
            {hoveredNode.metadata?.projectType === 'git' ? (
              <GitBranch className="w-4 h-4 text-[#6366F1]" />
            ) : (
              <Folder className="w-4 h-4 text-[#10B981]" />
            )}
            <h4 className="text-sm font-semibold">{hoveredNode.name}</h4>
          </div>
          {hoveredNode.metadata && (
            <div className="space-y-1 text-xs">
              {hoveredNode.metadata.sessionCount != null && (
                <p>Sessions: {hoveredNode.metadata.sessionCount.toLocaleString()}</p>
              )}
              {hoveredNode.metadata.messageCount != null && (
                <p>Messages: {hoveredNode.metadata.messageCount.toLocaleString()}</p>
              )}
              {hoveredNode.metadata.tokenCount != null && (
                <p>Tokens: {hoveredNode.metadata.tokenCount.toLocaleString()}</p>
              )}
              {hoveredNode.metadata.directories && hoveredNode.metadata.directories.length > 0 && (
                <div className="mt-2 pt-2 border-t border-white/10">
                  <p className="text-gray-400 mb-1">Directories:</p>
                  <div className="max-h-24 overflow-y-auto">
                    {hoveredNode.metadata.directories.map((dir, i) => (
                      <p key={i} className="text-gray-500 truncate text-[10px]">
                        {dir}
                      </p>
                    ))}
                  </div>
                </div>
              )}
              {hoveredNode.metadata.firstActivity && hoveredNode.metadata.lastActivity && (
                <p className="text-gray-400 mt-1">
                  {new Date(hoveredNode.metadata.firstActivity).toLocaleDateString()} -{' '}
                  {new Date(hoveredNode.metadata.lastActivity).toLocaleDateString()}
                </p>
              )}
            </div>
          )}
        </div>
      )}

      {hoveredNode && hoveredNode.nodeType === 'session' && (
        <div className="absolute bottom-4 right-4 bg-black/70 border border-white/20 rounded-lg p-4 text-white max-w-sm">
          <h4 className="text-sm font-semibold mb-2">{hoveredNode.name}</h4>
          {hoveredNode.metadata && (
            <div className="space-y-1 text-xs">
              {hoveredNode.metadata.messageCount != null && (
                <p>Messages: {hoveredNode.metadata.messageCount}</p>
              )}
              {hoveredNode.metadata.firstActivity && (
                <p className="text-gray-400">
                  {new Date(hoveredNode.metadata.firstActivity).toLocaleDateString()}
                </p>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
