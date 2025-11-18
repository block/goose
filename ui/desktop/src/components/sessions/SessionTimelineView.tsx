import React, { useMemo } from 'react';
import { Session } from '../../api/types.gen';
import { formatDistanceToNow } from 'date-fns';

interface SessionTimelineViewProps {
  sessions: Session[];
  onSessionClick: (sessionId: string) => void;
}

interface SessionData {
  id: string;
  title: string;
  created_at: string;
  updated_at: string;
  message_count: number;
  chat_type: string;
  day: string;
}

interface DayColumn {
  day: string;
  title: string;
  sessions: SessionData[];
}

const SessionTimelineView: React.FC<SessionTimelineViewProps> = ({ 
  sessions, 
  onSessionClick 
}) => {
  // Process sessions into day columns
  const dayColumns = useMemo(() => {
    console.log('SessionTimelineView: Processing sessions', { sessionCount: sessions.length });
    
    if (sessions.length === 0) {
      return [];
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

    // Sort days (newest first) and create columns
    const sortedDays = Array.from(sessionsByDay.keys())
      .sort((a, b) => new Date(b).getTime() - new Date(a).getTime());

    const columns: DayColumn[] = sortedDays.map(day => {
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

      return {
        day,
        title: dayTitle,
        sessions: sessionsByDay.get(day)!
      };
    });

    console.log('SessionTimelineView: Created day columns', { 
      dayCount: columns.length,
      totalSessions: sortedSessions.length,
      sessionsPerDay: Object.fromEntries(columns.map(col => [col.title, col.sessions.length]))
    });

    return columns;
  }, [sessions]);

  // Helper function to get chat type color
  const getChatTypeColor = (chatType: string) => {
    switch (chatType) {
      case 'collaborative': return 'bg-green-500';
      case 'direct_message': return 'bg-yellow-500';
      case 'group_chat': return 'bg-red-500';
      default: return 'bg-purple-500';
    }
  };

  // Helper function to get node size based on message count
  const getNodeSize = (messageCount: number) => {
    const size = Math.max(16, Math.min(32, Math.sqrt(messageCount) * 4 + 16));
    return `${size}px`;
  };

  if (dayColumns.length === 0) {
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
    <div className="w-full h-full min-h-[600px] bg-white rounded-lg border overflow-auto">
      <div className="p-4 border-b bg-gray-50">
        <h3 className="text-lg font-semibold text-gray-900">Tangled Tree Timeline</h3>
        <p className="text-sm text-gray-600">
          Days across the top, sessions flowing down with connections showing relationships
        </p>
      </div>
      
      <div className="p-6 overflow-x-auto">
        <div className="flex gap-8 min-w-fit">
          {dayColumns.map((column, columnIndex) => (
            <div key={column.day} className="flex flex-col items-center min-w-[200px]">
              {/* Day Header */}
              <div className="mb-6">
                <div className="flex flex-col items-center">
                  <div className="w-6 h-6 bg-blue-600 rounded-full border-2 border-white shadow-md"></div>
                  <div className="mt-2 text-sm font-bold text-gray-900 text-center">
                    {column.title}
                  </div>
                  <div className="text-xs text-gray-500 mt-1">
                    {column.sessions.length} session{column.sessions.length !== 1 ? 's' : ''}
                  </div>
                </div>
              </div>

              {/* Vertical Line */}
              <div className="w-0.5 bg-blue-300 h-8"></div>

              {/* Sessions */}
              <div className="flex flex-col gap-4 items-center">
                {column.sessions.map((session, sessionIndex) => (
                  <div key={session.id} className="flex flex-col items-center group">
                    {/* Connection Line */}
                    {sessionIndex === 0 && (
                      <div className="w-0.5 bg-blue-300 h-4"></div>
                    )}
                    
                    {/* Session Node */}
                    <div 
                      className={`
                        rounded-full border-2 border-white shadow-lg cursor-pointer
                        transition-all duration-200 hover:scale-110 hover:shadow-xl
                        ${getChatTypeColor(session.chat_type)}
                        group-hover:ring-2 group-hover:ring-blue-300
                      `}
                      style={{ 
                        width: getNodeSize(session.message_count),
                        height: getNodeSize(session.message_count)
                      }}
                      onClick={() => onSessionClick(session.id)}
                      title={`${session.title} (${session.message_count} messages)`}
                    />

                    {/* Session Info */}
                    <div className="mt-2 text-center max-w-[180px]">
                      <div className="text-xs font-medium text-gray-900 truncate">
                        {session.title.length > 25 ? session.title.substring(0, 25) + "..." : session.title}
                      </div>
                      <div className="text-xs text-gray-500 mt-1">
                        {session.message_count} msg{session.message_count !== 1 ? 's' : ''}
                      </div>
                    </div>

                    {/* Connection to next session */}
                    {sessionIndex < column.sessions.length - 1 && (
                      <div className="w-0.5 bg-blue-300 h-6 mt-2"></div>
                    )}
                  </div>
                ))}
              </div>

              {/* Tangled Connections (simplified visual indicators) */}
              {columnIndex < dayColumns.length - 1 && (
                <div className="absolute top-32 left-full w-8 h-0.5 bg-yellow-400 opacity-50 transform rotate-12 pointer-events-none" 
                     style={{ 
                       left: `${(columnIndex + 1) * 200 - 50}px`,
                       display: column.sessions.some(s1 => 
                         dayColumns.slice(columnIndex + 1).some(otherCol => 
                           otherCol.sessions.some(s2 => 
                             s1.chat_type === s2.chat_type || 
                             s1.title.toLowerCase().includes(s2.title.toLowerCase().split(' ')[0])
                           )
                         )
                       ) ? 'block' : 'none'
                     }}
                />
              )}
            </div>
          ))}
        </div>

        {/* Legend */}
        <div className="mt-8 pt-4 border-t border-gray-200">
          <div className="flex flex-wrap gap-4 text-xs">
            <div className="flex items-center gap-2">
              <div className="w-3 h-3 bg-blue-600 rounded-full"></div>
              <span className="text-gray-600">Day</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-3 h-3 bg-purple-500 rounded-full"></div>
              <span className="text-gray-600">Regular Chat</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-3 h-3 bg-green-500 rounded-full"></div>
              <span className="text-gray-600">Collaborative</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-3 h-3 bg-yellow-500 rounded-full"></div>
              <span className="text-gray-600">Direct Message</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-3 h-3 bg-red-500 rounded-full"></div>
              <span className="text-gray-600">Group Chat</span>
            </div>
          </div>
          <div className="text-xs text-gray-500 mt-2">
            Node size reflects message count • Click sessions to open • Hover for details
          </div>
        </div>
      </div>
    </div>
  );
};

export default SessionTimelineView;
