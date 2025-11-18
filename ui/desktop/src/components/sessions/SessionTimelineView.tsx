import React, { useMemo, useRef, useEffect, useState } from 'react';
import { Session } from '../../api';
import { formatMessageTimestamp } from '../../utils/timeUtils';
import { MessageSquareText, Target, Calendar, Clock, Users, Hash, MessageCircle } from 'lucide-react';
import { Card } from '../ui/card';
import { ScrollArea } from '../ui/scroll-area';
import { unifiedSessionService } from '../../services/UnifiedSessionService';
import { useMatrix } from '../../contexts/MatrixContext';

interface SessionTimelineViewProps {
  sessions: Session[];
  onSelectSession: (sessionId: string) => void;
  selectedSessionId?: string | null;
  className?: string;
}

interface TimelineData {
  date: Date;
  sessions: Session[];
  dayLabel: string;
}

interface SessionEvent {
  session: Session;
  type: 'start' | 'end';
  timestamp: Date;
  displayInfo: ReturnType<typeof unifiedSessionService.getSessionDisplayInfo>;
}

const SessionTimelineView: React.FC<SessionTimelineViewProps> = ({
  sessions,
  onSelectSession,
  selectedSessionId,
  className = '',
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const { rooms, currentUser } = useMatrix();
  const [hoveredSession, setHoveredSession] = useState<string | null>(null);

  // Group sessions by date and create timeline events
  const timelineData = useMemo(() => {
    // Create events for session start and end times
    const events: SessionEvent[] = [];
    
    sessions.forEach(session => {
      const displayInfo = unifiedSessionService.getSessionDisplayInfo(session);
      
      // Session start event
      events.push({
        session,
        type: 'start',
        timestamp: new Date(session.created_at),
        displayInfo,
      });
      
      // Session end event (using updated_at as approximation for when session ended)
      // Only add end event if it's different from start (session had activity)
      const startTime = new Date(session.created_at).getTime();
      const endTime = new Date(session.updated_at).getTime();
      
      if (endTime > startTime + 60000) { // Only if session lasted more than 1 minute
        events.push({
          session,
          type: 'end',
          timestamp: new Date(session.updated_at),
          displayInfo,
        });
      }
    });
    
    // Sort events by timestamp
    events.sort((a, b) => a.timestamp.getTime() - b.timestamp.getTime());
    
    // Group events by date
    const groupedByDate = new Map<string, SessionEvent[]>();
    
    events.forEach(event => {
      const dateKey = event.timestamp.toDateString();
      if (!groupedByDate.has(dateKey)) {
        groupedByDate.set(dateKey, []);
      }
      groupedByDate.get(dateKey)!.push(event);
    });
    
    // Convert to timeline data format
    const timeline: TimelineData[] = [];
    const today = new Date();
    const yesterday = new Date(today);
    yesterday.setDate(yesterday.getDate() - 1);
    
    Array.from(groupedByDate.entries())
      .sort(([a], [b]) => new Date(b).getTime() - new Date(a).getTime()) // Most recent first
      .forEach(([dateString, events]) => {
        const date = new Date(dateString);
        let dayLabel = '';
        
        if (date.toDateString() === today.toDateString()) {
          dayLabel = 'Today';
        } else if (date.toDateString() === yesterday.toDateString()) {
          dayLabel = 'Yesterday';
        } else {
          dayLabel = date.toLocaleDateString('en-US', {
            weekday: 'long',
            month: 'long',
            day: 'numeric',
          });
        }
        
        // Get unique sessions for this date
        const uniqueSessions = Array.from(
          new Map(events.map(event => [event.session.id, event.session])).values()
        );
        
        timeline.push({
          date,
          sessions: uniqueSessions,
          dayLabel,
        });
      });
    
    return { timeline, events };
  }, [sessions]);

  // Get participant details for Matrix sessions
  const getParticipantDetails = (session: Session) => {
    if (!session.extension_data?.matrix?.roomId) return [];
    
    const roomId = session.extension_data.matrix.roomId;
    const room = rooms.find(r => r.roomId === roomId);
    
    if (!room || !room.members) return [];
    
    return room.members
      .filter(member => member.userId !== currentUser?.userId)
      .map(member => ({
        userId: member.userId,
        displayName: member.displayName || member.userId.split(':')[0].substring(1),
        avatarUrl: member.avatarUrl,
      }));
  };

  // Calculate timeline position for events
  const getTimelinePosition = (timestamp: Date, dayStart: Date, dayEnd: Date) => {
    const totalDayMs = dayEnd.getTime() - dayStart.getTime();
    const eventMs = timestamp.getTime() - dayStart.getTime();
    return Math.max(0, Math.min(100, (eventMs / totalDayMs) * 100));
  };

  const SessionEventMarker: React.FC<{
    event: SessionEvent;
    position: number;
    isSelected: boolean;
  }> = ({ event, position, isSelected }) => {
    const { session, type, timestamp, displayInfo } = event;
    const isMatrix = displayInfo.type === 'matrix' || displayInfo.type === 'collaborative';
    const isMatrixDM = isMatrix && session.extension_data?.matrix?.isDirectMessage;
    const isCollaborative = displayInfo.type === 'collaborative';
    
    const participants = isMatrix ? getParticipantDetails(session) : [];
    
    // Color scheme based on session type and event type
    const getEventColor = () => {
      if (isSelected) return 'bg-blue-500 border-blue-600';
      
      if (type === 'start') {
        if (isMatrixDM) return 'bg-green-400 border-green-500';
        if (isCollaborative) return 'bg-purple-400 border-purple-500';
        if (isMatrix) return 'bg-blue-400 border-blue-500';
        return 'bg-gray-400 border-gray-500';
      } else {
        if (isMatrixDM) return 'bg-green-200 border-green-300';
        if (isCollaborative) return 'bg-purple-200 border-purple-300';
        if (isMatrix) return 'bg-blue-200 border-blue-300';
        return 'bg-gray-200 border-gray-300';
      }
    };
    
    const getEventIcon = () => {
      if (isMatrixDM) return <MessageCircle className="w-3 h-3" />;
      if (isCollaborative) return <Users className="w-3 h-3" />;
      if (isMatrix) return <Hash className="w-3 h-3" />;
      return <MessageSquareText className="w-3 h-3" />;
    };
    
    return (
      <div
        className="absolute transform -translate-x-1/2 cursor-pointer group"
        style={{ left: `${position}%`, top: type === 'start' ? '0px' : '30px' }}
        onClick={() => onSelectSession(session.id)}
        onMouseEnter={() => setHoveredSession(session.id)}
        onMouseLeave={() => setHoveredSession(null)}
      >
        {/* Event marker */}
        <div
          className={`w-4 h-4 rounded-full border-2 flex items-center justify-center text-white transition-all duration-200 ${getEventColor()} ${
            hoveredSession === session.id ? 'scale-125 shadow-lg' : ''
          }`}
        >
          {getEventIcon()}
        </div>
        
        {/* Tooltip */}
        {hoveredSession === session.id && (
          <div className="absolute bottom-full mb-2 left-1/2 transform -translate-x-1/2 z-50">
            <Card className="p-3 min-w-64 max-w-80 shadow-lg border">
              <div className="text-sm font-medium text-text-standard mb-1">
                {session.description || session.id}
              </div>
              <div className="text-xs text-text-muted space-y-1">
                <div className="flex items-center gap-1">
                  <Clock className="w-3 h-3" />
                  <span>
                    {type === 'start' ? 'Started' : 'Last activity'}: {formatMessageTimestamp(timestamp.getTime() / 1000)}
                  </span>
                </div>
                <div className="flex items-center gap-1">
                  <MessageSquareText className="w-3 h-3" />
                  <span>{session.message_count} messages</span>
                </div>
                {displayInfo.hasTokenCounts && session.total_tokens && (
                  <div className="flex items-center gap-1">
                    <Target className="w-3 h-3" />
                    <span>{session.total_tokens.toLocaleString()} tokens</span>
                  </div>
                )}
                {participants.length > 0 && (
                  <div className="flex items-center gap-1">
                    <Users className="w-3 h-3" />
                    <span>{participants.length} participant{participants.length > 1 ? 's' : ''}</span>
                  </div>
                )}
              </div>
            </Card>
          </div>
        )}
      </div>
    );
  };

  const TimelineDay: React.FC<{ data: TimelineData; events: SessionEvent[] }> = ({ data, events }) => {
    const dayEvents = events.filter(event => 
      event.timestamp.toDateString() === data.date.toDateString()
    );
    
    // Calculate day boundaries for positioning
    const dayStart = new Date(data.date);
    dayStart.setHours(0, 0, 0, 0);
    const dayEnd = new Date(data.date);
    dayEnd.setHours(23, 59, 59, 999);
    
    // Get actual time range of events for better positioning
    const eventTimes = dayEvents.map(e => e.timestamp.getTime());
    const minTime = Math.min(...eventTimes);
    const maxTime = Math.max(...eventTimes);
    
    // Use actual event range if it's more than 2 hours, otherwise use full day
    const useActualRange = (maxTime - minTime) > 2 * 60 * 60 * 1000;
    const rangeStart = useActualRange ? new Date(minTime) : dayStart;
    const rangeEnd = useActualRange ? new Date(maxTime) : dayEnd;
    
    return (
      <div className="space-y-6">
        {/* Date header */}
        <div className="flex items-center gap-3">
          <Calendar className="w-5 h-5 text-text-muted" />
          <h3 className="text-lg font-medium text-text-standard">{data.dayLabel}</h3>
          <div className="text-sm text-text-muted">
            {data.sessions.length} session{data.sessions.length !== 1 ? 's' : ''}
          </div>
        </div>
        
        {/* Timeline visualization */}
        <div className="relative">
          {/* Timeline background */}
          <div className="h-16 bg-gray-100 dark:bg-gray-800 rounded-lg relative overflow-hidden">
            {/* Time labels */}
            <div className="absolute inset-x-0 bottom-0 flex justify-between text-xs text-text-muted px-2 pb-1">
              <span>{rangeStart.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' })}</span>
              <span>{rangeEnd.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit' })}</span>
            </div>
            
            {/* Event markers */}
            {dayEvents.map((event, index) => (
              <SessionEventMarker
                key={`${event.session.id}-${event.type}-${index}`}
                event={event}
                position={getTimelinePosition(event.timestamp, rangeStart, rangeEnd)}
                isSelected={selectedSessionId === event.session.id}
              />
            ))}
          </div>
        </div>
        
        {/* Session summary cards */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
          {data.sessions.map(session => {
            const displayInfo = unifiedSessionService.getSessionDisplayInfo(session);
            const isMatrix = displayInfo.type === 'matrix' || displayInfo.type === 'collaborative';
            const isSelected = selectedSessionId === session.id;
            
            return (
              <Card
                key={session.id}
                className={`p-3 cursor-pointer transition-all duration-200 hover:shadow-md ${
                  isSelected ? 'ring-2 ring-blue-500 bg-blue-50 dark:bg-blue-950' : ''
                }`}
                onClick={() => onSelectSession(session.id)}
              >
                <div className="flex items-start justify-between mb-2">
                  <h4 className="text-sm font-medium text-text-standard flex-1 overflow-hidden" style={{ 
                    display: '-webkit-box',
                    WebkitLineClamp: 2,
                    WebkitBoxOrient: 'vertical'
                  }}>
                    {session.description || session.id}
                  </h4>
                  {isMatrix && (
                    <div className="ml-2 flex-shrink-0">
                      {displayInfo.type === 'collaborative' ? (
                        <Users className="w-4 h-4 text-purple-500" />
                      ) : session.extension_data?.matrix?.isDirectMessage ? (
                        <MessageCircle className="w-4 h-4 text-green-500" />
                      ) : (
                        <Hash className="w-4 h-4 text-blue-500" />
                      )}
                    </div>
                  )}
                </div>
                
                <div className="flex items-center justify-between text-xs text-text-muted">
                  <div className="flex items-center gap-3">
                    <div className="flex items-center gap-1">
                      <MessageSquareText className="w-3 h-3" />
                      <span>{session.message_count}</span>
                    </div>
                    {displayInfo.hasTokenCounts && session.total_tokens && (
                      <div className="flex items-center gap-1">
                        <Target className="w-3 h-3" />
                        <span>{session.total_tokens.toLocaleString()}</span>
                      </div>
                    )}
                  </div>
                  <div className="text-right">
                    <div>{formatMessageTimestamp(Date.parse(session.created_at) / 1000)}</div>
                  </div>
                </div>
              </Card>
            );
          })}
        </div>
      </div>
    );
  };

  return (
    <div className={`timeline-view ${className}`} ref={containerRef}>
      <ScrollArea className="h-full">
        <div className="space-y-8 p-6">
          {timelineData.timeline.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-64 text-text-muted">
              <Calendar className="w-12 h-12 mb-4" />
              <p className="text-lg mb-2">No sessions to display</p>
              <p className="text-sm">Your chat timeline will appear here</p>
            </div>
          ) : (
            timelineData.timeline.map((data, index) => (
              <TimelineDay
                key={`${data.date.toDateString()}-${index}`}
                data={data}
                events={timelineData.events}
              />
            ))
          )}
        </div>
      </ScrollArea>
    </div>
  );
};

export default SessionTimelineView;
