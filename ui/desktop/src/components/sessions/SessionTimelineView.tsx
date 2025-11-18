import React, { useEffect, useRef, useMemo } from 'react';
import * as d3 from 'd3';
import { Session } from '../../api/types.gen';
import { formatDistanceToNow } from 'date-fns';

interface SessionTimelineViewProps {
  sessions: Session[];
  onSessionClick: (sessionId: string) => void;
}

interface TreeNode {
  id: string;
  title: string;
  level: number;
  x: number;
  y: number;
  data: SessionData;
  children?: TreeNode[];
  parent?: TreeNode;
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

  // Process sessions into hierarchical tree structure
  const treeData = useMemo(() => {
    console.log('SessionTimelineView: Processing sessions', { sessionCount: sessions.length });
    
    if (sessions.length === 0) {
      return null;
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

    // Sort days (newest first)
    const sortedDays = Array.from(sessionsByDay.keys())
      .sort((a, b) => new Date(b).getTime() - new Date(a).getTime());

    // Create tree nodes
    const nodes: TreeNode[] = [];
    let yOffset = 50;

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
          day: 'numeric'
        });
      }

      // Create day node on the left spine
      const dayNode: TreeNode = {
        id: `day-${day}`,
        title: dayTitle,
        level: 0,
        x: 100,
        y: yOffset,
        data: {
          id: `day-${day}`,
          title: dayTitle,
          created_at: day,
          updated_at: day,
          message_count: sessionsByDay.get(day)!.reduce((sum, s) => sum + s.message_count, 0),
          chat_type: 'day',
          day
        },
        children: []
      };
      nodes.push(dayNode);

      // Create session nodes branching to the right
      const daySessions = sessionsByDay.get(day)!;
      daySessions.forEach((session, sessionIndex) => {
        const sessionNode: TreeNode = {
          id: session.id,
          title: session.title,
          level: 1,
          x: 300 + (sessionIndex % 3) * 200, // Spread sessions horizontally
          y: yOffset + Math.floor(sessionIndex / 3) * 40,
          data: session,
          parent: dayNode
        };
        nodes.push(sessionNode);
        dayNode.children!.push(sessionNode);
      });

      yOffset += Math.max(80, Math.ceil(daySessions.length / 3) * 40 + 40);
    });

    console.log('SessionTimelineView: Created tree structure', { 
      nodeCount: nodes.length,
      dayCount: sortedDays.length
    });

    return nodes;
  }, [sessions]);

  useEffect(() => {
    if (!treeData || !svgRef.current || !containerRef.current) return;

    const svg = d3.select(svgRef.current);
    const container = containerRef.current;
    
    // Clear previous content
    svg.selectAll("*").remove();

    // Calculate dimensions
    const containerRect = container.getBoundingClientRect();
    const maxX = Math.max(...treeData.map(n => n.x)) + 200;
    const maxY = Math.max(...treeData.map(n => n.y)) + 100;
    
    const width = Math.max(800, maxX, containerRect.width - 40);
    const height = Math.max(600, maxY);

    svg.attr("width", width).attr("height", height);

    // Create container group
    const g = svg.append("g").attr("transform", "translate(20, 20)");

    // Define colors for different connection types
    const colors = ['#e74c3c', '#3498db', '#2ecc71', '#f39c12', '#9b59b6', '#1abc9c', '#e67e22'];

    // Draw curved connections (tangled tree style)
    const dayNodes = treeData.filter(n => n.level === 0);
    dayNodes.forEach((dayNode, dayIndex) => {
      if (!dayNode.children) return;

      dayNode.children.forEach((sessionNode, sessionIndex) => {
        // Create curved path from day to session
        const path = g.append("path")
          .attr("d", () => {
            const sx = dayNode.x;
            const sy = dayNode.y;
            const tx = sessionNode.x;
            const ty = sessionNode.y;
            
            // Create smooth curve similar to the image
            const midX = sx + (tx - sx) * 0.5;
            const controlX1 = sx + (tx - sx) * 0.3;
            const controlX2 = sx + (tx - sx) * 0.7;
            
            return `M${sx},${sy}C${controlX1},${sy} ${controlX2},${ty} ${tx},${ty}`;
          })
          .attr("fill", "none")
          .attr("stroke", colors[dayIndex % colors.length])
          .attr("stroke-width", 2)
          .attr("opacity", 0.7);
      });
    });

    // Add tangled connections between similar sessions
    const sessionNodes = treeData.filter(n => n.level === 1);
    const tangledConnections: Array<[TreeNode, TreeNode]> = [];
    
    for (let i = 0; i < sessionNodes.length; i++) {
      for (let j = i + 1; j < sessionNodes.length; j++) {
        const nodeA = sessionNodes[i];
        const nodeB = sessionNodes[j];
        
        // Skip if same day
        if (nodeA.parent?.id === nodeB.parent?.id) continue;
        
        // Calculate similarity
        const titleSimilarity = calculateTitleSimilarity(nodeA.data.title, nodeB.data.title);
        const typeSimilarity = nodeA.data.chat_type === nodeB.data.chat_type ? 0.4 : 0;
        
        if (titleSimilarity + typeSimilarity > 0.3 && tangledConnections.length < 10) {
          tangledConnections.push([nodeA, nodeB]);
        }
      }
    }

    // Draw tangled connections
    tangledConnections.forEach(([nodeA, nodeB], index) => {
      g.append("path")
        .attr("d", () => {
          const sx = nodeA.x;
          const sy = nodeA.y;
          const tx = nodeB.x;
          const ty = nodeB.y;
          
          // Create curved tangled connection
          const midY = (sy + ty) / 2;
          const offset = 50;
          
          return `M${sx},${sy}Q${sx + offset},${midY} ${tx},${ty}`;
        })
        .attr("fill", "none")
        .attr("stroke", colors[(index + 3) % colors.length])
        .attr("stroke-width", 1.5)
        .attr("stroke-dasharray", "4,4")
        .attr("opacity", 0.5);
    });

    // Draw nodes
    const nodeGroups = g.selectAll(".node")
      .data(treeData)
      .enter()
      .append("g")
      .attr("class", "node")
      .attr("transform", d => `translate(${d.x},${d.y})`);

    // Add circles for nodes
    nodeGroups.append("circle")
      .attr("r", d => d.level === 0 ? 6 : 4)
      .attr("fill", d => {
        if (d.level === 0) return "#34495e"; // Day nodes
        
        // Session nodes colored by chat type
        switch (d.data.chat_type) {
          case 'collaborative': return "#2ecc71";
          case 'direct_message': return "#f39c12";
          case 'group_chat': return "#e74c3c";
          default: return "#3498db";
        }
      })
      .attr("stroke", "#fff")
      .attr("stroke-width", 1)
      .style("cursor", d => d.level === 1 ? "pointer" : "default")
      .on("click", (event, d) => {
        if (d.level === 1) {
          onSessionClick(d.data.id);
        }
      });

    // Add labels
    nodeGroups.append("text")
      .attr("dx", d => d.level === 0 ? -10 : 10)
      .attr("dy", 4)
      .attr("text-anchor", d => d.level === 0 ? "end" : "start")
      .style("font-size", "11px")
      .style("font-family", "system-ui, sans-serif")
      .style("fill", "#2c3e50")
      .text(d => {
        if (d.level === 1 && d.title.length > 20) {
          return d.title.substring(0, 20) + "...";
        }
        return d.title;
      });

    console.log('SessionTimelineView: Rendered tangled tree', { 
      nodeCount: treeData.length,
      tangledConnections: tangledConnections.length,
      dimensions: { width, height }
    });

  }, [treeData, onSessionClick]);

  // Helper function to calculate title similarity
  function calculateTitleSimilarity(title1: string, title2: string): number {
    const words1 = title1.toLowerCase().split(/\s+/).filter(w => w.length > 2);
    const words2 = title2.toLowerCase().split(/\s+/).filter(w => w.length > 2);
    
    if (words1.length === 0 || words2.length === 0) return 0;
    
    const commonWords = words1.filter(word => words2.includes(word));
    const totalWords = new Set([...words1, ...words2]).size;
    
    return commonWords.length / totalWords;
  }

  if (!treeData) {
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
          Days on the left spine, sessions branching out with curved connections showing relationships
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
