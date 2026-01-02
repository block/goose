import { useEffect, useRef, useState, useCallback } from 'react';
import ForceGraph3D, {
  GraphNode as ForceGraphNode,
  GraphLink as ForceGraphLink,
} from '3d-force-graph';
import * as THREE from 'three';
import { ArrowLeft, Clock, Link2, Box, Circle, AlertCircle } from 'lucide-react';
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

const LINK_COLORS = {
  temporal: 'rgba(251, 191, 36, 0.4)',
  similar: 'rgba(236, 72, 153, 0.5)',
  default: 'rgba(255, 255, 255, 0.15)',
};

// Neutral colors for shapes (not type-coded)
const NODE_MATERIAL = {
  project: 0xcccccc, // Light gray for all projects
  session: 0x666666, // Darker gray for sessions
  dirty: 0xff4444, // Red for dirty indicator ball
};

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
        linkType: l.linkType ?? undefined,
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

        // Shape description
        const shapeDesc = meta.projectType === 'git' ? '⬡ Cube' : '● Sphere';
        // Git status indicator
        const gitStatus =
          meta.projectType === 'git'
            ? meta.gitDirty === true
              ? ' (dirty)'
              : meta.gitDirty === false
                ? ' (clean)'
                : ''
            : '';

        const dirList =
          meta.directories && meta.directories.length > 1
            ? `<br/><span style="color: #666; font-size: 10px;">${meta.directories.length} directories</span>`
            : '';

        return `<div style="text-align: center; padding: 8px; max-width: 300px;">
          <strong>${n.name}</strong>
          <br/><span style="color: #888; font-size: 11px;">${shapeDesc}${gitStatus}</span>${dirList}
          ${meta.sessionCount ? `<br/>Sessions: ${meta.sessionCount}` : ''}
          ${meta.messageCount ? `<br/>Messages: ${meta.messageCount.toLocaleString()}` : ''}
          ${meta.tokenCount ? `<br/>Tokens: ${meta.tokenCount.toLocaleString()}` : ''}
        </div>`;
      })
      .nodeThreeObject((node: ForceGraphNode) => {
        const n = node as ForceNode;
        const size = Math.cbrt(n.val || 1) * 4; // Cube root for better scaling

        if (n.nodeType === 'project') {
          const isGitRepo = n.metadata?.projectType === 'git';
          const isDirty = n.metadata?.gitDirty === true;

          if (isGitRepo) {
            // Git repo = Cube
            const geometry = new THREE.BoxGeometry(size, size, size);
            const material = new THREE.MeshLambertMaterial({
              color: NODE_MATERIAL.project,
              transparent: true,
              opacity: 0.9,
            });
            const cube = new THREE.Mesh(geometry, material);

            if (isDirty) {
              // Add small red ball to indicate dirty status
              const dirtyGeometry = new THREE.SphereGeometry(size * 0.3, 16, 16);
              const dirtyMaterial = new THREE.MeshLambertMaterial({
                color: NODE_MATERIAL.dirty,
                transparent: true,
                opacity: 0.95,
              });
              const dirtyBall = new THREE.Mesh(dirtyGeometry, dirtyMaterial);
              dirtyBall.position.set(size * 0.6, size * 0.6, size * 0.6);

              const group = new THREE.Group();
              group.add(cube);
              group.add(dirtyBall);
              return group;
            }

            return cube;
          } else {
            // Directory = Sphere
            const geometry = new THREE.SphereGeometry(size * 0.6, 32, 32);
            const material = new THREE.MeshLambertMaterial({
              color: NODE_MATERIAL.project,
              transparent: true,
              opacity: 0.9,
            });
            return new THREE.Mesh(geometry, material);
          }
        } else {
          // Session = Small sphere
          const geometry = new THREE.SphereGeometry(size * 0.4, 16, 16);
          const material = new THREE.MeshLambertMaterial({
            color: NODE_MATERIAL.session,
            transparent: true,
            opacity: 0.7,
          });
          return new THREE.Mesh(geometry, material);
        }
      })
      .linkWidth((link: ForceGraphLink) => {
        const lt = link.linkType;
        if (lt === 'temporal') return 1.5;
        if (lt === 'similar') return 1.0;
        return Math.sqrt(link.value || 1) * 0.5;
      })
      .linkColor((link: ForceGraphLink) => {
        const lt = link.linkType;
        if (lt === 'temporal') return LINK_COLORS.temporal;
        if (lt === 'similar') return LINK_COLORS.similar;
        return LINK_COLORS.default;
      })
      .linkOpacity(0.6)
      .backgroundColor('#0a0a0a')
      .width(containerRef.current.clientWidth)
      .height(containerRef.current.clientHeight)
      .onNodeHover((node: ForceGraphNode | null) => {
        setHoveredNode(node as ForceNode | null);
        if (containerRef.current) {
          containerRef.current.style.cursor = node ? 'pointer' : 'default';
        }
      })
      .onNodeClick((node: ForceGraphNode) => {
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

  // Count stats for legend
  const stats = data
    ? {
        temporalLinks: data.links.filter((l) => l.linkType === 'temporal').length,
        similarLinks: data.links.filter((l) => l.linkType === 'similar').length,
        gitProjects: data.nodes.filter(
          (n) => n.nodeType === 'project' && n.metadata?.projectType === 'git'
        ).length,
        dirProjects: data.nodes.filter(
          (n) => n.nodeType === 'project' && n.metadata?.projectType === 'dir'
        ).length,
        dirtyRepos: data.nodes.filter(
          (n) => n.nodeType === 'project' && n.metadata?.gitDirty === true
        ).length,
        cleanRepos: data.nodes.filter(
          (n) => n.nodeType === 'project' && n.metadata?.gitDirty === false
        ).length,
        sessions: data.nodes.filter((n) => n.nodeType === 'session').length,
      }
    : {
        temporalLinks: 0,
        similarLinks: 0,
        gitProjects: 0,
        dirProjects: 0,
        dirtyRepos: 0,
        cleanRepos: 0,
        sessions: 0,
      };

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
              <span className="text-gray-500 text-xs ml-1">
                ({stats.gitProjects} git, {stats.dirProjects} dirs)
              </span>
            </p>
            <p>
              <span className="text-gray-400">Sessions (30d):</span> {stats.sessions}
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
          <div className="font-medium text-gray-400 mb-1">Nodes (by shape)</div>
          <div className="flex items-center gap-2">
            <Box className="w-4 h-4 text-gray-300" />
            <span>Git Repository ({stats.gitProjects})</span>
          </div>
          <div className="flex items-center gap-2 pl-5">
            <span className="text-gray-400">Clean: {stats.cleanRepos}</span>
          </div>
          <div className="flex items-center gap-2 pl-5">
            <AlertCircle className="w-3 h-3 text-red-400" />
            <span className="text-gray-400">With red ball = dirty ({stats.dirtyRepos})</span>
          </div>
          <div className="flex items-center gap-2">
            <Circle className="w-4 h-4 text-gray-300" />
            <span>Directory ({stats.dirProjects})</span>
          </div>
          <div className="flex items-center gap-2">
            <div className="w-3 h-3 rounded-full bg-gray-500" />
            <span>Session ({stats.sessions})</span>
          </div>

          <div className="font-medium text-gray-400 mt-3 mb-1">Links</div>
          <div className="flex items-center gap-2">
            <Clock className="w-3 h-3 text-amber-400" />
            <div className="w-6 h-0.5 bg-amber-400/60" />
            <span>Same day ({stats.temporalLinks})</span>
          </div>
          <div className="flex items-center gap-2">
            <Link2 className="w-3 h-3 text-pink-400" />
            <div className="w-6 h-0.5 bg-pink-400/60" />
            <span>Similar sessions ({stats.similarLinks})</span>
          </div>
        </div>
        <p className="text-[10px] text-gray-500 mt-3">Sessions from the last 30 days</p>
      </div>

      {hoveredNode && hoveredNode.nodeType === 'project' && (
        <div className="absolute bottom-4 right-4 bg-black/70 border border-white/20 rounded-lg p-4 text-white max-w-sm">
          <div className="flex items-center gap-2 mb-2">
            {hoveredNode.metadata?.projectType === 'git' ? (
              <Box className="w-4 h-4 text-gray-300" />
            ) : (
              <Circle className="w-4 h-4 text-gray-300" />
            )}
            <h4 className="text-sm font-semibold">{hoveredNode.name}</h4>
            {hoveredNode.metadata?.projectType === 'git' &&
              hoveredNode.metadata.gitDirty === true && (
                <span className="text-red-400 text-xs">(dirty)</span>
              )}
          </div>
          {hoveredNode.metadata && (
            <div className="space-y-1 text-xs">
              <p className="text-gray-400">
                {hoveredNode.metadata.projectType === 'git' ? 'Git Repository' : 'Directory'}
                {hoveredNode.metadata.gitDirty === true && ' • Uncommitted changes'}
                {hoveredNode.metadata.gitDirty === false && ' • Clean'}
              </p>
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
              {hoveredNode.metadata.sessionName && (
                <p className="text-gray-400 italic">{hoveredNode.metadata.sessionName}</p>
              )}
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
