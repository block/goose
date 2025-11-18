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
      .slice(0, 50); // Limit for performance

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

    // Create nodes array
    const nodes: TangledNode[] = [];
    const links: TangledLink[] = [];

    // Sort days (newest first)
    const sortedDays = Array.from(sessionsByDay.keys())
      .sort((a, b) => new Date(b).getTime() - new Date(a).getTime());

    let sessionIndex = 0;

    // Create date nodes (level 0) and session nodes (level 1)
    sortedDays.forEach((day, dayIndex) => {
      const dayTitle = day === new Date().toDateString() ? 'Today' : 
                      day === new Date(Date.now() - 24 * 60 * 60 * 1000).toDateString() ? 'Yesterday' :
                      formatDistanceToNow(new Date(day), { addSuffix: true });

      // Create day node
      const dayNode: TangledNode = {
        id: `day-${day}`,
        title: dayTitle,
        level: 0,
        x: 100, // Will be positioned properly later
        y: dayIndex * 120 + 60,
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

      // Create session nodes for this day
      const daySessions = sessionsByDay.get(day)!;
      daySessions.forEach((session, sessionInDayIndex) => {
        const sessionNode: TangledNode = {
          id: session.id,
          title: session.title,
          level: 1,
          x: 300, // Will be positioned properly later
          y: dayIndex * 120 + 60 + (sessionInDayIndex - (daySessions.length - 1) / 2) * 25,
          data: session
        };
        nodes.push(sessionNode);

        // Create tree link from day to session
        links.push({
          source: dayNode,
          target: sessionNode,
          type: 'tree'
        });

        sessionIndex++;
      });
    });

    // Create tangled links between similar sessions
    const sessionNodes = nodes.filter(n => n.level === 1);
    
    for (let i = 0; i < sessionNodes.length; i++) {
      for (let j = i + 1; j < sessionNodes.length; j++) {
        const nodeA = sessionNodes[i];
        const nodeB = sessionNodes[j];
        
        // Skip if same day (already connected via tree)
        if (nodeA.data.day === nodeB.data.day) continue;
        
        // Calculate similarity
        const titleSimilarity = calculateTitleSimilarity(nodeA.data.title, nodeB.data.title);
        const typeSimilarity = nodeA.data.chat_type === nodeB.data.chat_type ? 0.3 : 0;
        const messageSimilarity = Math.min(nodeA.data.message_count, nodeB.data.message_count) / 
                                 Math.max(nodeA.data.message_count, nodeB.data.message_count) * 0.2;
        
        const totalSimilarity = titleSimilarity + typeSimilarity + messageSimilarity;
        
        if (totalSimilarity > 0.4 && links.filter(l => l.type === 'tangle').length < 15) {
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

    // Set up dimensions
    const containerRect = container.getBoundingClientRect();
    const width = Math.max(800, containerRect.width - 40);
    const height = Math.max(600, nodes.filter(n => n.level === 0).length * 120 + 100);

    svg.attr("width", width).attr("height", height);

    // Position nodes properly for tangled tree layout
    const dayNodes = nodes.filter(n => n.level === 0);
    const sessionNodes = nodes.filter(n => n.level === 1);

    // Position day nodes vertically
    dayNodes.forEach((node, index) => {
      node.x = 100;
      node.y = index * 120 + 60;
    });

    // Position session nodes
    sessionNodes.forEach(node => {
      const dayNode = dayNodes.find(d => d.id === `day-${node.data.day}`);
      if (dayNode) {
        const daySessionNodes = sessionNodes.filter(s => s.data.day === node.data.day);
        const sessionIndex = daySessionNodes.indexOf(node);
        
        node.x = 300;
        node.y = dayNode.y + (sessionIndex - (daySessionNodes.length - 1) / 2) * 25;
      }
    });

    // Create container group
    const g = svg.append("g").attr("transform", "translate(20, 20)");

    // Draw tree links (day to sessions)
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
        
        // Horizontal tree connection
        const mx = (sx + tx) / 2;
        return `M${sx},${sy}C${mx},${sy} ${mx},${ty} ${tx},${ty}`;
      })
      .attr("fill", "none")
      .attr("stroke", "#6366f1")
      .attr("stroke-width", 2)
      .attr("opacity", 0.6);

    // Draw tangled links (session to session)
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
        
        // Curved tangled connection
        const dx = tx - sx;
        const dy = ty - sy;
        const dr = Math.sqrt(dx * dx + dy * dy);
        
        // Control points for more organic curves
        const cx1 = sx + dx * 0.3 + dr * 0.1;
        const cy1 = sy + dy * 0.3 - dr * 0.2;
        const cx2 = sx + dx * 0.7 - dr * 0.1;
        const cy2 = sy + dy * 0.7 + dr * 0.2;
        
        return `M${sx},${sy}C${cx1},${cy1} ${cx2},${cy2} ${tx},${ty}`;
      })
      .attr("fill", "none")
      .attr("stroke", "#f59e0b")
      .attr("stroke-width", 1.5)
      .attr("stroke-dasharray", "4,4")
      .attr("opacity", 0.5);

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
        if (d.level === 0) return 8; // Day nodes
        return Math.max(4, Math.min(10, Math.sqrt(d.data.message_count) + 2)); // Session nodes
      })
      .attr("fill", d => {
        if (d.level === 0) return "#4f46e5"; // Day nodes
        
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
          d3.select(this).attr("stroke-width", 3);
        }
      })
      .on("mouseout", function(event, d) {
        if (d.level === 1) {
          d3.select(this).attr("stroke-width", 2);
        }
      });

    // Add labels
    nodeGroups.append("text")
      .attr("dx", d => d.level === 0 ? -8 : 15)
      .attr("dy", 4)
      .attr("text-anchor", d => d.level === 0 ? "end" : "start")
      .style("font-size", d => d.level === 0 ? "12px" : "10px")
      .style("font-weight", d => d.level === 0 ? "bold" : "normal")
      .style("fill", "#374151")
      .text(d => {
        if (d.level === 1 && d.title.length > 25) {
          return d.title.substring(0, 25) + "...";
        }
        return d.title;
      });

    // Add message count for session nodes
    nodeGroups.filter(d => d.level === 1)
      .append("text")
      .attr("dx", 15)
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
          Dates on the left, sessions branching out, with tangled connections showing relationships
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
