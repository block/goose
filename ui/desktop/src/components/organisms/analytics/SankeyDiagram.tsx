/**
 * Sankey Diagram for Routing Flow Visualization
 * Shows Expected Agent → Actual Agent routing paths
 * Path width proportional to case count
 */
import { useMemo } from 'react';

interface SankeyFlow {
  source: string;
  target: string;
  value: number;
}

interface SankeyProps {
  labels: string[];
  matrix: number[][];
  height?: number;
}

interface SankeyNode {
  id: string;
  label: string;
  x: number;
  y: number;
  height: number;
  total: number;
  side: 'left' | 'right';
}

interface SankeyLink {
  source: string;
  target: string;
  value: number;
  sourceY: number;
  targetY: number;
  width: number;
  isCorrect: boolean;
}

const COLORS = [
  '#818cf8', // indigo
  '#34d399', // emerald
  '#f97316', // orange
  '#06b6d4', // cyan
  '#f472b6', // pink
  '#a78bfa', // violet
  '#fbbf24', // amber
  '#10b981', // green
  '#ef4444', // red
  '#3b82f6', // blue
];

function getColor(index: number): string {
  return COLORS[index % COLORS.length];
}

export default function SankeyDiagram({ labels, matrix, height: propHeight }: SankeyProps) {
  const { nodes, links, width, height, totalFlows } = useMemo(() => {
    if (!labels.length || !matrix.length) {
      return { nodes: [], links: [], width: 600, height: 200, totalFlows: 0 };
    }

    // Extract flows from confusion matrix
    const flows: SankeyFlow[] = [];
    let totalFlows = 0;
    for (let i = 0; i < labels.length; i++) {
      for (let j = 0; j < labels.length; j++) {
        if (matrix[i] && matrix[i][j] > 0) {
          flows.push({
            source: `expected-${labels[i]}`,
            target: `actual-${labels[j]}`,
            value: matrix[i][j],
          });
          totalFlows += matrix[i][j];
        }
      }
    }

    // Calculate node totals
    const leftTotals = labels.map((_, i) => (matrix[i] ? matrix[i].reduce((s, v) => s + v, 0) : 0));
    const rightTotals = labels.map((_, j) => matrix.reduce((s, row) => s + (row[j] || 0), 0));

    const maxTotal = Math.max(...leftTotals, ...rightTotals, 1);
    const padding = 40;
    const nodeWidth = 24;
    const w = 600;
    const minNodeHeight = 20;
    const nodeGap = 8;

    // Calculate heights
    const availableHeight = propHeight || Math.max(300, labels.length * 60 + padding * 2);
    const usableHeight = availableHeight - padding * 2;

    // Build left nodes (Expected)
    const leftNodes: SankeyNode[] = [];
    let leftY = padding;
    for (let i = 0; i < labels.length; i++) {
      const h = Math.max(minNodeHeight, (leftTotals[i] / maxTotal) * usableHeight * 0.6);
      leftNodes.push({
        id: `expected-${labels[i]}`,
        label: labels[i],
        x: padding,
        y: leftY,
        height: h,
        total: leftTotals[i],
        side: 'left',
      });
      leftY += h + nodeGap;
    }

    // Build right nodes (Actual)
    const rightNodes: SankeyNode[] = [];
    let rightY = padding;
    for (let j = 0; j < labels.length; j++) {
      const h = Math.max(minNodeHeight, (rightTotals[j] / maxTotal) * usableHeight * 0.6);
      rightNodes.push({
        id: `actual-${labels[j]}`,
        label: labels[j],
        x: w - padding - nodeWidth,
        y: rightY,
        height: h,
        total: rightTotals[j],
        side: 'right',
      });
      rightY += h + nodeGap;
    }

    const allNodes = [...leftNodes, ...rightNodes];
    const actualHeight = Math.max(leftY, rightY) + padding;

    // Build links with positioned offsets
    const sourceOffsets: Record<string, number> = {};
    const targetOffsets: Record<string, number> = {};
    allNodes.forEach((n) => {
      sourceOffsets[n.id] = 0;
      targetOffsets[n.id] = 0;
    });

    const sankeyLinks: SankeyLink[] = [];
    for (const flow of flows) {
      const sourceNode = allNodes.find((n) => n.id === flow.source);
      const targetNode = allNodes.find((n) => n.id === flow.target);
      if (!sourceNode || !targetNode) continue;

      const sourceTotal = sourceNode.total || 1;
      const targetTotal = targetNode.total || 1;
      const sourceWidth = (flow.value / sourceTotal) * sourceNode.height;
      const targetWidth = (flow.value / targetTotal) * targetNode.height;
      const linkWidth = Math.max(sourceWidth, targetWidth, 2);

      sankeyLinks.push({
        source: flow.source,
        target: flow.target,
        value: flow.value,
        sourceY: sourceNode.y + sourceOffsets[flow.source],
        targetY: targetNode.y + targetOffsets[flow.target],
        width: linkWidth,
        isCorrect: flow.source.replace('expected-', '') === flow.target.replace('actual-', ''),
      });

      sourceOffsets[flow.source] += sourceWidth;
      targetOffsets[flow.target] += targetWidth;
    }

    return {
      nodes: allNodes,
      links: sankeyLinks,
      width: w,
      height: propHeight || actualHeight,
      totalFlows,
    };
  }, [labels, matrix, propHeight]);

  if (!labels.length || totalFlows === 0) {
    return (
      <div className="flex items-center justify-center h-32 text-text-subtle text-sm">
        No routing flow data available
      </div>
    );
  }

  const nodeWidth = 24;
  const padding = 40;

  return (
    <div className="w-full overflow-x-auto">
      {/* Column headers */}
      <div className="flex justify-between px-10 mb-1">
        <span className="text-xs font-medium text-text-muted uppercase tracking-wider">
          Expected Agent
        </span>
        <span className="text-xs font-medium text-text-muted uppercase tracking-wider">
          Actual Agent
        </span>
      </div>

      <svg
        viewBox={`0 0 ${width} ${height}`}
        className="w-full"
        style={{ minHeight: Math.min(height, 500) }}
      >
        {/* Links */}
        {links.map((link) => {
          const sourceX = padding + nodeWidth;
          const targetX = width - padding - nodeWidth;
          const midX = (sourceX + targetX) / 2;
          const sy = link.sourceY + link.width / 2;
          const ty = link.targetY + link.width / 2;

          return (
            <g key={`${link.source}-${link.target}`}>
              <path
                d={`M ${sourceX} ${sy} C ${midX} ${sy}, ${midX} ${ty}, ${targetX} ${ty}`}
                fill="none"
                stroke={link.isCorrect ? '#22c55e' : '#ef4444'}
                strokeWidth={Math.max(link.width, 1.5)}
                strokeOpacity={link.isCorrect ? 0.35 : 0.5}
                className="transition-all duration-200 hover:stroke-opacity-80"
              />
              {/* Flow label on hover */}
              <title>
                {link.source.replace('expected-', '')} → {link.target.replace('actual-', '')}:{' '}
                {link.value} case{link.value !== 1 ? 's' : ''}
                {link.isCorrect ? ' ✓' : ' ✗'}
              </title>
            </g>
          );
        })}

        {/* Left nodes (Expected) */}
        {nodes
          .filter((n) => n.side === 'left')
          .map((node, i) => (
            <g key={node.id}>
              <rect
                x={node.x}
                y={node.y}
                width={nodeWidth}
                height={node.height}
                rx={4}
                fill={getColor(i)}
                fillOpacity={0.8}
              />
              <text
                x={node.x + nodeWidth + 6}
                y={node.y + node.height / 2}
                dominantBaseline="central"
                className="fill-zinc-300 text-[11px]"
              >
                {node.label} ({node.total})
              </text>
            </g>
          ))}

        {/* Right nodes (Actual) */}
        {nodes
          .filter((n) => n.side === 'right')
          .map((node, i) => (
            <g key={node.id}>
              <rect
                x={node.x}
                y={node.y}
                width={nodeWidth}
                height={node.height}
                rx={4}
                fill={getColor(i)}
                fillOpacity={0.8}
              />
              <text
                x={node.x - 6}
                y={node.y + node.height / 2}
                dominantBaseline="central"
                textAnchor="end"
                className="fill-zinc-300 text-[11px]"
              >
                {node.label} ({node.total})
              </text>
            </g>
          ))}

        {/* Legend */}
        <g transform={`translate(${width / 2 - 80}, ${height - 20})`}>
          <rect x={0} y={0} width={12} height={12} rx={2} fill="#22c55e" fillOpacity={0.5} />
          <text x={16} y={10} className="fill-zinc-400 text-[10px]">
            Correct routing
          </text>
          <rect x={100} y={0} width={12} height={12} rx={2} fill="#ef4444" fillOpacity={0.5} />
          <text x={116} y={10} className="fill-zinc-400 text-[10px]">
            Misrouted
          </text>
        </g>
      </svg>
    </div>
  );
}
