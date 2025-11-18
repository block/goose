import React, { useEffect, useRef, useMemo } from 'react';
import * as d3 from 'd3';
import { Session } from '../../api/types.gen';
import { formatDistanceToNow } from 'date-fns';

interface SessionTimelineViewProps {
  sessions: Session[];
  onSessionClick: (sessionId: string) => void;
}

interface TangledNode {
  id: string;
  title: string;
  level: number;
  x: number;
  y: number;
  data: SessionData;
  dayIndex?: number;
  sessionIndex?: number;
}

interface TangledLink {
  source: TangledNode;
  target: TangledNode;
  type: 'tree' | 'tangle';
}

interface SessionData {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
  message_count: number;
  chat_type: string;
  day?: string;
}

const SessionTimelineView: React.FC<SessionTimelineViewProps> = ({ 
  sessions, 
  onSessionClick 
}) => {
  const svgRef = useRef<SVGSVGElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Process sessions into tangled tree data structure
  const { nodes, links } = useMemo(() => {
    console.log('SessionTimelineView: Processing sessions', { sessionCount: sessions.length });
    
    if (sessions.length === 0) {
      return { nodes: [], links: [] };
    }

    // Sort sessions by creation date (newest first)
    const sortedSessions = [...sessions]
      .sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())
      .slice(0, 100); // Limit for performance

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
        chat_type: session.chat_type || 'regular',
        day
      });
    });

    // Sort days (newest first)
    const sortedDays = Array.from(sessionsByDay.keys())
      .sort((a, b) => new Date(b).getTime() - new Date(a).getTime());

    // Create nodes array
    const nodes: TangledNode[] = [];
    const links: TangledLink[] = [];

    // Layout configuration - matching Observable example
    const daySpacing = 200; // Horizontal spacing between days
    const sessionSpacing = 30; // Vertical spacing between sessions
    const dayToSessionOffset = 80; // Vertical offset from day to first session

    // Create day nodes (level 0) positioned horizontally across the top
    sortedDays.forEach((day, dayIndex) => {
      const dayDate = new Date(day);
      const today = new Date();
      const yesterday = new Date(Date.now() - 24 * 60 * 60 * 1000);
      
      let dayTitle;
      if (day === today.toDateString()) {
        dayTitle = 'Today';
      } else if (day === yesterday.toDateString()) {
        dayTitle = 'Yesterday';
      } else {
        dayTitle = dayDate.toLocaleDateString('en-US', { 
          month: 'short', 
          day: 'numeric',
          year: dayDate.getFullYear() !== today.getFullYear() ? 'numeric' : undefined
        });
      }

      // Position day nodes horizontally across the top
      const dayNode: TangledNode = {
        id: `day-${day}`,
        title: dayTitle,
        level: 0,
        x: dayIndex * daySpacing + 100,
        y: 50, // Top position
        dayIndex,
        data: {
          id: `day-${day}`,
          title: dayTitle,
          created_at: day,
          updated_at: day,
          message_count: sessionsByDay.get(day)!.reduce((sum, s) => sum + s.message_count, 0),
          chat_type: 'day',
          day
        }
      };
      nodes.push(dayNode);

      // Create session nodes for this day, positioned vertically below
      const daySessions = sessionsByDay.get(day)!;
      daySessions.forEach((session, sessionInDayIndex) => {
        const sessionNode: TangledNode = {
          id: session.id,
          title: session.title,
          level: 1,
          x: dayNode.x, // Same x as parent day
          y: dayNode.y + dayToSessionOffset + (sessionInDayIndex * sessionSpacing),
          dayIndex,
          sessionIndex: sessionInDayIndex,
          data: session
        };
        nodes.push(sessionNode);

        // Create tree link from day to session (90-degree angle)
        links.push({
          source: dayNode,
          target: sessionNode,
          type: 'tree'
        });
      });
    });

    // Create tangled links between similar sessions across different days
    const sessionNodes = nodes.filter(n => n.level === 1);
    
    for (let i = 0; i < sessionNodes.length; i++) {
      for (let j = i + 1; j < sessionNodes.length; j++) {
        const nodeA = sessionNodes[i];
        const nodeB = sessionNodes[j];
        
        // Skip if same day (already connected via tree)
        if (nodeA.dayIndex === nodeB.dayIndex) continue;
        
        // Calculate similarity
        const titleSimilarity = calculateTitleSimilarity(nodeA.data.title, nodeB.data.title);
        const typeSimilarity = nodeA.data.chat_type === nodeB.data.chat_type ? 0.4 : 0;
        const messageSimilarity = Math.min(nodeA.data.message_count, nodeB.data.message_count) / 
                                 Math.max(nodeA.data.message_count, nodeB.data.message_count) * 0.2;
        
        const totalSimilarity = titleSimilarity + typeSimilarity + messageSimilarity;
        
        if (totalSimilarity > 0.3 && links.filter(l => l.type === 'tangle').length < 20) {
          links.push({
            source: nodeA,
            target: nodeB,
            type: 'tangle'
          });
        }
      }
    }

    console.log('SessionTimelineView: Created tangled tree', { 
      nodeCount: nodes.length,
      dayCount: sortedDays.length,
      sessionCount: sortedSessions.length,
      sessionsPerDay: Object.fromEntries(Array.from(sessionsByDay.entries()).map(([day, sessions]) => [day, sessions.length])),
      treeLinks: links.filter(l => l.type === 'tree').length,
      tangleLinks: links.filter(l => l.type === 'tangle').length
    });

    return { nodes, links };
  }, [sessions]);

  useEffect(() => {
    if (nodes.length === 0 || !svgRef.current || !containerRef.current) return;

    const svg = d3.select(svgRef.current);
    const container = containerRef.current;
    
    // Clear previous content
    svg.selectAll("*").remove();

    // Calculate dimensions based on layout
    const containerRect = container.getBoundingClientRect();
    const dayNodes = nodes.filter(n => n.level === 0);
    const sessionNodes = nodes.filter(n => n.level === 1);
    
    const maxX = Math.max(...nodes.map(n => n.x)) + 150;
    const maxY = Math.max(...nodes.map(n => n.y)) + 50;
    
    const width = Math.max(800, maxX, containerRect.width - 40);
    const height = Math.max(400, maxY);

    svg.attr("width", width).attr("height", height);

    // Create container group
    const g = svg.append("g").attr("transform", "translate(20, 20)");

    // Draw tree links (day to sessions) with 90-degree angles
    const treeLinks = links.filter(l => l.type === 'tree');
    g.selectAll(".tree-link")
      .data(treeLinks)
      .enter()
      .append("path")
      .attr("class", "tree-link")
      .attr("d", d => {
        const sx = d.source.x;
        const sy = d.source.y;
        const tx = d.target.x;
        const ty = d.target.y;
        
        // 90-degree angle connection: straight down from source, then straight to target
        return `M${sx},${sy}L${sx},${ty}L${tx},${ty}`;
      })
      .attr("fill", "none")
      .attr("stroke", "#6366f1")
      .attr("stroke-width", 2)
      .attr("opacity", 0.7);

    // Draw tangled links (session to session) with curved paths
    const tangleLinks = links.filter(l => l.type === 'tangle');
    g.selectAll(".tangle-link")
      .data(tangleLinks)
      .enter()
      .append("path")
      .attr("class", "tangle-link")
      .attr("d", d => {
        const sx = d.source.x;
        const sy = d.source.y;
        const tx = d.target.x;
        const ty = d.target.y;
        
        // Curved connection for tangled links
        const midY = (sy + ty) / 2;
        const controlOffset = Math.abs(tx - sx) * 0.3;
        
        return `M${sx},${sy}C${sx},${midY - controlOffset} ${tx},${midY - controlOffset} ${tx},${ty}`;
      })
      .attr("fill", "none")
      .attr("stroke", "#f59e0b")
      .attr("stroke-width", 1.5)
      .attr("stroke-dasharray", "5,5")
      .attr("opacity", 0.6);

    // Draw nodes
    const nodeGroups = g.selectAll(".node")
      .data(nodes)
      .enter()
      .append("g")
      .attr("class", "node")
      .attr("transform", d => `translate(${d.x},${d.y})`);

    // Add circles for nodes
    nodeGroups.append("circle")
      .attr("r", d => {
        if (d.level === 0) return 10; // Day nodes - larger
        return Math.max(5, Math.min(12, Math.sqrt(d.data.message_count) + 3)); // Session nodes
      })
      .attr("fill", d => {
        if (d.level === 0) return "#4f46e5"; // Day nodes - blue
        
        // Session nodes colored by chat type
        switch (d.data.chat_type) {
          case 'collaborative': return "#10b981";
          case 'direct_message': return "#f59e0b";
          case 'group_chat': return "#ef4444";
          default: return "#8b5cf6";
        }
      })
      .attr("stroke", "#fff")
      .attr("stroke-width", 2)
      .style("cursor", d => d.level === 1 ? "pointer" : "default")
      .on("click", (event, d) => {
        if (d.level === 1) {
          onSessionClick(d.data.id);
        }
      })
      .on("mouseover", function(event, d) {
        if (d.level === 1) {
          d3.select(this)
            .attr("stroke-width", 3)
            .attr("r", Math.max(6, Math.min(14, Math.sqrt(d.data.message_count) + 4)));
        }
      })
      .on("mouseout", function(event, d) {
        if (d.level === 1) {
          d3.select(this)
            .attr("stroke-width", 2)
            .attr("r", Math.max(5, Math.min(12, Math.sqrt(d.data.message_count) + 3)));
        }
      });

    // Add labels for day nodes (above the circle)
    nodeGroups.filter(d => d.level === 0)
      .append("text")
      .attr("dx", 0)
      .attr("dy", -15)
      .attr("text-anchor", "middle")
      .style("font-size", "12px")
      .style("font-weight", "bold")
      .style("fill", "#374151")
      .text(d => d.title);

    // Add labels for session nodes (to the right)
    nodeGroups.filter(d => d.level === 1)
      .append("text")
      .attr("dx", 18)
      .attr("dy", 4)
      .attr("text-anchor", "start")
      .style("font-size", "10px")
      .style("font-weight", "normal")
      .style("fill", "#374151")
      .text(d => {
        if (d.title.length > 30) {
          return d.title.substring(0, 30) + "...";
        }
        return d.title;
      });

    // Add message count for session nodes
    nodeGroups.filter(d => d.level === 1)
      .append("text")
      .attr("dx", 18)
      .attr("dy", 16)
      .style("font-size", "8px")
      .style("fill", "#6b7280")
      .text(d => `${d.data.message_count} msgs`);

    console.log('SessionTimelineView: Tangled tree rendered', { 
      nodeCount: nodes.length,
      treeLinks: treeLinks.length,
      tangleLinks: tangleLinks.length,
      dimensions: { width, height }
    });

  }, [nodes, links, onSessionClick]);

  // Helper function to calculate title similarity
  function calculateTitleSimilarity(title1: string, title2: string): number {
    const words1 = title1.toLowerCase().split(/\s+/).filter(w => w.length > 2);
    const words2 = title2.toLowerCase().split(/\s+/).filter(w => w.length > 2);
    
    if (words1.length === 0 || words2.length === 0) return 0;
    
    const commonWords = words1.filter(word => words2.includes(word));
    const totalWords = new Set([...words1, ...words2]).size;
    
    return commonWords.length / totalWords;
  }

  if (nodes.length === 0) {
    return (
      <div className="flex items-center justify-center h-64 text-gray-500">
        <div className="text-center">
          <p className="text-lg font-medium">No sessions to display</p>
          <p className="text-sm">Start a conversation to see your tangled tree timeline</p>
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
        <h3 className="text-lg font-semibold text-gray-900">Tangled Tree Timeline</h3>
        <p className="text-sm text-gray-600">
          Days across the top, sessions flowing down with 90° connections, tangled links show relationships
        </p>
      </div>
      
      <div className="p-4">
        <svg
          ref={svgRef}
          className="w-full"
          style={{ minHeight: '400px' }}
        />
      </div>
    </div>
  );
};

export default SessionTimelineView;
