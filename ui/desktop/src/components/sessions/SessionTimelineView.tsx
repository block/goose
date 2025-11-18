import React, { useEffect, useRef, useMemo } from 'react';
import * as d3 from 'd3';
import { Session } from '../../api/types.gen';

interface SessionTimelineViewProps {
  sessions: Session[];
  onSessionClick: (sessionId: string) => void;
}

interface ForceNode extends d3.SimulationNodeDatum {
  id: string;
  title: string;
  level: number;
  data: SessionData;
  children?: ForceNode[];
  parent?: ForceNode;
}

interface ForceLink extends d3.SimulationLinkDatum<ForceNode> {
  source: ForceNode;
  target: ForceNode;
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

  // Process sessions into hierarchical data for force simulation
  const { nodes, links } = useMemo(() => {
    console.log('SessionTimelineView: Processing sessions', { sessionCount: sessions.length });
    
    if (sessions.length === 0) {
      return { nodes: [], links: [] };
    }

    // Sort sessions by creation date (newest first)
    const allSessions = [...sessions]
      .sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime());

    // Group sessions by day first to ensure we get multiple days
    const sessionsByDay = new Map<string, SessionData[]>();
    
    allSessions.forEach(session => {
      const day = new Date(session.created_at).toDateString();
      if (!sessionsByDay.has(day)) {
        sessionsByDay.set(day, []);
      }
      
      // Use description as title, or generate a meaningful title
      const title = session.description || 
                   `Chat ${session.id.slice(0, 8)}` || 'Untitled Session';
      
      sessionsByDay.get(day)!.push({
        id: session.id,
        title: title,
        created_at: session.created_at,
        updated_at: session.updated_at,
        message_count: session.message_count || 0,
        chat_type: session.chat_type || 'regular',
        day
      });
    });

    // Limit sessions per day to keep visualization manageable
    // but ensure we show up to 15 days
    const maxDays = 15;
    const maxSessionsPerDay = 8;
    
    const sortedDays = Array.from(sessionsByDay.keys())
      .sort((a, b) => new Date(b).getTime() - new Date(a).getTime())
      .slice(0, maxDays);

    // Limit sessions within each day
    sortedDays.forEach(day => {
      const sessions = sessionsByDay.get(day)!;
      if (sessions.length > maxSessionsPerDay) {
        sessionsByDay.set(day, sessions.slice(0, maxSessionsPerDay));
      }
    });

    // Create root node
    const rootNode: ForceNode = {
      id: 'root',
      title: 'Timeline',
      level: 0,
      data: {
        id: 'root',
        title: 'Timeline',
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        message_count: 0,
        chat_type: 'root'
      },
      children: []
    };

    const nodes: ForceNode[] = [rootNode];
    const links: ForceLink[] = [];

    // Create day nodes and session nodes
    sortedDays.forEach((day) => {
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

      // Create day node
      const dayNode: ForceNode = {
        id: `day-${day}`,
        title: dayTitle,
        level: 1,
        data: {
          id: `day-${day}`,
          title: dayTitle,
          created_at: day,
          updated_at: day,
          message_count: sessionsByDay.get(day)!.reduce((sum, s) => sum + s.message_count, 0),
          chat_type: 'day',
          day
        },
        parent: rootNode,
        children: []
      };
      
      nodes.push(dayNode);
      rootNode.children!.push(dayNode);
      
      // Create link from root to day
      links.push({
        source: rootNode,
        target: dayNode
      });

      // Create session nodes for this day
      const daySessions = sessionsByDay.get(day)!;
      daySessions.forEach((session) => {
        const sessionNode: ForceNode = {
          id: session.id,
          title: session.title,
          level: 2,
          data: session,
          parent: dayNode
        };
        
        nodes.push(sessionNode);
        dayNode.children!.push(sessionNode);
        
        // Create link from day to session
        links.push({
          source: dayNode,
          target: sessionNode
        });
      });
    });

    console.log('SessionTimelineView: Created force graph data', { 
      nodeCount: nodes.length,
      linkCount: links.length,
      dayCount: sortedDays.length
    });

    return { nodes, links };
  }, [sessions]);

  useEffect(() => {
    if (nodes.length === 0 || !svgRef.current || !containerRef.current) return;

    const svg = d3.select(svgRef.current);
    const container = containerRef.current;
    
    // Clear previous content
    svg.selectAll("*").remove();

    // Get theme colors from CSS custom properties
    const computedStyle = getComputedStyle(container);
    const backgroundColor = computedStyle.getPropertyValue('--background') || '#ffffff';
    const foregroundColor = computedStyle.getPropertyValue('--foreground') || '#000000';
    const mutedColor = computedStyle.getPropertyValue('--muted-foreground') || '#6b7280';
    const borderColor = computedStyle.getPropertyValue('--border') || '#e5e7eb';

    // Set up dimensions
    const containerRect = container.getBoundingClientRect();
    const width = Math.max(800, containerRect.width - 40);
    const height = Math.max(600, containerRect.height - 100);

    svg.attr("width", width).attr("height", height)
       .style("background-color", `hsl(${backgroundColor})`);

    // Create force simulation
    const simulation = d3.forceSimulation(nodes)
      .force("link", d3.forceLink(links).id((d: any) => d.id).distance(80).strength(0.8))
      .force("charge", d3.forceManyBody().strength(-300))
      .force("center", d3.forceCenter(width / 2, height / 2))
      .force("collision", d3.forceCollide().radius(30));

    // Create container group
    const g = svg.append("g");

    // Add zoom behavior
    const zoom = d3.zoom<SVGSVGElement, unknown>()
      .scaleExtent([0.1, 4])
      .on("zoom", (event) => {
        g.attr("transform", event.transform);
      });

    svg.call(zoom);

    // Create links
    const link = g.append("g")
      .attr("class", "links")
      .selectAll("line")
      .data(links)
      .enter().append("line")
      .attr("stroke", `hsl(${borderColor})`)
      .attr("stroke-opacity", 0.6)
      .attr("stroke-width", 2);

    // Create nodes
    const node = g.append("g")
      .attr("class", "nodes")
      .selectAll("g")
      .data(nodes)
      .enter().append("g")
      .attr("class", "node")
      .call(d3.drag<SVGGElement, ForceNode>()
        .on("start", dragstarted)
        .on("drag", dragged)
        .on("end", dragended));

    // Add circles to nodes
    node.append("circle")
      .attr("r", d => {
        if (d.level === 0) return 12; // Root
        if (d.level === 1) return 8;  // Day nodes
        return Math.max(4, Math.min(10, Math.sqrt(d.data.message_count || 1) + 2)); // Session nodes
      })
      .attr("fill", d => {
        if (d.level === 0) return "#6366f1"; // Root - indigo
        if (d.level === 1) return "#3b82f6"; // Day nodes - blue
        
        // Session nodes colored by chat type
        switch (d.data.chat_type) {
          case 'collaborative': return "#10b981"; // green
          case 'direct_message': return "#f59e0b"; // amber
          case 'group_chat': return "#ef4444"; // red
          default: return "#8b5cf6"; // purple
        }
      })
      .attr("stroke", `hsl(${backgroundColor})`)
      .attr("stroke-width", 2)
      .style("cursor", d => d.level === 2 ? "pointer" : "default")
      .on("click", (event, d) => {
        if (d.level === 2) {
          onSessionClick(d.data.id);
        }
      });

    // Add labels to nodes
    node.append("text")
      .text(d => {
        if (d.level === 2 && d.title.length > 15) {
          return d.title.substring(0, 15) + "...";
        }
        return d.title;
      })
      .attr("x", 0)
      .attr("y", d => d.level === 0 ? -18 : d.level === 1 ? -12 : 18)
      .attr("text-anchor", "middle")
      .style("font-size", d => d.level === 0 ? "14px" : d.level === 1 ? "12px" : "10px")
      .style("font-weight", d => d.level <= 1 ? "bold" : "normal")
      .style("fill", `hsl(${foregroundColor})`)
      .style("pointer-events", "none");

    // Add message count for session nodes
    node.filter(d => d.level === 2)
      .append("text")
      .text(d => `${d.data.message_count}`)
      .attr("x", 0)
      .attr("y", 28)
      .attr("text-anchor", "middle")
      .style("font-size", "8px")
      .style("fill", `hsl(${mutedColor})`)
      .style("pointer-events", "none");

    // Update positions on simulation tick
    simulation.on("tick", () => {
      link
        .attr("x1", (d: any) => d.source.x)
        .attr("y1", (d: any) => d.source.y)
        .attr("x2", (d: any) => d.target.x)
        .attr("y2", (d: any) => d.target.y);

      node
        .attr("transform", d => `translate(${d.x},${d.y})`);
    });

    // Drag functions
    function dragstarted(event: any, d: ForceNode) {
      if (!event.active) simulation.alphaTarget(0.3).restart();
      d.fx = d.x;
      d.fy = d.y;
    }

    function dragged(event: any, d: ForceNode) {
      d.fx = event.x;
      d.fy = event.y;
    }

    function dragended(event: any, d: ForceNode) {
      if (!event.active) simulation.alphaTarget(0);
      d.fx = null;
      d.fy = null;
    }

    // Cleanup function
    return () => {
      simulation.stop();
    };

  }, [nodes, links, onSessionClick]);

  if (nodes.length === 0) {
    return (
      <div className="flex items-center justify-center h-64 text-muted-foreground">
        <div className="text-center">
          <p className="text-lg font-medium">No sessions to display</p>
          <p className="text-sm">Start a conversation to see your force-directed timeline</p>
        </div>
      </div>
    );
  }

  return (
    <div 
      ref={containerRef}
      className="w-full h-full min-h-[600px] bg-background rounded-lg border overflow-hidden"
    >
      <div className="p-4 border-b bg-muted/50">
        <h3 className="text-lg font-semibold text-foreground">Force-Directed Timeline</h3>
        <p className="text-sm text-muted-foreground">
          Interactive network showing session relationships • Drag nodes to explore • Zoom and pan to navigate
        </p>
      </div>
      
      <div className="relative w-full h-full">
        <svg
          ref={svgRef}
          className="w-full h-full"
          style={{ minHeight: '500px' }}
        />
        
        {/* Legend */}
        <div className="absolute bottom-4 left-4 bg-background/90 backdrop-blur-sm rounded-lg p-3 border shadow-sm">
          <div className="text-xs font-semibold text-foreground mb-2">Legend</div>
          <div className="space-y-1 text-xs text-muted-foreground">
            <div className="flex items-center gap-2">
              <div className="w-3 h-3 bg-indigo-500 rounded-full"></div>
              <span>Timeline Root</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-2.5 h-2.5 bg-blue-500 rounded-full"></div>
              <span>Day</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-2 h-2 bg-purple-500 rounded-full"></div>
              <span>Regular Chat</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-2 h-2 bg-green-500 rounded-full"></div>
              <span>Collaborative</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-2 h-2 bg-amber-500 rounded-full"></div>
              <span>Direct Message</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-2 h-2 bg-red-500 rounded-full"></div>
              <span>Group Chat</span>
            </div>
          </div>
        </div>

        {/* Controls */}
        <div className="absolute top-4 right-4 bg-background/90 backdrop-blur-sm rounded-lg p-2 border shadow-sm">
          <div className="text-xs text-muted-foreground">
            Drag • Zoom • Click sessions
          </div>
        </div>
      </div>
    </div>
  );
};

export default SessionTimelineView;
