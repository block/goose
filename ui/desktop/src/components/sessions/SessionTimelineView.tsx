import React, { useMemo, useState } from 'react';
import { Session } from '../../api';
import { formatMessageTimestamp } from '../../utils/timeUtils';
import { MessageSquareText, Target, Calendar, Users, Hash, MessageCircle } from 'lucide-react';
import { ScrollArea } from '../ui/scroll-area';
import { unifiedSessionService } from '../../services/UnifiedSessionService';

interface TimelineDay {
  date: Date;
  dateString: string;
  dayLabel: string;
  yPosition: number;
  sessions: SessionBranch[];
}

interface SessionBranch {
  id: string;
  session: Session;
  displayInfo: ReturnType<typeof unifiedSessionService.getSessionDisplayInfo>;
  startTime: Date;
  endTime: Date;
  duration: number;
  startDay: string;
  endDay: string;
  branchPosition: number; // horizontal offset from main timeline
  nodeSize: number;
  spansMultipleDays: boolean;
  continuationLines: ContinuationLine[];
}

interface ContinuationLine {
  fromDay: string;
  toDay: string;
  fromY: number;
  toY: number;
  branchPosition: number;
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

  const timelineData = useMemo(() => {
    if (sessions.length === 0) return { days: [], branches: [], continuationLines: [] };

    // Group sessions by start day
    const sessionsByDay = new Map<string, Session[]>();
    const allDays = new Set<string>();

    sessions.forEach(session => {
      const startDay = new Date(session.created_at).toDateString();
      const endDay = new Date(session.updated_at).toDateString();
      
      allDays.add(startDay);
      allDays.add(endDay);
      
      if (!sessionsByDay.has(startDay)) {
        sessionsByDay.set(startDay, []);
      }
      sessionsByDay.get(startDay)!.push(session);
    });

    // Create timeline days
    const sortedDays = Array.from(allDays).sort((a, b) => new Date(a).getTime() - new Date(b).getTime());
    const dayHeight = 200;
    const daySpacing = 50;

    const timelineDays: TimelineDay[] = sortedDays.map((dateString, index) => {
      const date = new Date(dateString);
      const today = new Date();
      const yesterday = new Date(today);
      yesterday.setDate(yesterday.getDate() - 1);

      let dayLabel = '';
      if (date.toDateString() === today.toDateString()) {
        dayLabel = 'Today';
      } else if (date.toDateString() === yesterday.toDateString()) {
        dayLabel = 'Yesterday';
      } else {
        dayLabel = date.toLocaleDateString('en-US', {
          weekday: 'short',
          month: 'short',
          day: 'numeric',
        });
      }

      return {
        date,
        dateString,
        dayLabel,
        yPosition: 80 + (index * (dayHeight + daySpacing)),
        sessions: [], // Will be populated below
      };
    });

    // Create session branches
    const allBranches: SessionBranch[] = [];
    const continuationLines: ContinuationLine[] = [];
    const branchPositions = new Map<string, number>(); // Track horizontal positions per day

    sessions.forEach(session => {
      const displayInfo = unifiedSessionService.getSessionDisplayInfo(session);
      const startTime = new Date(session.created_at);
      const endTime = new Date(session.updated_at);
      const startDay = startTime.toDateString();
      const endDay = endTime.toDateString();
      const duration = endTime.getTime() - startTime.getTime();
      const spansMultipleDays = startDay !== endDay;

      // Calculate node size based on message count
      const nodeSize = Math.max(20, Math.min(40, 20 + (session.message_count * 1)));

      // Assign horizontal branch position
      const currentBranchCount = branchPositions.get(startDay) || 0;
      const branchPosition = 120 + (currentBranchCount * 80); // 80px spacing between branches
      branchPositions.set(startDay, currentBranchCount + 1);

      const branch: SessionBranch = {
        id: session.id,
        session,
        displayInfo,
        startTime,
        endTime,
        duration,
        startDay,
        endDay,
        branchPosition,
        nodeSize,
        spansMultipleDays,
        continuationLines: [],
      };

      allBranches.push(branch);

      // Create continuation lines for multi-day sessions
      if (spansMultipleDays) {
        const startDayIndex = sortedDays.indexOf(startDay);
        const endDayIndex = sortedDays.indexOf(endDay);
        
        for (let i = startDayIndex; i < endDayIndex; i++) {
          const fromDay = sortedDays[i];
          const toDay = sortedDays[i + 1];
          const fromDayData = timelineDays.find(d => d.dateString === fromDay);
          const toDayData = timelineDays.find(d => d.dateString === toDay);
          
          if (fromDayData && toDayData) {
            continuationLines.push({
              fromDay,
              toDay,
              fromY: fromDayData.yPosition + dayHeight - 20,
              toY: toDayData.yPosition + 20,
              branchPosition,
            });
          }
        }
      }
    });

    // Assign branches to their starting days
    timelineDays.forEach(day => {
      day.sessions = allBranches.filter(branch => branch.startDay === day.dateString);
    });

    return { days: timelineDays, branches: allBranches, continuationLines };
  }, [sessions]);

  const getSessionIcon = (branch: SessionBranch) => {
    const { displayInfo, session } = branch;
    
    if (displayInfo.type === 'collaborative') {
      return <Users className="w-4 h-4 text-white" />;
    } else if (session.extension_data?.matrix?.isDirectMessage) {
      return <MessageCircle className="w-4 h-4 text-white" />;
    } else if (displayInfo.type === 'matrix') {
      return <Hash className="w-4 h-4 text-white" />;
    }
    
    return <MessageSquareText className="w-4 h-4 text-white" />;
  };

  const getSessionColor = (branch: SessionBranch) => {
    const { displayInfo, session } = branch;
    const isSelected = selectedSessionId === branch.id;
    const isHovered = hoveredSession === branch.id;
    
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

  const getBranchLineColor = (branch: SessionBranch) => {
    const { displayInfo, session } = branch;
    
    if (displayInfo.type === 'collaborative') {
      return 'border-purple-400';
    } else if (session.extension_data?.matrix?.isDirectMessage) {
      return 'border-green-400';
    } else if (displayInfo.type === 'matrix') {
      return 'border-blue-400';
    }
    
    return 'border-gray-400';
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

  const handleSessionClick = (sessionId: string) => {
    onSelectSession(sessionId);
  };

  if (timelineData.days.length === 0) {
    return (
      <div className={`flex flex-col items-center justify-center h-full text-text-muted ${className}`}>
        <Calendar className="h-12 w-12 mb-4" />
        <p className="text-lg mb-2">No sessions to display</p>
        <p className="text-sm">Your session timeline will appear here</p>
      </div>
    );
  }

  const totalHeight = timelineData.days.length > 0 
    ? timelineData.days[timelineData.days.length - 1].yPosition + 250 
    : 600;

  return (
    <div className={`${className}`}>
      {/* Header */}
      <div className="mb-6 px-6">
        <h3 className="text-lg font-medium text-text-standard mb-2">Session Timeline</h3>
        <p className="text-sm text-text-muted">
          Daily timeline showing {sessions.length} sessions. Each day has its own section with conversation branches.
        </p>
      </div>

      <ScrollArea className="h-full">
        <div className="relative px-6" style={{ height: `${totalHeight}px` }}>
          {/* Main vertical timeline spine */}
          <div 
            className="absolute w-2 bg-gradient-to-b from-blue-300 via-purple-300 to-green-300 rounded-full shadow-sm"
            style={{ 
              left: '60px', 
              top: '40px', 
              height: `${totalHeight - 80}px` 
            }}
          />
          
          {/* Day sections and session branches */}
          {timelineData.days.map((day, dayIndex) => (
            <div key={day.dateString}>
              {/* Day marker on main timeline */}
              <div
                className="absolute w-6 h-6 bg-white dark:bg-gray-800 border-2 border-blue-500 rounded-full flex items-center justify-center shadow-md z-10"
                style={{
                  left: '54px',
                  top: `${day.yPosition}px`,
                }}
              >
                <Calendar className="w-3 h-3 text-blue-500" />
              </div>
              
              {/* Day label */}
              <div
                className="absolute text-sm font-medium text-text-standard bg-background-default px-2 py-1 rounded border shadow-sm"
                style={{
                  left: '10px',
                  top: `${day.yPosition - 10}px`,
                }}
              >
                {day.dayLabel}
              </div>
              
              {/* Date label */}
              <div
                className="absolute text-xs text-text-muted"
                style={{
                  left: '10px',
                  top: `${day.yPosition + 15}px`,
                }}
              >
                {day.date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' })}
              </div>
              
              {/* Session branches for this day */}
              {day.sessions.map((branch, branchIndex) => {
                const isSelected = selectedSessionId === branch.id;
                const isHovered = hoveredSession === branch.id;
                const branchY = day.yPosition + 40 + (branchIndex * 60);
                
                return (
                  <div key={branch.id}>
                    {/* Branch line from main timeline to session */}
                    <div
                      className={`absolute h-0.5 border-t-2 border-dashed ${getBranchLineColor(branch)}`}
                      style={{
                        left: '72px',
                        top: `${branchY + branch.nodeSize / 2}px`,
                        width: `${branch.branchPosition - 72}px`,
                      }}
                    />
                    
                    {/* Session node */}
                    <div
                      className={`absolute cursor-pointer transition-all duration-300 rounded-full border-2 flex items-center justify-center shadow-md hover:shadow-lg ${getSessionColor(branch)}`}
                      style={{
                        left: `${branch.branchPosition}px`,
                        top: `${branchY}px`,
                        width: `${branch.nodeSize}px`,
                        height: `${branch.nodeSize}px`,
                      }}
                      onClick={() => handleSessionClick(branch.id)}
                      onMouseEnter={() => setHoveredSession(branch.id)}
                      onMouseLeave={() => setHoveredSession(null)}
                    >
                      {getSessionIcon(branch)}
                      
                      {/* Message count badge */}
                      {branch.session.message_count > 0 && (
                        <div className="absolute -top-2 -right-2 bg-white dark:bg-gray-800 border border-gray-300 dark:border-gray-600 rounded-full min-w-4 h-4 flex items-center justify-center text-xs font-medium text-text-standard px-1">
                          {branch.session.message_count}
                        </div>
                      )}
                    </div>
                    
                    {/* Multi-day indicator */}
                    {branch.spansMultipleDays && (
                      <div
                        className="absolute text-xs text-purple-600 dark:text-purple-400 font-medium"
                        style={{
                          left: `${branch.branchPosition + branch.nodeSize + 8}px`,
                          top: `${branchY - 8}px`,
                        }}
                      >
                        Multi-day
                      </div>
                    )}
                    
                    {/* Session details tooltip on hover */}
                    {isHovered && (
                      <div
                        className="absolute z-50 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-700 rounded-lg shadow-lg p-3 min-w-64 max-w-80"
                        style={{
                          left: `${branch.branchPosition + branch.nodeSize + 10}px`,
                          top: `${branchY}px`,
                        }}
                      >
                        <div className="text-sm font-medium text-text-standard mb-2 break-words">
                          {branch.session.description || branch.session.id}
                        </div>
                        
                        <div className="space-y-1 text-xs text-text-muted">
                          <div>Started: {formatMessageTimestamp(branch.startTime.getTime() / 1000)}</div>
                          {branch.duration > 60000 && (
                            <div>Duration: {formatDuration(branch.duration)}</div>
                          )}
                          
                          <div className="flex items-center gap-3 mt-2">
                            <div className="flex items-center gap-1">
                              <MessageSquareText className="w-3 h-3" />
                              <span>{branch.session.message_count}</span>
                            </div>
                            
                            {branch.session.total_tokens && (
                              <div className="flex items-center gap-1">
                                <Target className="w-3 h-3" />
                                <span>{branch.session.total_tokens.toLocaleString()}</span>
                              </div>
                            )}
                          </div>
                        </div>
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          ))}
          
          {/* Continuation lines for multi-day sessions */}
          {timelineData.continuationLines.map((line, index) => (
            <div
              key={`continuation-${index}`}
              className="absolute border-l-2 border-purple-400 border-dashed opacity-60"
              style={{
                left: `${line.branchPosition + 10}px`,
                top: `${line.fromY}px`,
                height: `${line.toY - line.fromY}px`,
              }}
            />
          ))}
        </div>
      </ScrollArea>
      
      {/* Legend */}
      <div className="mt-6 mx-6 p-4 bg-background-subtle rounded-lg border">
        <h4 className="text-sm font-medium text-text-standard mb-3">Timeline Legend</h4>
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
          
          {/* Visual elements */}
          <div className="space-y-2">
            <h5 className="font-medium text-text-standard">Visual Elements</h5>
            <div className="flex items-center gap-2">
              <div className="w-4 h-0.5 border-t-2 border-dashed border-gray-400"></div>
              <span>Session branch</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-4 h-4 border-l-2 border-purple-400 border-dashed opacity-60"></div>
              <span>Multi-day continuation</span>
            </div>
            <div className="flex items-center gap-2">
              <Calendar className="w-4 h-4 text-blue-500" />
              <span>Day marker</span>
            </div>
          </div>
          
          {/* Interaction */}
          <div className="space-y-2">
            <h5 className="font-medium text-text-standard">Interaction</h5>
            <div className="flex items-center gap-2">
              <span>Node size:</span>
              <span className="text-text-muted">Message count</span>
            </div>
            <div className="flex items-center gap-2">
              <span>Hover:</span>
              <span className="text-text-muted">Session details</span>
            </div>
            <div className="flex items-center gap-2">
              <span>Click:</span>
              <span className="text-text-muted">Open session</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default SessionTimelineView;
