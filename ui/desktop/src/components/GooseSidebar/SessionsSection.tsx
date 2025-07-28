import React, { useEffect, useState, useCallback, useRef } from 'react';
import { Search, ChevronDown, Folder, Loader2 } from 'lucide-react';
import { fetchSessions, type Session } from '../../sessions';
import { Input } from '../ui/input';
import {
  SidebarMenu,
  SidebarMenuItem,
  SidebarMenuButton,
  SidebarGroup,
  SidebarGroupLabel,
  SidebarGroupContent,
} from '../ui/sidebar';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '../ui/collapsible';
import { useTextAnimator } from '../../hooks/use-text-animator';

interface SessionsSectionProps {
  onSelectSession: (sessionId: string) => void;
  refreshTrigger?: number;
}

interface GroupedSessions {
  today: Session[];
  yesterday: Session[];
  older: { [key: string]: Session[] };
}

export const SessionsSection: React.FC<SessionsSectionProps> = ({
  onSelectSession,
  refreshTrigger,
}) => {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [searchTerm, setSearchTerm] = useState('');
  const [groupedSessions, setGroupedSessions] = useState<GroupedSessions>({
    today: [],
    yesterday: [],
    older: {},
  });
  const [sessionsWithDescriptions, setSessionsWithDescriptions] = useState<Set<string>>(new Set());

  const refreshTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const groupSessions = useCallback((sessionsToGroup: Session[]) => {
    const now = new Date();
    const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
    const yesterday = new Date(today);
    yesterday.setDate(yesterday.getDate() - 1);

    const grouped: GroupedSessions = {
      today: [],
      yesterday: [],
      older: {},
    };

    sessionsToGroup.forEach((session) => {
      const sessionDate = new Date(session.modified);
      const sessionDateOnly = new Date(
        sessionDate.getFullYear(),
        sessionDate.getMonth(),
        sessionDate.getDate()
      );

      if (sessionDateOnly.getTime() === today.getTime()) {
        grouped.today.push(session);
      } else if (sessionDateOnly.getTime() === yesterday.getTime()) {
        grouped.yesterday.push(session);
      } else {
        const dateKey = sessionDateOnly.toISOString().split('T')[0];
        if (!grouped.older[dateKey]) {
          grouped.older[dateKey] = [];
        }
        grouped.older[dateKey].push(session);
      }
    });

    // Sort older sessions by date (newest first)
    const sortedOlder: { [key: string]: Session[] } = {};
    Object.keys(grouped.older)
      .sort()
      .reverse()
      .forEach((key) => {
        sortedOlder[key] = grouped.older[key];
      });

    grouped.older = sortedOlder;
    setGroupedSessions(grouped);
  }, []);

  const loadSessions = useCallback(async () => {
    try {
      const sessions = await fetchSessions();
      setSessions(sessions);
      groupSessions(sessions);
    } catch (err) {
      console.error('Failed to load sessions:', err);
      setSessions([]);
      setGroupedSessions({ today: [], yesterday: [], older: {} });
    }
  }, [groupSessions]);

  // Debounced refresh function
  const debouncedRefresh = useCallback(() => {
    console.log('SessionsSection: Debounced refresh triggered');
    // Clear any existing timeout
    if (refreshTimeoutRef.current) {
      window.clearTimeout(refreshTimeoutRef.current);
    }

    // Set new timeout - reduced to 200ms for faster response
    refreshTimeoutRef.current = setTimeout(() => {
      console.log('SessionsSection: Executing debounced refresh');
      loadSessions();
      refreshTimeoutRef.current = null;
    }, 200);
  }, [loadSessions]);

  // Cleanup timeout on unmount
  useEffect(() => {
    return () => {
      if (refreshTimeoutRef.current) {
        window.clearTimeout(refreshTimeoutRef.current);
      }
    };
  }, []);

  useEffect(() => {
    console.log('SessionsSection: Initial load');
    loadSessions();
  }, [loadSessions]);

  // Add effect to refresh sessions when refreshTrigger changes
  useEffect(() => {
    if (refreshTrigger) {
      console.log('SessionsSection: Refresh trigger changed, triggering refresh');
      debouncedRefresh();
    }
  }, [refreshTrigger, debouncedRefresh]);

  // Add effect to listen for session creation events
  useEffect(() => {
    const handleSessionCreated = () => {
      console.log('SessionsSection: Session created event received');
      debouncedRefresh();
    };

    const handleMessageStreamFinish = () => {
      console.log('SessionsSection: Message stream finished event received');
      // Always refresh when message stream finishes
      debouncedRefresh();
    };

    // Listen for custom events that indicate a session was created
    window.addEventListener('session-created', handleSessionCreated);

    // Also listen for message stream finish events
    window.addEventListener('message-stream-finished', handleMessageStreamFinish);

    return () => {
      window.removeEventListener('session-created', handleSessionCreated);
      window.removeEventListener('message-stream-finished', handleMessageStreamFinish);
    };
  }, [debouncedRefresh]);

  useEffect(() => {
    if (searchTerm) {
      const filtered = sessions.filter((session) =>
        (session.metadata.description || session.id)
          .toLowerCase()
          .includes(searchTerm.toLowerCase())
      );
      groupSessions(filtered);
    } else {
      groupSessions(sessions);
    }
  }, [searchTerm, sessions, groupSessions]);

  // Component for individual session items with loading and animation states
  const SessionItem = ({ session }: { session: Session }) => {
    const hasDescription =
      session.metadata.description && session.metadata.description.trim() !== '';
    const isNewSession = session.id.match(/^\d{8}_\d{6}$/);
    const messageCount = session.metadata.message_count || 0;
    // Show loading for new sessions with few messages and no description
    // Only show loading for sessions created in the last 5 minutes
    const sessionDate = new Date(session.modified);
    const fiveMinutesAgo = new Date(Date.now() - 5 * 60 * 1000);
    const isRecentSession = sessionDate > fiveMinutesAgo;
    const shouldShowLoading =
      !hasDescription && isNewSession && messageCount <= 2 && isRecentSession;
    const [isAnimating, setIsAnimating] = useState(false);

    // Use text animator only for sessions that need animation
    const descriptionRef = useTextAnimator({
      text: isAnimating ? session.metadata.description : '',
    });

    // Track when description becomes available and trigger animation
    useEffect(() => {
      if (hasDescription && !sessionsWithDescriptions.has(session.id)) {
        setSessionsWithDescriptions((prev) => new Set(prev).add(session.id));

        // Only animate for new sessions that were showing loading
        if (shouldShowLoading) {
          setIsAnimating(true);
        }
      }
    }, [hasDescription, session.id, shouldShowLoading]);

    const handleClick = () => {
      console.log('SessionItem: Clicked on session:', session.id);
      onSelectSession(session.id);
    };

    return (
      <SidebarMenuItem key={session.id}>
        <SidebarMenuButton
          onClick={handleClick}
          className="cursor-pointer w-56 transition-all duration-300 ease-in-out hover:bg-background-medium hover:shadow-sm rounded-xl text-text-muted hover:text-text-default h-fit flex items-start transform hover:scale-[1.02] active:scale-[0.98]"
        >
          <div className="flex flex-col w-full">
            <div className="text-sm w-48 truncate mb-1 px-1 text-ellipsis text-text-default flex items-center gap-2">
              {shouldShowLoading ? (
                <div className="flex items-center gap-2 animate-in fade-in duration-300">
                  <Loader2 className="size-3 animate-spin text-text-default" />
                  <span className="text-text-default animate-pulse">Generating description...</span>
                </div>
              ) : (
                <span
                  ref={isAnimating ? descriptionRef : undefined}
                  className={`transition-all duration-300 ${isAnimating ? 'animate-in fade-in duration-300' : ''}`}
                >
                  {hasDescription ? session.metadata.description : `Session ${session.id}`}
                </span>
              )}
            </div>
            <div className="text-xs w-48 truncate px-1 flex items-center gap-2 text-ellipsis transition-colors duration-300">
              <Folder className="size-4 transition-transform duration-300 group-hover:scale-110" />
              <span className="transition-all duration-300">{session.metadata.working_dir}</span>
            </div>
          </div>
        </SidebarMenuButton>
      </SidebarMenuItem>
    );
  };

  const renderSessionGroup = (sessions: Session[], title: string, index: number) => {
    if (sessions.length === 0) return null;

    const isFirstTwoGroups = index < 2;

    return (
      <Collapsible defaultOpen={isFirstTwoGroups} className="group/collapsible">
        <SidebarGroup>
          <CollapsibleTrigger className="w-full">
            <SidebarGroupLabel className="flex cursor-pointer items-center justify-between text-text-default hover:text-text-default h-12 pl-3 transition-all duration-200 rounded-lg">
              <div className="flex min-w-0 items-center">
                <span className="opacity-100 transition-all duration-300 text-xs font-medium">
                  {title}
                </span>
              </div>
              <ChevronDown className="size-4 text-text-muted flex-shrink-0 opacity-100 transition-all duration-300 ease-in-out group-data-[state=open]/collapsible:rotate-180" />
            </SidebarGroupLabel>
          </CollapsibleTrigger>
          <CollapsibleContent className="data-[state=open]:animate-collapsible-down data-[state=closed]:animate-collapsible-up overflow-hidden transition-all duration-300 ease-in-out">
            <SidebarGroupContent>
              <SidebarMenu className="mb-2 space-y-1">
                {sessions.map((session, sessionIndex) => (
                  <div
                    key={session.id}
                    className="animate-in slide-in-from-left-2 fade-in duration-300"
                    style={{
                      animationDelay: `${sessionIndex * 50}ms`,
                      animationFillMode: 'both',
                    }}
                  >
                    <SessionItem session={session} />
                  </div>
                ))}
              </SidebarMenu>
            </SidebarGroupContent>
          </CollapsibleContent>
        </SidebarGroup>
      </Collapsible>
    );
  };

  return (
    <Collapsible defaultOpen={false} className="group/collapsible rounded-xl">
      <SidebarGroup className="px-1">
        <CollapsibleTrigger className="w-full">
          <SidebarGroupLabel className="flex cursor-pointer items-center py-6 justify-between text-text-default px-4 transition-all duration-200 hover:bg-background-default rounded-lg">
            <div className="flex min-w-0 items-center">
              <span className="text-sm">Sessions</span>
            </div>
            <ChevronDown className="size-4 text-text-muted flex-shrink-0 opacity-100 transition-all duration-300 ease-in-out group-data-[state=open]/collapsible:rotate-180" />
          </SidebarGroupLabel>
        </CollapsibleTrigger>
        <CollapsibleContent className="data-[state=open]:animate-collapsible-down data-[state=closed]:animate-collapsible-up overflow-hidden transition-all duration-300 ease-in-out">
          <SidebarGroupContent>
            {/* Search Input */}
            <div className="p-1 pb-2 animate-in slide-in-from-top-2 fade-in duration-300">
              <div className="relative flex flex-row items-center gap-2">
                <Search className="absolute top-2.5 left-2.5 size-4 text-muted-foreground" />
                <Input
                  type="search"
                  placeholder="Search sessions..."
                  className="pl-8 transition-all duration-200 focus:ring-2 focus:ring-borderProminent"
                  value={searchTerm}
                  onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                    setSearchTerm(e.target.value)
                  }
                />
              </div>
            </div>

            {/* Sessions Groups */}
            <div className="space-y-2">
              {(() => {
                let groupIndex = 0;
                const groups = [
                  { sessions: groupedSessions.today, title: 'Today' },
                  { sessions: groupedSessions.yesterday, title: 'Yesterday' },
                  ...Object.entries(groupedSessions.older).map(([date, sessions]) => ({
                    sessions,
                    title: new Date(date).toLocaleDateString('en-US', {
                      weekday: 'long',
                      year: 'numeric',
                      month: 'long',
                      day: 'numeric',
                    }),
                  })),
                ];

                return groups.map(({ sessions, title }) => {
                  if (sessions.length === 0) return null;
                  const currentIndex = groupIndex++;
                  return (
                    <div
                      key={title}
                      className="animate-in slide-in-from-left-2 fade-in duration-300"
                      style={{
                        animationDelay: `${currentIndex * 100}ms`,
                        animationFillMode: 'both',
                      }}
                    >
                      {renderSessionGroup(sessions, title, currentIndex)}
                    </div>
                  );
                });
              })()}
            </div>
          </SidebarGroupContent>
        </CollapsibleContent>
      </SidebarGroup>
    </Collapsible>
  );
};
