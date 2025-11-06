import { useEffect, useState } from 'react';
import { Card, CardContent, CardDescription } from '../ui/card';
import { Greeting } from '../common/Greeting';
import { useNavigate } from 'react-router-dom';
import { Button } from '../ui/button';
import { ChatSmart } from '../icons/';
import { Goose } from '../icons/Goose';
import { Skeleton } from '../ui/skeleton';
import {
  getSessionInsights,
  listSessions,
  Session,
  SessionInsights as ApiSessionInsights,
} from '../../api';
import { resumeSession } from '../../sessions';
import { useNavigation } from '../../hooks/useNavigation';

export function SessionInsights() {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const [insights, setInsights] = useState<ApiSessionInsights | null>(null);
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  const [error, setError] = useState<string | null>(null);
  const [recentSessions, setRecentSessions] = useState<Session[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isLoadingSessions, setIsLoadingSessions] = useState(true);
  const navigate = useNavigate();
  const setView = useNavigation();

  useEffect(() => {
    let loadingTimeout: ReturnType<typeof setTimeout>;

    const loadInsights = async () => {
      try {
        const response = await getSessionInsights({ throwOnError: true });
        setInsights(response.data);
        setError(null);
      } catch (error) {
        console.error('Failed to load insights:', error);
        setError(error instanceof Error ? error.message : 'Failed to load insights');
        setInsights({
          totalSessions: 0,
          totalTokens: 0,
        });
      } finally {
        setIsLoading(false);
      }
    };

    const loadRecentSessions = async () => {
      try {
        const response = await listSessions<true>({ throwOnError: true });
        setRecentSessions(response.data.sessions.slice(0, 3));
      } finally {
        setIsLoadingSessions(false);
      }
    };

    // Set a maximum loading time to prevent infinite skeleton
    loadingTimeout = setTimeout(() => {
      // Only apply fallback if we still don't have insights data
      setInsights((currentInsights) => {
        if (!currentInsights) {
          console.warn('Loading timeout reached, showing fallback content');
          setError('Failed to load insights');
          setIsLoading(false);
          return {
            totalSessions: 0,
            mostActiveDirs: [],
            avgSessionDuration: 0,
            totalTokens: 0,
            recentActivity: [],
          };
        }
        // If we already have insights, just make sure loading is false
        setIsLoading(false);
        return currentInsights;
      });
    }, 10000); // 10 second timeout

    loadInsights();
    loadRecentSessions();

    // Cleanup timeout on unmount
    return () => {
      if (loadingTimeout) {
        window.clearTimeout(loadingTimeout);
      }
    };
  }, []);

  const handleSessionClick = async (session: Session) => {
    try {
      resumeSession(session, setView);
    } catch (error) {
      console.error('Failed to start session:', error);
      navigate('/sessions', {
        state: { selectedSessionId: session.id },
        replace: true,
      });
    }
  };

  const navigateToSessionHistory = () => {
    navigate('/sessions');
  };

  // Format date to show only the date part (without time)
  const formatDateOnly = (dateStr: string) => {
    const date = new Date(dateStr);
    return date
      .toLocaleDateString('en-US', { month: '2-digit', day: '2-digit', year: 'numeric' })
      .replace(/\//g, '/');
  };

  // Render skeleton loader while data is loading
  const renderSkeleton = () => (
    <div className="bg-background-muted flex flex-col h-full">
      {/* Header container with rounded top and bottom */}
      <div className="bg-background-default rounded-2xl mb-0.5">
        <div className="px-8 pb-12 pt-19 space-y-4">
          <div className="origin-bottom-left goose-icon-animation">
            <Goose className="size-8" />
          </div>
          <Greeting />
        </div>
      </div>

      {/* Stats containers - full bleed with 2px gaps */}
      <div className="flex flex-col flex-1 space-y-0.5">
        {/* Recent Chats Card Skeleton */}
        <div className="grid grid-cols-1 gap-0.5">
          <Card className="w-full py-6 px-6 border-none rounded-2xl bg-background-default">
            <CardContent className="p-0">
              <div className="flex justify-between items-center mb-4">
                <CardDescription className="mb-0">
                  <span className="text-lg text-text-default">Recent chats</span>
                </CardDescription>
                <Button
                  variant="ghost"
                  size="sm"
                  className="text-xs text-text-muted flex items-center gap-1 !px-0 hover:bg-transparent hover:underline hover:text-text-default"
                  onClick={navigateToSessionHistory}
                >
                  See all
                </Button>
              </div>
              <div className="space-y-3 min-h-[96px]">
                {/* Skeleton chat items */}
                <div className="flex items-center justify-between py-1 px-2">
                  <div className="flex items-center space-x-2">
                    <Skeleton className="h-4 w-4 rounded-sm" />
                    <Skeleton className="h-4 w-48" />
                  </div>
                  <Skeleton className="h-4 w-16" />
                </div>
                <div className="flex items-center justify-between py-1 px-2">
                  <div className="flex items-center space-x-2">
                    <Skeleton className="h-4 w-4 rounded-sm" />
                    <Skeleton className="h-4 w-40" />
                  </div>
                  <Skeleton className="h-4 w-16" />
                </div>
                <div className="flex items-center justify-between py-1 px-2">
                  <div className="flex items-center space-x-2">
                    <Skeleton className="h-4 w-4 rounded-sm" />
                    <Skeleton className="h-4 w-52" />
                  </div>
                  <Skeleton className="h-4 w-16" />
                </div>
              </div>
            </CardContent>
          </Card>
        </div>

        {/* Filler container - extends to fill remaining space */}
        <div className="bg-background-default rounded-2xl flex-1"></div>
      </div>
    </div>
  );

  // Show skeleton while loading, then show actual content
  if (isLoading) {
    return renderSkeleton();
  }

  return (
    <div className="bg-background-muted flex flex-col h-full">
      {/* Header container with rounded top and bottom */}
      <div className="bg-background-default rounded-2xl mb-0.5">
        <div className="px-8 pb-12 pt-19 space-y-4">
          <div className="origin-bottom-left goose-icon-animation">
            <Goose className="size-8" />
          </div>
          <Greeting />
        </div>
      </div>

      {/* Stats containers - full bleed with 2px gaps */}
      <div className="flex flex-col flex-1 space-y-0.5">
        {/* Recent Chats Card */}
        <div className="grid grid-cols-1 gap-0.5">
          {/* Recent Chats Card */}
          <Card className="w-full py-6 px-6 border-none rounded-2xl bg-background-default">
            <CardContent className="page-transition p-0">
              <div className="flex justify-between items-center mb-4">
                <CardDescription className="mb-0">
                  <span className="text-lg text-text-default">Recent chats</span>
                </CardDescription>
                <Button
                  variant="ghost"
                  size="sm"
                  className="text-xs text-text-muted flex items-center gap-1 !px-0 hover:bg-transparent hover:underline hover:text-text-default"
                  onClick={navigateToSessionHistory}
                >
                  See all
                </Button>
              </div>
              <div className="space-y-1 min-h-[96px] transition-all duration-300 ease-in-out">
                {isLoadingSessions ? (
                  // Show skeleton while sessions are loading
                  <>
                    <div className="flex items-center justify-between py-1 px-2">
                      <div className="flex items-center space-x-2">
                        <Skeleton className="h-4 w-4 rounded-sm" />
                        <Skeleton className="h-4 w-48" />
                      </div>
                      <Skeleton className="h-4 w-16" />
                    </div>
                    <div className="flex items-center justify-between py-1 px-2">
                      <div className="flex items-center space-x-2">
                        <Skeleton className="h-4 w-4 rounded-sm" />
                        <Skeleton className="h-4 w-40" />
                      </div>
                      <Skeleton className="h-4 w-16" />
                    </div>
                    <div className="flex items-center justify-between py-1 px-2">
                      <div className="flex items-center space-x-2">
                        <Skeleton className="h-4 w-4 rounded-sm" />
                        <Skeleton className="h-4 w-52" />
                      </div>
                      <Skeleton className="h-4 w-16" />
                    </div>
                  </>
                ) : recentSessions.length > 0 ? (
                  recentSessions.map((session, index) => (
                    <div
                      key={session.id}
                      className="flex items-center justify-between text-sm py-1 px-2 rounded-md hover:bg-background-muted cursor-pointer transition-colors session-item"
                      onClick={() => handleSessionClick(session)}
                      role="button"
                      tabIndex={0}
                      style={{ animationDelay: `${index * 0.1}s` }}
                      onKeyDown={async (e) => {
                        if (e.key === 'Enter' || e.key === ' ') {
                          await handleSessionClick(session);
                        }
                      }}
                    >
                      <div className="flex items-center space-x-2">
                        <ChatSmart className="h-4 w-4 text-text-muted" />
                        <span className="truncate max-w-[300px]">{session.name}</span>
                      </div>
                      <span className="text-text-muted font-mono font-light">
                        {formatDateOnly(session.updated_at)}
                      </span>
                    </div>
                  ))
                ) : (
                  <div className="text-text-muted text-sm py-2">No recent chat sessions found.</div>
                )}
              </div>
            </CardContent>
          </Card>
        </div>

        {/* Filler container - extends to fill remaining space */}
        <div className="bg-background-default rounded-2xl flex-1"></div>
      </div>
    </div>
  );
}
