import React, { useEffect, useRef, useMemo } from 'react';
import * as d3 from 'd3';
import { hexbin } from 'd3-hexbin';
import { Session } from '../../api/types.gen';

interface SessionTimelineViewProps {
  sessions: Session[];
  onSessionClick: (sessionId: string) => void;
}

interface HexbinPoint {
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
}

const SessionTimelineView: React.FC<SessionTimelineViewProps> = ({ 
  sessions, 
  onSessionClick 
}) => {
  const svgRef = useRef<SVGSVGElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // Process sessions into hexbin data points
  const hexbinData = useMemo(() => {
    console.log('SessionTimelineView: Processing sessions for hexbin', { sessionCount: sessions.length });
    
    if (sessions.length === 0) {
      return [];
    }

    // Sort sessions by creation date (newest first)
    const sortedSessions = [...sessions]
      .sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime())
      .slice(0, 200); // Limit for performance

    // Convert sessions to points for hexbin
    const points: HexbinPoint[] = sortedSessions.map((session, index) => {
      const sessionDate = new Date(session.created_at);
      const now = new Date();
      
      // X axis: Days ago (0 = today, 1 = yesterday, etc.)
      const daysAgo = Math.floor((now.getTime() - sessionDate.getTime()) / (1000 * 60 * 60 * 24));
      
      // Y axis: Hour of day (0-23)
      const hourOfDay = sessionDate.getHours();
      
      // Add some jitter to avoid exact overlaps
      const jitterX = (Math.random() - 0.5) * 0.8;
      const jitterY = (Math.random() - 0.5) * 0.8;

      const title = session.description || 
                   `Chat ${session.id.slice(0, 8)}` || 'Untitled Session';

      return {
        x: daysAgo + jitterX,
        y: hourOfDay + jitterY,
        session: {
          id: session.id,
          title: title,
          created_at: session.created_at,
          updated_at: session.updated_at,
          message_count: session.message_count || 0,
          chat_type: session.chat_type || 'regular'
        }
      };
    });

    console.log('SessionTimelineView: Created hexbin data', { 
      pointCount: points.length,
      dateRange: {
        oldest: Math.max(...points.map(p => p.x)),
        newest: Math.min(...points.map(p => p.x))
      },
      hourRange: {
        min: Math.min(...points.map(p => p.y)),
        max: Math.max(...points.map(p => p.y))
      }
    });

    return points;
  }, [sessions]);

  useEffect(() => {
    if (hexbinData.length === 0 || !svgRef.current || !containerRef.current) return;

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
    const margin = { top: 60, right: 60, bottom: 80, left: 80 };
    const width = Math.max(800, containerRect.width - 40) - margin.left - margin.right;
    const height = Math.max(500, containerRect.height - 200) - margin.top - margin.bottom;

    svg.attr("width", width + margin.left + margin.right)
       .attr("height", height + margin.top + margin.bottom)
       .style("background-color", `hsl(${backgroundColor})`);

    const g = svg.append("g")
      .attr("transform", `translate(${margin.left},${margin.top})`);

    // Create scales
    const maxDaysAgo = Math.max(...hexbinData.map(d => d.x));
    const xScale = d3.scaleLinear()
      .domain([0, Math.max(30, maxDaysAgo)]) // Show at least 30 days
      .range([0, width]);

    const yScale = d3.scaleLinear()
      .domain([0, 24]) // 24 hours
      .range([height, 0]);

    // Create hexbin generator
    const hexbinGenerator = hexbin()
      .x((d: any) => xScale(d.x))
      .y((d: any) => yScale(d.y))
      .radius(12)
      .extent([[0, 0], [width, height]]);

    // Generate hexbins
    const bins = hexbinGenerator(hexbinData as any);

    // Create color scale based on bin density
    const colorScale = d3.scaleSequential(d3.interpolateBlues)
      .domain([0, d3.max(bins, d => d.length) || 1]);

    // Draw hexbins
    const hexagons = g.selectAll(".hexagon")
      .data(bins)
      .enter().append("g")
      .attr("class", "hexagon")
      .attr("transform", d => `translate(${d.x},${d.y})`);

    hexagons.append("path")
      .attr("d", hexbinGenerator.hexagon())
      .attr("fill", d => d.length > 0 ? colorScale(d.length) : "none")
      .attr("stroke", `hsl(${borderColor})`)
      .attr("stroke-width", 0.5)
      .attr("opacity", 0.8)
      .style("cursor", "pointer")
      .on("mouseover", function(event, d) {
        if (d.length > 0) {
          d3.select(this)
            .attr("stroke-width", 2)
            .attr("opacity", 1);

          // Show tooltip
          const tooltip = g.append("g")
            .attr("class", "tooltip")
            .attr("transform", `translate(${d.x + 15},${d.y - 15})`);

          const rect = tooltip.append("rect")
            .attr("fill", `hsl(${backgroundColor})`)
            .attr("stroke", `hsl(${borderColor})`)
            .attr("rx", 4)
            .attr("opacity", 0.95);

          const text = tooltip.append("text")
            .attr("fill", `hsl(${foregroundColor})`)
            .style("font-size", "12px")
            .attr("x", 8)
            .attr("y", 16);

          text.append("tspan")
            .text(`${d.length} session${d.length !== 1 ? 's' : ''}`);

          if (d.length <= 3) {
            d.forEach((point: any, i: number) => {
              text.append("tspan")
                .attr("x", 8)
                .attr("dy", 14)
                .style("font-size", "10px")
                .text(`• ${point.session.title.substring(0, 25)}${point.session.title.length > 25 ? '...' : ''}`);
            });
          }

          // Size the rectangle to fit the text
          const bbox = text.node()?.getBBox();
          if (bbox) {
            rect.attr("width", bbox.width + 16)
                .attr("height", bbox.height + 12);
          }
        }
      })
      .on("mouseout", function(event, d) {
        if (d.length > 0) {
          d3.select(this)
            .attr("stroke-width", 0.5)
            .attr("opacity", 0.8);
        }
        g.selectAll(".tooltip").remove();
      })
      .on("click", (event, d) => {
        if (d.length === 1) {
          onSessionClick((d[0] as any).session.id);
        } else if (d.length > 1) {
          // For multiple sessions, open the most recent one
          const mostRecent = d.reduce((latest: any, current: any) => 
            new Date(current.session.created_at) > new Date(latest.session.created_at) ? current : latest
          );
          onSessionClick(mostRecent.session.id);
        }
      });

    // Add text labels for non-empty hexbins
    hexagons.filter(d => d.length > 0)
      .append("text")
      .attr("text-anchor", "middle")
      .attr("dy", "0.35em")
      .style("font-size", "10px")
      .style("font-weight", "bold")
      .style("fill", d => d.length > 3 ? "white" : `hsl(${foregroundColor})`)
      .style("pointer-events", "none")
      .text(d => d.length);

    // Create axes
    const xAxis = d3.axisBottom(xScale)
      .tickFormat(d => {
        const days = +d;
        if (days === 0) return 'Today';
        if (days === 1) return 'Yesterday';
        return `${days}d ago`;
      });

    const yAxis = d3.axisLeft(yScale)
      .tickFormat(d => {
        const hour = +d;
        if (hour === 0) return '12 AM';
        if (hour === 12) return '12 PM';
        if (hour < 12) return `${hour} AM`;
        return `${hour - 12} PM`;
      });

    // Add axes
    g.append("g")
      .attr("class", "x-axis")
      .attr("transform", `translate(0,${height})`)
      .call(xAxis)
      .selectAll("text")
      .style("fill", `hsl(${foregroundColor})`)
      .style("font-size", "11px");

    g.append("g")
      .attr("class", "y-axis")
      .call(yAxis)
      .selectAll("text")
      .style("fill", `hsl(${foregroundColor})`)
      .style("font-size", "11px");

    // Style axis lines
    g.selectAll(".domain, .tick line")
      .attr("stroke", `hsl(${borderColor})`);

    // Add axis labels
    g.append("text")
      .attr("transform", `translate(${width / 2}, ${height + 50})`)
      .style("text-anchor", "middle")
      .style("font-size", "14px")
      .style("font-weight", "bold")
      .style("fill", `hsl(${foregroundColor})`)
      .text("Days Ago");

    g.append("text")
      .attr("transform", "rotate(-90)")
      .attr("y", -50)
      .attr("x", -height / 2)
      .style("text-anchor", "middle")
      .style("font-size", "14px")
      .style("font-weight", "bold")
      .style("fill", `hsl(${foregroundColor})`)
      .text("Time of Day");

    // Add title
    g.append("text")
      .attr("x", width / 2)
      .attr("y", -30)
      .attr("text-anchor", "middle")
      .style("font-size", "16px")
      .style("font-weight", "bold")
      .style("fill", `hsl(${foregroundColor})`)
      .text("Session Activity Heatmap");

    console.log('SessionTimelineView: Hexbin rendered', { 
      binCount: bins.length,
      nonEmptyBins: bins.filter(d => d.length > 0).length,
      maxDensity: d3.max(bins, d => d.length),
      dimensions: { width, height }
    });

  }, [hexbinData, onSessionClick]);

  if (hexbinData.length === 0) {
    return (
      <div className="flex items-center justify-center h-64 text-muted-foreground">
        <div className="text-center">
          <p className="text-lg font-medium">No sessions to display</p>
          <p className="text-sm">Start a conversation to see your activity heatmap</p>
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
        <h3 className="text-lg font-semibold text-foreground">Hexbin Timeline</h3>
        <p className="text-sm text-muted-foreground">
          Hexagonal heatmap showing session density by time and day • Hover for details • Click to open sessions
        </p>
      </div>
      
      <div className="relative w-full h-full overflow-auto">
        <svg
          ref={svgRef}
          className="w-full"
          style={{ minHeight: '600px' }}
        />
        
        {/* Legend */}
        <div className="absolute bottom-4 right-4 bg-background/90 backdrop-blur-sm rounded-lg p-3 border shadow-sm">
          <div className="text-xs font-semibold text-foreground mb-2">Density</div>
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <div className="w-4 h-4 bg-blue-100 border border-border rounded-sm"></div>
            <span>Low</span>
            <div className="w-4 h-4 bg-blue-500 border border-border rounded-sm"></div>
            <span>High</span>
          </div>
          <div className="text-xs text-muted-foreground mt-2">
            Numbers show session count per hexagon
          </div>
        </div>
      </div>
    </div>
  );
};

export default SessionTimelineView;
