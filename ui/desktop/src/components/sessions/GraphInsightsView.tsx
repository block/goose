import { useEffect, useRef, useState, useCallback } from 'react';
import ForceGraph3D, {
  GraphNode as ForceGraphNode,
  GraphLink as ForceGraphLink,
} from '3d-force-graph';
import { ArrowLeft } from 'lucide-react';
import { Button } from '../ui/button';
import { getGraphInsights } from '../../api';
import type { GraphInsights, GraphNode as ApiGraphNode } from '../../api/types.gen';

// Extended node type for force graph with position data
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

  // Fetch graph data
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

  // Handle window resize
  const handleResize = useCallback(() => {
    if (graphRef.current && containerRef.current) {
      graphRef.current
        .width(containerRef.current.clientWidth)
        .height(containerRef.current.clientHeight);
    }
  }, []);

  // Initialize graph
  useEffect(() => {
    if (!data || !containerRef.current || loading) return;

    // Clean up any existing graph
    if (graphRef.current) {
      if (graphRef.current._destructor) {
        graphRef.current._destructor();
      }
      graphRef.current = null;
    }

    // Clear container
    containerRef.current.innerHTML = '';

    // Build graph data
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

    // Create the graph - 3d-force-graph has adequate default lighting
    const graph = ForceGraph3D()(containerRef.current)
      .graphData(graphData)
      .nodeId('id')
      .nodeVal('val')
      .nodeLabel((node) => {
        const n = node as ForceNode;
        const meta = n.metadata;
        if (!meta) return n.name;
        return `<div style="text-align: center; padding: 8px;">
          <strong>${n.name}</strong><br/>
          <span style="color: #aaa;">${n.nodeType}</span><br/>
          ${meta.sessionCount ? `Sessions: ${meta.sessionCount}` : ''}
          ${meta.messageCount ? `<br/>Messages: ${meta.messageCount.toLocaleString()}` : ''}
          ${meta.tokenCount ? `<br/>Tokens: ${meta.tokenCount.toLocaleString()}` : ''}
        </div>`;
      })
      .nodeColor((node) => (node as ForceNode).color || '#888888')
      .nodeOpacity(0.9)
      .linkWidth((link: ForceGraphLink) => Math.sqrt(link.value || 1) * 0.5)
      .linkColor(() => 'rgba(255, 255, 255, 0.2)')
      .linkOpacity(0.6)
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
        const distance = 200;
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

    // Handle resize
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
        <div className="text-white text-lg">Loading graph data...</div>
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
      {/* Graph container */}
      <div ref={containerRef} className="w-full h-full" />

      {/* Back button */}
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

      {/* Summary panel */}
      {data && (
        <div className="absolute top-4 right-4 bg-black/70 border border-white/20 rounded-lg p-4 text-white max-w-xs">
          <h3 className="text-lg font-semibold mb-2">Session Insights</h3>
          <div className="space-y-1 text-sm">
            <p>
              <span className="text-gray-400">Sessions:</span>{' '}
              {data.summary.totalSessions.toLocaleString()}
            </p>
            <p>
              <span className="text-gray-400">Messages:</span>{' '}
              {data.summary.totalMessages.toLocaleString()}
            </p>
            <p>
              <span className="text-gray-400">Tokens:</span>{' '}
              {data.summary.totalTokens.toLocaleString()}
            </p>
            <p>
              <span className="text-gray-400">Directories:</span>{' '}
              {data.summary.uniqueDirectories.toLocaleString()}
            </p>
          </div>
        </div>
      )}

      {/* Legend */}
      <div className="absolute bottom-4 left-4 bg-black/70 border border-white/20 rounded-lg p-4 text-white">
        <h4 className="text-sm font-semibold mb-2">Legend</h4>
        <div className="space-y-1 text-xs">
          <div className="flex items-center gap-2">
            <div className="w-3 h-3 rounded-full bg-[#4CAF50]" />
            <span>Directories</span>
          </div>
          <div className="flex items-center gap-2">
            <div className="w-3 h-3 rounded-full bg-[#FF5722]" />
            <span>Providers</span>
          </div>
          <div className="flex items-center gap-2">
            <div className="w-3 h-3 rounded-full bg-[#2196F3]" />
            <span>Session Types</span>
          </div>
          <div className="flex items-center gap-2">
            <div className="w-3 h-3 rounded-full bg-[#FF6B35]" />
            <span>Hub</span>
          </div>
        </div>
      </div>

      {/* Hovered node details */}
      {hoveredNode && (
        <div className="absolute bottom-4 right-4 bg-black/70 border border-white/20 rounded-lg p-4 text-white max-w-sm">
          <h4 className="text-sm font-semibold mb-2">{hoveredNode.name}</h4>
          <p className="text-xs text-gray-400 mb-2">{hoveredNode.nodeType}</p>
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
              {hoveredNode.metadata.firstActivity && (
                <p className="text-gray-400">
                  First: {new Date(hoveredNode.metadata.firstActivity).toLocaleDateString()}
                </p>
              )}
              {hoveredNode.metadata.lastActivity && (
                <p className="text-gray-400">
                  Last: {new Date(hoveredNode.metadata.lastActivity).toLocaleDateString()}
                </p>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
