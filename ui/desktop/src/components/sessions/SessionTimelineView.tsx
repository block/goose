import React, { useEffect, useRef, useMemo } from 'react';
import * as d3 from 'd3';
import { Session } from '../../api/types.gen';
import { formatDistanceToNow } from 'date-fns';

interface SessionTimelineViewProps {
  sessions: Session[];
  onSessionClick: (sessionId: string) => void;
}

interface TreeNode extends d3.HierarchyNode<SessionData> {
  x: number;
  y: number;
  session: SessionData;
}

interface SessionData {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
  message_count: number;
  chat_type: string;
  children?: SessionData[];
}

const SessionTimelineView: React.FC<SessionTimelineViewProps> = ({ 
  sessions, 
  onSessionClick 
}) => {
  const svgRef = useRef<SVGSVGElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Process sessions into hierarchical data
  const treeData = useMemo(() => {
    console.log('SessionTimelineView: Processing sessions', { sessionCount: sessions.length });
    
    if (sessions.length === 0) {
      return null;
    }

    // Sort sessions by creation date (newest first for "today at top")
    const sortedSessions = [...sessions]
      .sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())
      .slice(0, 100); // Limit for performance

    console.log('SessionTimelineView: Sorted sessions', sortedSessions.length);

    // Group sessions by day
    const sessionsByDay = new Map<string, SessionData[]>();
    
    sortedSessions.forEach(session => {
      const day = new Date(session.created_at).toDateString();
      if (!sessionsByDay.has(day)) {
        sessionsByDay.set(day, []);
      }
      sessionsByDay.get(day)!.push({
        id: session.id,
        title: session.title || 'Untitled Session',
        created_at: session.created_at,
        updated_at: session.updated_at,
        message_count: session.message_count || 0,
        chat_type: session.chat_type || 'regular'
      });
    });

    // Create hierarchical structure
    const days = Array.from(sessionsByDay.entries())
      .sort(([a], [b]) => new Date(b).getTime() - new Date(a).getTime()) // Newest first
      .map(([day, daySessions]) => ({
        id: `day-${day}`,
        title: day === new Date().toDateString() ? 'Today' : 
               day === new Date(Date.now() - 24 * 60 * 60 * 1000).toDateString() ? 'Yesterday' :
               formatDistanceToNow(new Date(day), { addSuffix: true }),
        created_at: day,
        updated_at: day,
        message_count: daySessions.reduce((sum, s) => sum + s.message_count, 0),
        chat_type: 'day',
        children: daySessions
      }));

    const rootData: SessionData = {
      id: 'root',
      title: 'Timeline',
      created_at: new Date().toISOString(),
      updated_at: new Date().toISOString(),
      message_count: sessions.length,
      chat_type: 'root',
      children: days
    };

    console.log('SessionTimelineView: Created tree data', { 
      dayCount: days.length,
      totalSessions: sortedSessions.length 
    });

    return rootData;
  }, [sessions]);

  useEffect(() => {
    if (!treeData || !svgRef.current || !containerRef.current) return;

    const svg = d3.select(svgRef.current);
    const container = containerRef.current;
    
    // Clear previous content
    svg.selectAll("*").remove();

    // Set up dimensions
    const containerRect = container.getBoundingClientRect();
    const width = Math.max(800, containerRect.width);
    const height = Math.max(600, treeData.children!.length * 200);

    svg.attr("width", width).attr("height", height);

    // Create hierarchy
    const root = d3.hierarchy(treeData);
    
    // Create tree layout
    const treeLayout = d3.tree<SessionData>()
      .size([height - 100, width - 200])
      .separation((a, b) => {
        // More separation for day nodes
        if (a.parent === root || b.parent === root) return 2;
        return 1;
      });

    // Apply layout
    const treeNodes = treeLayout(root);

    // Create curved link generator
    const linkGenerator = d3.linkHorizontal<any, TreeNode>()
      .x(d => d.y + 100)
      .y(d => d.x + 50);

    // Create container group
    const g = svg.append("g");

    // Draw links
    const links = g.selectAll(".link")
      .data(treeNodes.links())
      .enter()
      .append("path")
      .attr("class", "link")
      .attr("d", linkGenerator as any)
      .attr("fill", "none")
      .attr("stroke", (d: any) => {
        // Different colors for different levels
        if (d.source.depth === 0) return "#6366f1"; // Root to day
        return "#94a3b8"; // Day to session
      })
      .attr("stroke-width", (d: any) => {
        if (d.source.depth === 0) return 3;
        return 2;
      })
      .attr("opacity", 0.7);

    // Add tangled connections between similar sessions
    const sessionNodes = treeNodes.descendants().filter(d => d.depth === 2);
    
    // Create tangled connections based on similar titles or types
    const tangledConnections: Array<[TreeNode, TreeNode]> = [];
    
    for (let i = 0; i < sessionNodes.length; i++) {
      for (let j = i + 1; j < sessionNodes.length; j++) {
        const nodeA = sessionNodes[i] as TreeNode;
        const nodeB = sessionNodes[j] as TreeNode;
        
        // Connect sessions with similar characteristics
        const titleSimilarity = calculateTitleSimilarity(nodeA.data.title, nodeB.data.title);
        const typeSimilarity = nodeA.data.chat_type === nodeB.data.chat_type ? 0.5 : 0;
        
        if (titleSimilarity + typeSimilarity > 0.3 && tangledConnections.length < 20) {
          tangledConnections.push([nodeA, nodeB]);
        }
      }
    }

    // Draw tangled connections
    g.selectAll(".tangled-link")
      .data(tangledConnections)
      .enter()
      .append("path")
      .attr("class", "tangled-link")
      .attr("d", ([source, target]) => {
        const sx = source.y + 100;
        const sy = source.x + 50;
        const tx = target.y + 100;
        const ty = target.x + 50;
        
        // Create curved path
        const mx = (sx + tx) / 2;
        const my = (sy + ty) / 2;
        const dx = tx - sx;
        const dy = ty - sy;
        const dr = Math.sqrt(dx * dx + dy * dy);
        
        return `M${sx},${sy}Q${mx + dr * 0.1},${my - dr * 0.1} ${tx},${ty}`;
      })
      .attr("fill", "none")
      .attr("stroke", "#f59e0b")
      .attr("stroke-width", 1)
      .attr("stroke-dasharray", "3,3")
      .attr("opacity", 0.4);

    // Draw nodes
    const nodes = g.selectAll(".node")
      .data(treeNodes.descendants())
      .enter()
      .append("g")
      .attr("class", "node")
      .attr("transform", (d: any) => `translate(${d.y + 100},${d.x + 50})`);

    // Add circles for nodes
    nodes.append("circle")
      .attr("r", (d: any) => {
        if (d.depth === 0) return 8; // Root
        if (d.depth === 1) return 6; // Day
        return Math.max(3, Math.min(8, Math.sqrt(d.data.message_count))); // Session
      })
      .attr("fill", (d: any) => {
        if (d.depth === 0) return "#4f46e5";
        if (d.depth === 1) return "#6366f1";
        
        // Color by chat type
        switch (d.data.chat_type) {
          case 'collaborative': return "#10b981";
          case 'direct_message': return "#f59e0b";
          case 'group_chat': return "#ef4444";
          default: return "#8b5cf6";
        }
      })
      .attr("stroke", "#fff")
      .attr("stroke-width", 2)
      .style("cursor", (d: any) => d.depth === 2 ? "pointer" : "default")
      .on("click", (event: any, d: any) => {
        if (d.depth === 2) {
          onSessionClick(d.data.id);
        }
      });

    // Add labels
    nodes.append("text")
      .attr("dx", 12)
      .attr("dy", 4)
      .style("font-size", (d: any) => {
        if (d.depth === 0) return "14px";
        if (d.depth === 1) return "12px";
        return "10px";
      })
      .style("font-weight", (d: any) => d.depth <= 1 ? "bold" : "normal")
      .style("fill", "#374151")
      .text((d: any) => {
        if (d.depth === 2 && d.data.title.length > 20) {
          return d.data.title.substring(0, 20) + "...";
        }
        return d.data.title;
      });

    // Add message count labels for sessions
    nodes.filter((d: any) => d.depth === 2)
      .append("text")
      .attr("dx", 12)
      .attr("dy", 16)
      .style("font-size", "8px")
      .style("fill", "#6b7280")
      .text((d: any) => `${d.data.message_count} msgs`);

    console.log('SessionTimelineView: D3 tree rendered', { 
      nodeCount: treeNodes.descendants().length,
      linkCount: treeNodes.links().length,
      tangledCount: tangledConnections.length,
      dimensions: { width, height }
    });

  }, [treeData, onSessionClick]);

  // Helper function to calculate title similarity
  function calculateTitleSimilarity(title1: string, title2: string): number {
    const words1 = title1.toLowerCase().split(/\s+/);
    const words2 = title2.toLowerCase().split(/\s+/);
    
    const commonWords = words1.filter(word => words2.includes(word));
    const totalWords = new Set([...words1, ...words2]).size;
    
    return commonWords.length / totalWords;
  }

  if (!treeData) {
    return (
      <div className="flex items-center justify-center h-64 text-gray-500">
        <div className="text-center">
          <p className="text-lg font-medium">No sessions to display</p>
          <p className="text-sm">Start a conversation to see your timeline</p>
        </div>
      </div>
    );
  }

  return (
    <div 
      ref={containerRef}
      className="w-full h-full min-h-[600px] bg-white rounded-lg border overflow-auto"
    >
      <div className="p-4 border-b bg-gray-50">
        <h3 className="text-lg font-semibold text-gray-900">Session Timeline</h3>
        <p className="text-sm text-gray-600">
          Tangled tree visualization of your conversation history
        </p>
      </div>
      
      <div className="p-4">
        <svg
          ref={svgRef}
          className="w-full"
          style={{ minHeight: '600px' }}
        />
      </div>
    </div>
  );
};

export default SessionTimelineView;
