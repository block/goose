import React, { useMemo, useState } from 'react';
import { Session } from '../../api';
import { formatMessageTimestamp } from '../../utils/timeUtils';
import { MessageSquareText, Target, Calendar, Users, Hash, MessageCircle } from 'lucide-react';
import { ScrollArea } from '../ui/scroll-area';
import { unifiedSessionService } from '../../services/UnifiedSessionService';

interface TimelineNode {
  id: string;
  session: Session;
  displayInfo: ReturnType<typeof unifiedSessionService.getSessionDisplayInfo>;
  startTime: Date;
  endTime: Date;
  duration: number;
  yPosition: number;
  nodeSize: number;
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
  const [hoveredNode, setHoveredNode] = useState<string | null>(null);

  const timelineNodes = useMemo(() => {
    if (sessions.length === 0) return [];

    // Create nodes from sessions
    const nodes: TimelineNode[] = sessions.map((session) => {
      const displayInfo = unifiedSessionService.getSessionDisplayInfo(session);
      const startTime = new Date(session.created_at);
      const endTime = new Date(session.updated_at);
      const duration = endTime.getTime() - startTime.getTime();
      
      // Node size based on message count (min 24px, max 60px)
      const nodeSize = Math.max(24, Math.min(60, 24 + (session.message_count * 1.5)));
      
      return {
        id: session.id,
        session,
        displayInfo,
        startTime,
        endTime,
        duration,
        yPosition: 0, // Will be calculated below
        nodeSize,
      };
    });

    // Sort by start time (oldest first for top-to-bottom timeline)
    nodes.sort((a, b) => a.startTime.getTime() - b.startTime.getTime());

    // Calculate Y positions with spacing
    const nodeSpacing = 100; // pixels between nodes
    nodes.forEach((node, index) => {
      node.yPosition = 80 + (index * nodeSpacing);
    });

    return nodes;
  }, [sessions]);

  const getNodeIcon = (node: TimelineNode) => {
    const { displayInfo, session } = node;
    
    if (displayInfo.type === 'collaborative') {
      return <Users className="w-5 h-5 text-white" />;
    } else if (session.extension_data?.matrix?.isDirectMessage) {
      return <MessageCircle className="w-5 h-5 text-white" />;
    } else if (displayInfo.type === 'matrix') {
      return <Hash className="w-5 h-5 text-white" />;
    }
    
    return <MessageSquareText className="w-5 h-5 text-white" />;
  };

  const getNodeColor = (node: TimelineNode) => {
    const { displayInfo, session } = node;
    const isSelected = selectedSessionId === node.id;
    const isHovered = hoveredNode === node.id;
    
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
      return `${baseColor} ring-4 ring-yellow-300 ring-opacity-60`;
    }
    
    if (isHovered) {
      return `${baseColor} scale-110 shadow-lg`;
    }
    
    return baseColor;
  };

  const getDurationBarColor = (duration: number) => {
    if (duration < 5 * 60 * 1000) return 'bg-gray-300'; // < 5 minutes
    if (duration < 30 * 60 * 1000) return 'bg-blue-400'; // < 30 minutes  
    if (duration < 2 * 60 * 60 * 1000) return 'bg-yellow-400'; // < 2 hours
    return 'bg-red-400'; // > 2 hours
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

  if (timelineNodes.length === 0) {
    return (
      <div className={`flex flex-col items-center justify-center h-full text-text-muted ${className}`}>
        <Calendar className="h-12 w-12 mb-4" />
        <p className="text-lg mb-2">No sessions to display</p>
        <p className="text-sm">Your session timeline will appear here</p>
      </div>
    );
  }

  const timelineHeight = Math.max(600, timelineNodes[timelineNodes.length - 1]?.yPosition + 100);

  return (
    <div className={`${className}`}>
      {/* Header */}
      <div className="mb-6 px-6">
        <h3 className="text-lg font-medium text-text-standard mb-2">Session Timeline</h3>
        <p className="text-sm text-text-muted">
          Chronological view of {timelineNodes.length} sessions. Node size reflects message count, colors indicate session type.
        </p>
      </div>

      <ScrollArea className="h-full">
        <div className="relative px-6" style={{ height: `${timelineHeight}px` }}>
          {/* Main vertical timeline line */}
          <div 
            className="absolute w-1 bg-gradient-to-b from-blue-200 via-purple-200 to-green-200 rounded-full shadow-sm"
            style={{ 
              left: '60px', 
              top: '40px', 
              height: `${timelineHeight - 80}px` 
            }}
          />
          
          {/* Timeline nodes and connections */}
          {timelineNodes.map((node, index) => {
            const isSelected = selectedSessionId === node.id;
            const isHovered = hoveredNode === node.id;
            const nextNode = timelineNodes[index + 1];
            
            return (
              <div key={node.id}>
                {/* Connection line to next node */}
                {nextNode && (
                  <div
                    className="absolute w-0.5 bg-border-subtle"
                    style={{
                      left: '60px',
                      top: `${node.yPosition + node.nodeSize / 2}px`,
                      height: `${nextNode.yPosition - node.yPosition - node.nodeSize / 2}px`,
                    }}
                  />
                )}
                
                {/* Horizontal connector line */}
                <div
                  className="absolute h-0.5 bg-border-default"
                  style={{
                    left: '60px',
                    top: `${node.yPosition + node.nodeSize / 2}px`,
                    width: '40px',
                  }}
                />
                
                {/* Timeline node */}
                <div
                  className={`absolute cursor-pointer transition-all duration-300 rounded-full border-2 flex items-center justify-center shadow-md hover:shadow-lg ${getNodeColor(node)}`}
                  style={{
                    left: `${60 - node.nodeSize / 2}px`,
                    top: `${node.yPosition}px`,
                    width: `${node.nodeSize}px`,
                    height: `${node.nodeSize}px`,
                  }}
                  onClick={() => handleNodeClick(node.id)}
                  onMouseEnter={() => setHoveredNode(node.id)}
                  onMouseLeave={() => setHoveredNode(null)}
                >
                  {getNodeIcon(node)}
                  
                  {/* Message count badge */}
                  {node.session.message_count > 0 && (
                    <div className="absolute -top-2 -right-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-full min-w-5 h-5 flex items-center justify-center text-xs font-medium text-text-standard px-1">
                      {node.session.message_count}
                    </div>
                  )}
                </div>
                
                {/* Session details panel */}
                <div
                  className={`absolute transition-all duration-200 ${
                    isSelected ? 'bg-blue-50 dark:bg-blue-950 border-blue-200 dark:border-blue-800' : 'bg-background-default border-border-subtle'
                  } border rounded-lg p-4 shadow-sm hover:shadow-md cursor-pointer`}
                  style={{
                    left: '120px',
                    top: `${node.yPosition}px`,
                    minWidth: '320px',
                    maxWidth: '500px',
                  }}
                  onClick={() => handleNodeClick(node.id)}
                >
                  {/* Session header */}
                  <div className="flex items-start justify-between mb-3">
                    <div className="flex-1 min-w-0">
                      <h4 className="text-sm font-medium text-text-standard mb-1 break-words">
                        {node.session.description || node.session.id}
                      </h4>
                      
                      {/* Session type badge */}
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
                  </div>
                  
                  {/* Session metadata */}
                  <div className="space-y-2 text-xs text-text-muted">
                    <div className="flex items-center justify-between">
                      <span>Started: {formatMessageTimestamp(node.startTime.getTime() / 1000)}</span>
                      {node.duration > 60000 && (
                        <span className="text-text-standard font-medium">
                          Duration: {formatDuration(node.duration)}
                        </span>
                      )}
                    </div>
                    
                    <div className="flex items-center gap-4">
                      <div className="flex items-center gap-1">
                        <MessageSquareText className="w-3 h-3" />
                        <span>{node.session.message_count} messages</span>
                      </div>
                      
                      {node.session.total_tokens && (
                        <div className="flex items-center gap-1">
                          <Target className="w-3 h-3" />
                          <span>{node.session.total_tokens.toLocaleString()} tokens</span>
                        </div>
                      )}
                      
                      {node.displayInfo.participants && (
                        <div className="flex items-center gap-1">
                          <Users className="w-3 h-3" />
                          <span>{node.displayInfo.participants.length} participants</span>
                        </div>
                      )}
                    </div>
                    
                    {/* Duration bar */}
                    {node.duration > 60000 && (
                      <div className="mt-2">
                        <div className="flex items-center gap-2">
                          <div className="flex-1 h-2 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
                            <div 
                              className={`h-full ${getDurationBarColor(node.duration)} transition-all duration-300`}
                              style={{ width: '100%' }}
                            />
                          </div>
                        </div>
                      </div>
                    )}
                  </div>
                </div>
              </div>
            );
          })}
          
          {/* Timeline start/end markers */}
          <div className="absolute left-2 top-8 text-xs text-text-muted font-medium">
            Earliest
          </div>
          <div className="absolute left-2 text-xs text-text-muted font-medium" style={{ top: `${timelineHeight - 40}px` }}>
            Latest
          </div>
        </div>
      </ScrollArea>
      
      {/* Legend */}
      <div className="mt-6 mx-6 p-4 bg-background-subtle rounded-lg border">
        <h4 className="text-sm font-medium text-text-standard mb-3">Legend</h4>
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
          
          {/* Duration colors */}
          <div className="space-y-2">
            <h5 className="font-medium text-text-standard">Duration</h5>
            <div className="flex items-center gap-2">
              <div className="w-4 h-2 bg-gray-300 rounded"></div>
              <span>&lt; 5 minutes</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-4 h-2 bg-blue-400 rounded"></div>
              <span>&lt; 30 minutes</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-4 h-2 bg-yellow-400 rounded"></div>
              <span>&lt; 2 hours</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-4 h-2 bg-red-400 rounded"></div>
              <span>2+ hours</span>
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
              <span>Position:</span>
              <span className="text-text-muted">Chronological order</span>
            </div>
            <div className="flex items-center gap-2">
              <span>Badge:</span>
              <span className="text-text-muted">Message count</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default SessionTimelineView;
