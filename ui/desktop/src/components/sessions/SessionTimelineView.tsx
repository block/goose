import React, { useMemo, useState } from 'react';
import { Session } from '../../api';
import { formatMessageTimestamp } from '../../utils/timeUtils';
import { MessageSquareText, Target, Calendar, Users, Hash, MessageCircle } from 'lucide-react';
import { ScrollArea } from '../ui/scroll-area';
import { unifiedSessionService } from '../../services/UnifiedSessionService';

interface TreeNode {
  id: string;
  session: Session;
  displayInfo: ReturnType<typeof unifiedSessionService.getSessionDisplayInfo>;
  startTime: Date;
  endTime: Date;
  duration: number;
  x: number;
  y: number;
  level: number; // depth in the tree
  nodeSize: number;
  children: TreeNode[];
  parent?: TreeNode;
  spansMultipleDays: boolean;
  dayGroup: string;
}

interface TreePath {
  id: string;
  d: string; // SVG path data
  stroke: string;
  strokeWidth: number;
  opacity: number;
  isMultiDay: boolean;
}

interface SessionTimelineViewProps {
  sessions: Session[];
  onSelectSession: (sessionId: string) => void;
  selectedSessionId?: string | null;
  className?: string;
}

const SessionTimelineView: React.FC<SessionTimelineViewProps> = ({
  sessions,
  onSelectSession,
  selectedSessionId,
  className = '',
}) => {
  const [hoveredSession, setHoveredSession] = useState<string | null>(null);

  const treeData = useMemo(() => {
    console.log('SessionTimelineView: Processing sessions', { 
      sessionCount: sessions.length,
      sessions: sessions.slice(0, 3).map(s => ({ id: s.id, created_at: s.created_at, message_count: s.message_count }))
    });

    if (sessions.length === 0) {
      console.log('SessionTimelineView: No sessions found');
      return { nodes: [], paths: [], width: 800, height: 600 };
    }

    // Limit to first 50 sessions for performance and debugging
    const limitedSessions = sessions.slice(0, 50);
    console.log('SessionTimelineView: Limited to', limitedSessions.length, 'sessions for performance');

    // Sort sessions by start time (newest first - today at top)
    const sortedSessions = [...limitedSessions].sort((a, b) => 
      new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
    );

    console.log('SessionTimelineView: Sorted sessions', sortedSessions.length);

    // Group sessions by day for initial positioning
    const dayGroups = new Map<string, Session[]>();
    sortedSessions.forEach(session => {
      const dayKey = new Date(session.created_at).toDateString();
      if (!dayGroups.has(dayKey)) {
        dayGroups.set(dayKey, []);
      }
      dayGroups.get(dayKey)!.push(session);
    });

    const sortedDays = Array.from(dayGroups.keys()).sort((a, b) => 
      new Date(b).getTime() - new Date(a).getTime()
    );

    // Create tree nodes
    const nodes: TreeNode[] = [];
    const nodeMap = new Map<string, TreeNode>();
    
    let currentY = 100;
    const daySpacing = 200;
    const sessionSpacing = 80;
    const levelSpacing = 150;

    sortedDays.forEach((dayKey, dayIndex) => {
      const daySessions = dayGroups.get(dayKey)!;
      
      daySessions.forEach((session, sessionIndex) => {
        const displayInfo = unifiedSessionService.getSessionDisplayInfo(session);
        const startTime = new Date(session.created_at);
        const endTime = new Date(session.updated_at);
        const duration = endTime.getTime() - startTime.getTime();
        const spansMultipleDays = startTime.toDateString() !== endTime.toDateString();
        
        // Node size based on message count
        const nodeSize = Math.max(16, Math.min(32, 16 + (session.message_count * 0.8)));
        
        // Calculate position
        const level = Math.floor(sessionIndex / 3); // Group sessions into levels
        const positionInLevel = sessionIndex % 3;
        
        const x = 100 + (level * levelSpacing) + (positionInLevel * 60);
        const y = currentY + (sessionIndex * sessionSpacing);

        const node: TreeNode = {
          id: session.id,
          session,
          displayInfo,
          startTime,
          endTime,
          duration,
          x,
          y,
          level,
          nodeSize,
          children: [],
          spansMultipleDays,
          dayGroup: dayKey,
        };

        nodes.push(node);
        nodeMap.set(session.id, node);
      });
      
      currentY += daySpacing + (daySessions.length * sessionSpacing);
    });

    // Create hierarchical relationships based on temporal proximity and type
    nodes.forEach((node, index) => {
      // Find potential parent nodes (earlier sessions of similar type or in same day)
      const potentialParents = nodes.slice(0, index).filter(parent => {
        const timeDiff = node.startTime.getTime() - parent.endTime.getTime();
        const sameDay = node.dayGroup === parent.dayGroup;
        const sameType = node.displayInfo.type === parent.displayInfo.type;
        const closeInTime = timeDiff < 24 * 60 * 60 * 1000; // Within 24 hours
        
        return (sameDay || (sameType && closeInTime)) && parent.children.length < 3;
      });

      if (potentialParents.length > 0) {
        // Choose the most recent parent
        const parent = potentialParents[potentialParents.length - 1];
        parent.children.push(node);
        node.parent = parent;
        
        // Adjust child position to create tree structure
        const childIndex = parent.children.length - 1;
        node.x = parent.x + 80 + (childIndex * 40);
        node.y = parent.y + 60 + (childIndex * 30);
      }
    });

    // Generate curved paths between connected nodes
    const paths: TreePath[] = [];
    
    nodes.forEach(node => {
      if (node.parent) {
        const parent = node.parent;
        
        // Create curved path using SVG cubic bezier
        const dx = node.x - parent.x;
        const dy = node.y - parent.y;
        
        // Control points for smooth curves
        const cp1x = parent.x + dx * 0.5;
        const cp1y = parent.y;
        const cp2x = parent.x + dx * 0.5;
        const cp2y = node.y;
        
        const pathData = `M ${parent.x + parent.nodeSize/2} ${parent.y + parent.nodeSize/2} 
                         C ${cp1x} ${cp1y}, ${cp2x} ${cp2y}, ${node.x + node.nodeSize/2} ${node.y + node.nodeSize/2}`;

        // Path styling based on session types
        let stroke = '#94a3b8'; // default gray
        let strokeWidth = 2;
        let opacity = 0.6;
        
        if (node.displayInfo.type === 'collaborative') {
          stroke = '#a855f7'; // purple
          strokeWidth = 3;
        } else if (node.session.extension_data?.matrix?.isDirectMessage) {
          stroke = '#22c55e'; // green
        } else if (node.displayInfo.type === 'matrix') {
          stroke = '#3b82f6'; // blue
        }
        
        if (node.spansMultipleDays) {
          opacity = 0.8;
          strokeWidth += 1;
        }

        paths.push({
          id: `${parent.id}-${node.id}`,
          d: pathData,
          stroke,
          strokeWidth,
          opacity,
          isMultiDay: node.spansMultipleDays,
        });
      }
    });

    // Add some additional tangled connections for sessions of the same type
    nodes.forEach((node, index) => {
      const similarNodes = nodes.filter((other, otherIndex) => {
        return otherIndex !== index && 
               other.displayInfo.type === node.displayInfo.type &&
               !other.parent && 
               !node.parent &&
               Math.abs(other.startTime.getTime() - node.startTime.getTime()) < 12 * 60 * 60 * 1000; // Within 12 hours
      });

      // Create tangled connections between similar sessions
      similarNodes.slice(0, 2).forEach(similarNode => {
        const dx = similarNode.x - node.x;
        const dy = similarNode.y - node.y;
        const distance = Math.sqrt(dx * dx + dy * dy);
        
        if (distance > 50 && distance < 300) { // Only connect if at reasonable distance
          // Create a more complex curved path for tangled effect
          const cp1x = node.x + dx * 0.3 + (Math.random() - 0.5) * 60;
          const cp1y = node.y + dy * 0.7;
          const cp2x = similarNode.x - dx * 0.3 + (Math.random() - 0.5) * 60;
          const cp2y = similarNode.y - dy * 0.3;
          
          const tangledPath = `M ${node.x + node.nodeSize/2} ${node.y + node.nodeSize/2} 
                              C ${cp1x} ${cp1y}, ${cp2x} ${cp2y}, ${similarNode.x + similarNode.nodeSize/2} ${similarNode.y + similarNode.nodeSize/2}`;

          let stroke = '#e2e8f0';
          if (node.displayInfo.type === 'collaborative') stroke = '#ddd6fe';
          else if (node.displayInfo.type === 'matrix') stroke = '#dbeafe';
          
          paths.push({
            id: `tangled-${node.id}-${similarNode.id}`,
            d: tangledPath,
            stroke,
            strokeWidth: 1,
            opacity: 0.3,
            isMultiDay: false,
          });
        }
      });
    });

    // Calculate canvas dimensions
    const maxX = nodes.length > 0 ? Math.max(...nodes.map(n => n.x + n.nodeSize + 100)) : 800;
    const maxY = nodes.length > 0 ? Math.max(...nodes.map(n => n.y + n.nodeSize + 50)) : 600;

    const result = {
      nodes,
      paths,
      width: Math.max(800, maxX),
      height: Math.max(600, maxY),
    };

    console.log('SessionTimelineView: Final tree data', {
      nodeCount: result.nodes.length,
      pathCount: result.paths.length,
      dimensions: { width: result.width, height: result.height },
      firstNode: result.nodes[0] ? { 
        id: result.nodes[0].id, 
        x: result.nodes[0].x, 
        y: result.nodes[0].y, 
        nodeSize: result.nodes[0].nodeSize 
      } : null,
      sampleNodes: result.nodes.slice(0, 5).map(n => ({ 
        id: n.id, 
        x: n.x, 
        y: n.y, 
        nodeSize: n.nodeSize,
        messageCount: n.session.message_count 
      }))
    });

    return result;
  }, [sessions]);

  const getNodeIcon = (node: TreeNode) => {
    const { displayInfo, session } = node;
    
    if (displayInfo.type === 'collaborative') {
      return <Users className="w-3 h-3 text-white" />;
    } else if (session.extension_data?.matrix?.isDirectMessage) {
      return <MessageCircle className="w-3 h-3 text-white" />;
    } else if (displayInfo.type === 'matrix') {
      return <Hash className="w-3 h-3 text-white" />;
    }
    
    return <MessageSquareText className="w-3 h-3 text-white" />;
  };

  const getNodeColor = (node: TreeNode) => {
    const { displayInfo, session } = node;
    const isSelected = selectedSessionId === node.id;
    const isHovered = hoveredSession === node.id;
    
    let baseColor = '';
    if (displayInfo.type === 'collaborative') {
      baseColor = 'bg-purple-500 border-purple-600';
    } else if (session.extension_data?.matrix?.isDirectMessage) {
      baseColor = 'bg-green-500 border-green-600';
    } else if (displayInfo.type === 'matrix') {
      baseColor = 'bg-blue-500 border-blue-600';
    } else {
      baseColor = 'bg-gray-500 border-gray-600';
    }
    
    if (isSelected) {
      return `${baseColor} ring-4 ring-yellow-300 ring-opacity-60 scale-110`;
    }
    
    if (isHovered) {
      return `${baseColor} scale-125 shadow-xl`;
    }
    
    return `${baseColor} hover:scale-110`;
  };

  const formatDuration = (duration: number) => {
    const minutes = Math.floor(duration / (1000 * 60));
    const hours = Math.floor(minutes / 60);
    
    if (hours > 0) {
      return `${hours}h ${minutes % 60}m`;
    }
    if (minutes > 0) {
      return `${minutes}m`;
    }
    return '< 1m';
  };

  const handleNodeClick = (nodeId: string) => {
    onSelectSession(nodeId);
  };

  if (treeData.nodes.length === 0) {
    return (
      <div className={`flex flex-col items-center justify-center h-full text-text-muted ${className}`}>
        <Calendar className="h-12 w-12 mb-4" />
        <p className="text-lg mb-2">No sessions to display</p>
        <p className="text-sm">Your session tree will appear here</p>
      </div>
    );
  }

  return (
    <div className={`${className}`}>
      {/* Header */}
      <div className="mb-6 px-6">
        <h3 className="text-lg font-medium text-text-standard mb-2">Session Tangled Tree</h3>
        <p className="text-sm text-text-muted">
          Tree visualization of {sessions.length} sessions showing temporal and topical relationships with curved connections.
        </p>
      </div>

      <ScrollArea className="h-full">
        <div className="p-6">
          <div className="relative bg-gradient-to-br from-slate-50 to-blue-50 dark:from-gray-900 dark:to-blue-950 rounded-xl border border-border-subtle overflow-hidden">
            {/* SVG for curved paths */}
            <svg
              width={treeData.width}
              height={treeData.height}
              className="absolute inset-0"
              style={{ zIndex: 1 }}
            >
              <defs>
                {/* Gradient definitions for paths */}
                <linearGradient id="purpleGrad" x1="0%" y1="0%" x2="100%" y2="100%">
                  <stop offset="0%" stopColor="#a855f7" stopOpacity="0.8" />
                  <stop offset="100%" stopColor="#8b5cf6" stopOpacity="0.4" />
                </linearGradient>
                <linearGradient id="blueGrad" x1="0%" y1="0%" x2="100%" y2="100%">
                  <stop offset="0%" stopColor="#3b82f6" stopOpacity="0.8" />
                  <stop offset="100%" stopColor="#60a5fa" stopOpacity="0.4" />
                </linearGradient>
                <linearGradient id="greenGrad" x1="0%" y1="0%" x2="100%" y2="100%">
                  <stop offset="0%" stopColor="#22c55e" stopOpacity="0.8" />
                  <stop offset="100%" stopColor="#4ade80" stopOpacity="0.4" />
                </linearGradient>
                
                {/* Filter for glowing effect */}
                <filter id="glow">
                  <feGaussianBlur stdDeviation="3" result="coloredBlur"/>
                  <feMerge> 
                    <feMergeNode in="coloredBlur"/>
                    <feMergeNode in="SourceGraphic"/>
                  </feMerge>
                </filter>
              </defs>
              
              {/* Render all paths */}
              {treeData.paths.map((path) => (
                <path
                  key={path.id}
                  d={path.d}
                  fill="none"
                  stroke={path.stroke}
                  strokeWidth={path.strokeWidth}
                  opacity={path.opacity}
                  strokeDasharray={path.isMultiDay ? "5,5" : "none"}
                  filter={path.isMultiDay ? "url(#glow)" : "none"}
                  className="transition-all duration-300"
                />
              ))}
            </svg>

            {/* Render nodes */}
            <div className="relative" style={{ zIndex: 2 }}>
              {treeData.nodes.map((node) => {
                const isSelected = selectedSessionId === node.id;
                const isHovered = hoveredSession === node.id;
                
                return (
                  <div key={node.id}>
                    {/* Node */}
                    <div
                      className={`absolute cursor-pointer transition-all duration-300 rounded-full border-2 flex items-center justify-center shadow-lg ${getNodeColor(node)}`}
                      style={{
                        left: `${node.x}px`,
                        top: `${node.y}px`,
                        width: `${node.nodeSize}px`,
                        height: `${node.nodeSize}px`,
                      }}
                      onClick={() => handleNodeClick(node.id)}
                      onMouseEnter={() => setHoveredSession(node.id)}
                      onMouseLeave={() => setHoveredSession(null)}
                    >
                      {getNodeIcon(node)}
                      
                      {/* Message count badge */}
                      {node.session.message_count > 0 && (
                        <div className="absolute -top-1 -right-1 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-full min-w-4 h-4 flex items-center justify-center text-xs font-bold text-text-standard px-1">
                          {node.session.message_count}
                        </div>
                      )}
                      
                      {/* Multi-day indicator */}
                      {node.spansMultipleDays && (
                        <div className="absolute -bottom-1 -left-1 w-3 h-3 bg-purple-400 border border-white dark:border-gray-800 rounded-full"></div>
                      )}
                    </div>
                    
                    {/* Hover tooltip */}
                    {isHovered && (
                      <div
                        className="absolute z-50 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-xl p-4 min-w-72 max-w-80"
                        style={{
                          left: `${node.x + node.nodeSize + 15}px`,
                          top: `${node.y}px`,
                        }}
                      >
                        <div className="text-sm font-semibold text-text-standard mb-2 break-words">
                          {node.session.description || node.session.id}
                        </div>
                        
                        {/* Session type badge */}
                        <div className="mb-3">
                          <span className={`inline-flex items-center px-2 py-1 rounded-full text-xs font-medium ${
                            node.displayInfo.type === 'collaborative' 
                              ? 'bg-purple-100 text-purple-800 dark:bg-purple-900/30 dark:text-purple-300'
                              : node.displayInfo.type === 'matrix'
                              ? 'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300'
                              : 'bg-gray-100 text-gray-800 dark:bg-gray-900/30 dark:text-gray-300'
                          }`}>
                            {node.displayInfo.type === 'collaborative' ? 'Collaborative' : 
                             node.displayInfo.type === 'matrix' ? 'Matrix' : 'Regular'}
                          </span>
                        </div>
                        
                        <div className="space-y-2 text-xs text-text-muted">
                          <div>Started: {formatMessageTimestamp(node.startTime.getTime() / 1000)}</div>
                          {node.duration > 60000 && (
                            <div>Duration: {formatDuration(node.duration)}</div>
                          )}
                          
                          <div className="flex items-center gap-4 mt-3">
                            <div className="flex items-center gap-1">
                              <MessageSquareText className="w-3 h-3" />
                              <span>{node.session.message_count} messages</span>
                            </div>
                            
                            {node.session.total_tokens && (
                              <div className="flex items-center gap-1">
                                <Target className="w-3 h-3" />
                                <span>{node.session.total_tokens.toLocaleString()}</span>
                              </div>
                            )}
                          </div>
                          
                          {node.children.length > 0 && (
                            <div className="mt-2 pt-2 border-t border-gray-200 dark:border-gray-600">
                              <span className="font-medium">Connected to {node.children.length} session{node.children.length > 1 ? 's' : ''}</span>
                            </div>
                          )}
                        </div>
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      </ScrollArea>
      
      {/* Legend */}
      <div className="mt-6 mx-6 p-4 bg-background-subtle rounded-lg border">
        <h4 className="text-sm font-medium text-text-standard mb-3">Tangled Tree Legend</h4>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4 text-xs">
          {/* Session types */}
          <div className="space-y-2">
            <h5 className="font-medium text-text-standard">Session Types</h5>
            <div className="flex items-center gap-2">
              <div className="w-4 h-4 rounded-full bg-gray-500 flex items-center justify-center">
                <MessageSquareText className="w-2 h-2 text-white" />
              </div>
              <span>Regular Session</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-4 h-4 rounded-full bg-green-500 flex items-center justify-center">
                <MessageCircle className="w-2 h-2 text-white" />
              </div>
              <span>Direct Message</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-4 h-4 rounded-full bg-blue-500 flex items-center justify-center">
                <Hash className="w-2 h-2 text-white" />
              </div>
              <span>Matrix Room</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-4 h-4 rounded-full bg-purple-500 flex items-center justify-center">
                <Users className="w-2 h-2 text-white" />
              </div>
              <span>Collaborative</span>
            </div>
          </div>
          
          {/* Connections */}
          <div className="space-y-2">
            <h5 className="font-medium text-text-standard">Connections</h5>
            <div className="flex items-center gap-2">
              <svg width="20" height="8">
                <path d="M 0 4 C 10 4, 10 4, 20 4" stroke="#94a3b8" strokeWidth="2" fill="none"/>
              </svg>
              <span>Temporal relationship</span>
            </div>
            <div className="flex items-center gap-2">
              <svg width="20" height="8">
                <path d="M 0 4 C 10 4, 10 4, 20 4" stroke="#a855f7" strokeWidth="3" fill="none"/>
              </svg>
              <span>Collaborative connection</span>
            </div>
            <div className="flex items-center gap-2">
              <svg width="20" height="8">
                <path d="M 0 4 C 10 4, 10 4, 20 4" stroke="#94a3b8" strokeWidth="2" fill="none" strokeDasharray="5,5"/>
              </svg>
              <span>Multi-day session</span>
            </div>
            <div className="flex items-center gap-2">
              <svg width="20" height="8">
                <path d="M 0 4 C 10 4, 10 4, 20 4" stroke="#e2e8f0" strokeWidth="1" fill="none" opacity="0.3"/>
              </svg>
              <span>Topical similarity</span>
            </div>
          </div>
          
          {/* Visual elements */}
          <div className="space-y-2">
            <h5 className="font-medium text-text-standard">Visual Elements</h5>
            <div className="flex items-center gap-2">
              <span>Node size:</span>
              <span className="text-text-muted">Message count</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-3 h-3 bg-purple-400 rounded-full"></div>
              <span>Multi-day indicator</span>
            </div>
            <div className="flex items-center gap-2">
              <span>Curved paths:</span>
              <span className="text-text-muted">Relationships</span>
            </div>
            <div className="flex items-center gap-2">
              <span>Tree structure:</span>
              <span className="text-text-muted">Temporal hierarchy</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default SessionTimelineView;
