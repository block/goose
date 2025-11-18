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

interface DayGroup {
  day: string;
  title: string;
  sessions: SessionData[];
  color: string;
}

const SessionTimelineView: React.FC<SessionTimelineViewProps> = ({ 
  sessions, 
  onSessionClick 
}) => {
  // Process sessions into day groups
  const dayGroups = useMemo(() => {
    console.log('SessionTimelineView: Processing sessions', { sessionCount: sessions.length });
    
    if (sessions.length === 0) {
      return [];
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
      
      // Debug session data
      console.log('Session data:', {
        id: session.id,
        description: session.description,
        created_at: session.created_at,
        message_count: session.message_count,
        allKeys: Object.keys(session)
      });
      
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

    // Sort days (newest first) and create groups with colors
    const colors = ['border-red-400', 'border-blue-400', 'border-green-400', 'border-yellow-400', 'border-purple-400', 'border-pink-400', 'border-indigo-400'];
    
    const sortedDays = Array.from(sessionsByDay.keys())
      .sort((a, b) => new Date(b).getTime() - new Date(a).getTime());

    const groups: DayGroup[] = sortedDays.map((day, index) => {
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

      return {
        day,
        title: dayTitle,
        sessions: sessionsByDay.get(day)!,
        color: colors[index % colors.length]
      };
    });

    console.log('SessionTimelineView: Created day groups', { 
      dayCount: groups.length,
      totalSessions: sortedSessions.length,
      sessionsPerDay: Object.fromEntries(groups.map(group => [group.title, group.sessions.length]))
    });

    return groups;
  }, [sessions]);

  // Helper function to get chat type styling
  const getChatTypeStyle = (chatType: string) => {
    switch (chatType) {
      case 'collaborative': return 'bg-green-500 border-green-600';
      case 'direct_message': return 'bg-yellow-500 border-yellow-600';
      case 'group_chat': return 'bg-red-500 border-red-600';
      default: return 'bg-blue-500 border-blue-600';
    }
  };

  if (dayGroups.length === 0) {
    return (
      <div className="flex items-center justify-center h-64 text-gray-500 dark:text-gray-400">
        <div className="text-center">
          <p className="text-lg font-medium">No sessions to display</p>
          <p className="text-sm">Start a conversation to see your tangled tree timeline</p>
        </div>
      </div>
    );
  }

  return (
    <div className="w-full h-full min-h-[600px] bg-background rounded-lg border overflow-auto">
      <div className="p-4 border-b bg-muted/50">
        <h3 className="text-lg font-semibold text-foreground">Tangled Tree Timeline</h3>
        <p className="text-sm text-muted-foreground">
          Days on the left spine, sessions stacked vertically with connections showing relationships
        </p>
      </div>
      
      <div className="relative p-6 bg-background">
        {/* Main spine line */}
        <div className="absolute left-16 top-6 bottom-6 w-0.5 bg-border"></div>
        
        {dayGroups.map((group, groupIndex) => (
          <div key={group.day} className="relative mb-8 last:mb-0">
            {/* Day node */}
            <div className="flex items-start">
              <div className="relative z-10 flex items-center">
                <div className="w-3 h-3 bg-foreground rounded-full border-2 border-background shadow-sm"></div>
                <div className="ml-4 min-w-0">
                  <h4 className="text-sm font-semibold text-foreground">{group.title}</h4>
                  <p className="text-xs text-muted-foreground">
                    {group.sessions.length} session{group.sessions.length !== 1 ? 's' : ''}
                  </p>
                </div>
              </div>
            </div>

            {/* Sessions for this day */}
            <div className="ml-20 mt-4 space-y-3">
              {group.sessions.map((session, sessionIndex) => (
                <div key={session.id} className="relative group">
                  {/* Connection line from spine to session */}
                  <div className={`absolute -left-16 top-3 w-12 h-0.5 ${group.color} opacity-70`}></div>
                  <div className={`absolute -left-4 top-2.5 w-1 h-1 rounded-full ${group.color.replace('border-', 'bg-')}`}></div>
                  
                  {/* Session card */}
                  <div 
                    className={`
                      relative p-3 rounded-lg border bg-card hover:bg-accent/50 
                      cursor-pointer transition-all duration-200 hover:shadow-md
                      border-l-4 ${group.color}
                    `}
                    onClick={() => onSessionClick(session.id)}
                  >
                    <div className="flex items-start justify-between">
                      <div className="min-w-0 flex-1">
                        <h5 className="text-sm font-medium text-card-foreground truncate">
                          {session.title}
                        </h5>
                        <div className="flex items-center gap-2 mt-1">
                          <span className="text-xs text-muted-foreground">
                            {session.message_count} message{session.message_count !== 1 ? 's' : ''}
                          </span>
                          <div className={`w-2 h-2 rounded-full ${getChatTypeStyle(session.chat_type).split(' ')[0]}`}></div>
                        </div>
                      </div>
                    </div>
                  </div>

                  {/* Tangled connections to similar sessions */}
                  {groupIndex < dayGroups.length - 1 && (
                    dayGroups.slice(groupIndex + 1).map(otherGroup => 
                      otherGroup.sessions
                        .filter(otherSession => {
                          // Simple similarity check
                          const titleSimilarity = session.title.toLowerCase().includes(otherSession.title.toLowerCase().split(' ')[0]) ||
                                                 otherSession.title.toLowerCase().includes(session.title.toLowerCase().split(' ')[0]);
                          const typeSimilarity = session.chat_type === otherSession.chat_type;
                          return titleSimilarity || typeSimilarity;
                        })
                        .slice(0, 1) // Limit to one connection per day to avoid clutter
                        .map(similarSession => (
                          <div 
                            key={`tangle-${session.id}-${similarSession.id}`}
                            className="absolute top-3 left-full w-8 h-0.5 bg-yellow-400 opacity-40 transform rotate-12 pointer-events-none"
                            style={{ 
                              transformOrigin: 'left center',
                              width: '60px'
                            }}
                          />
                        ))
                    )
                  )}
                </div>
              ))}
            </div>
          </div>
        ))}

        {/* Legend */}
        <div className="mt-8 pt-4 border-t border-border">
          <div className="flex flex-wrap gap-4 text-xs">
            <div className="flex items-center gap-2">
              <div className="w-3 h-3 bg-foreground rounded-full"></div>
              <span className="text-muted-foreground">Day</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-2 h-2 bg-blue-500 rounded-full"></div>
              <span className="text-muted-foreground">Regular Chat</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-2 h-2 bg-green-500 rounded-full"></div>
              <span className="text-muted-foreground">Collaborative</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-2 h-2 bg-yellow-500 rounded-full"></div>
              <span className="text-muted-foreground">Direct Message</span>
            </div>
            <div className="flex items-center gap-2">
              <div className="w-2 h-2 bg-red-500 rounded-full"></div>
              <span className="text-muted-foreground">Group Chat</span>
            </div>
          </div>
          <div className="text-xs text-muted-foreground mt-2">
            Click sessions to open • Colored connections show day relationships • Similar sessions have tangled connections
          </div>
        </div>
      </div>
    </div>
  );
};

export default SessionTimelineView;
