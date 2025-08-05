import { useEffect, useState } from 'react';
import { Card, CardContent, CardDescription } from '../ui/card';
// import { Folder } from 'lucide-react';
import { getApiUrl } from '../../config';
import { Greeting } from '../common/Greeting';
import { fetchSessions, fetchSessionDetails, type Session } from '../../sessions';
// import { fetchProjects, type ProjectMetadata } from '../../projects';
import { useNavigate } from 'react-router-dom';
import { Button } from '../ui/button';
import { ChatSmart } from '../icons/';
import { Goose } from '../icons/Goose';
import { Skeleton } from '../ui/skeleton';

interface SessionInsightsType {
  totalSessions: number;
  mostActiveDirs: [string, number][];
  avgSessionDuration: number;
  totalTokens: number;
}

export function SessionInsights() {
  const [insights, setInsights] = useState<SessionInsightsType | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [recentSessions, setRecentSessions] = useState<Session[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isLoadingSessions, setIsLoadingSessions] = useState(true);
  // const [recentProjects, setRecentProjects] = useState<ProjectMetadata[]>([]);
  const navigate = useNavigate();

  useEffect(() => {
    let loadingTimeout: ReturnType<typeof setTimeout>;

    const loadInsights = async () => {
      try {
        const response = await fetch(getApiUrl('/sessions/insights'), {
          headers: {
            Accept: 'application/json',
            'Content-Type': 'application/json',
            'X-Secret-Key': await window.electron.getSecretKey(),
          },
        });

        if (!response.ok) {
          const errorText = await response.text();
          throw new Error(`Failed to fetch insights: ${response.status} ${errorText}`);
        }

        const data = await response.json();
        setInsights(data);
        // Clear any previous error when insights load successfully
        setError(null);
      } catch (error) {
        console.error('Failed to load insights:', error);
        setError(error instanceof Error ? error.message : 'Failed to load insights');
        // Set fallback insights data so the UI can still render
        setInsights({
          totalSessions: 0,
          mostActiveDirs: [],
          avgSessionDuration: 0,
          totalTokens: 0,
        });
      } finally {
        setIsLoading(false);
      }
    };

    const loadRecentSessions = async () => {
      try {
        const sessions = await fetchSessions();
        setRecentSessions(sessions.slice(0, 3));
      } catch (error) {
        console.error('Failed to load recent sessions:', error);
      } finally {
        setIsLoadingSessions(false);
      }
    };

    // const loadRecentProjects = async () => {
    //   try {
    //     const projects = await fetchProjects();
    //     setRecentProjects(projects.slice(0, 3));
    //   } catch (error) {
    //     console.error('Failed to load recent projects:', error);
    //   }
    // };

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
          };
        }
        // If we already have insights, just make sure loading is false
        setIsLoading(false);
        return currentInsights;
      });
    }, 10000); // 10 second timeout

    loadInsights();
    loadRecentSessions();
    // loadRecentProjects();

    // Cleanup timeout on unmount
    return () => {
      if (loadingTimeout) {
        window.clearTimeout(loadingTimeout);
      }
    };
  }, []);

  const handleSessionClick = async (sessionId: string) => {
    try {
      // Fetch the session details
      const sessionDetails = await fetchSessionDetails(sessionId);

      // Navigate to pair view with the resumed session
      navigate('/pair', {
        state: { resumedSession: sessionDetails },
        replace: true,
      });
    } catch (error) {
      console.error('Failed to load session:', error);
      // Fallback to the sessions view if loading fails
      navigate('/sessions', {
        state: { selectedSessionId: sessionId },
        replace: true,
      });
    }
  };

  const navigateToSessionHistory = () => {
    navigate('/sessions');
  };

  // const navigateToProjects = () => {
  //   navigate('/projects');
  // };
  //
  // const handleProjectClick = (projectId: string) => {
  //   navigate('/projects', {
  //     state: { selectedProjectId: projectId },
  //     replace: true,
  //   });
  // };

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
      {/* Header container with rounded bottom */}
      <div className="bg-background-default rounded-b-2xl mb-0.5">
        <div className="px-8 pb-12 pt-19 space-y-4">
          <div className="origin-bottom-left goose-icon-animation">
            <Goose className="size-8" />
          </div>
          <Greeting />
        </div>
      </div>

      {/* Stats containers - full bleed with 2px gaps */}
      <div className="flex flex-col flex-1 space-y-0.5">
        {/* Top row with three equal columns */}
        <div className="grid grid-cols-2 gap-0.5">
          {/* Total Sessions Card Skeleton */}
          <Card className="w-full py-6 px-6 border-none rounded-2xl bg-background-default">
            <CardContent className="flex flex-col justify-end h-full p-0">
              <div className="flex flex-col justify-end">
                <Skeleton className="h-10 w-16 mb-1" />
                <span className="text-xs text-text-muted">Total sessions</span>
              </div>
            </CardContent>
          </Card>

          {/* Average Duration Card Skeleton */}
          {/*<Card className="w-full py-6 px-6 border-none rounded-2xl bg-background-default">*/}
          {/*  <CardContent className="flex flex-col justify-end h-full p-0">*/}
          {/*    <div className="flex flex-col justify-end">*/}
          {/*      <Skeleton className="h-10 w-20 mb-1" />*/}
          {/*      <span className="text-xs text-text-muted">Avg. chat length</span>*/}
          {/*    </div>*/}
          {/*  </CardContent>*/}
          {/*</Card>*/}

          {/* Total Tokens Card Skeleton */}
          <Card className="w-full py-6 px-6 border-none rounded-2xl bg-background-default">
            <CardContent className="flex flex-col justify-end h-full p-0">
              <div className="flex flex-col justify-end">
                <Skeleton className="h-10 w-24 mb-1" />
                <span className="text-xs text-text-muted">Total tokens</span>
              </div>
            </CardContent>
          </Card>
        </div>

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
      {/* Header container with rounded bottom */}
      <div className="bg-background-default rounded-b-2xl mb-0.5">
        <div className="px-8 pb-12 pt-19 space-y-4">
          <div className="origin-bottom-left goose-icon-animation">
            <Goose className="size-8" />
          </div>
          <Greeting />
        </div>
      </div>

      {/* Stats containers - full bleed with 2px gaps */}
      <div className="flex flex-col flex-1 space-y-0.5">
        {/* Error notice if insights failed to load */}
        {error && (
          <div className="mx-0.5 px-4 py-2 bg-orange-50 dark:bg-orange-950/20 border border-orange-200 dark:border-orange-800/30 rounded-xl">
            <div className="flex items-center space-x-2">
              <div className="w-2 h-2 bg-orange-400 rounded-full flex-shrink-0"></div>
              <span className="text-xs text-orange-700 dark:text-orange-300">
                Failed to load insights
              </span>
            </div>
          </div>
        )}

        {/* Top row with three equal columns */}
        <div className="grid grid-cols-2 gap-0.5">
          {/* Total Sessions Card */}
          <Card className="w-full py-6 px-6 border-none rounded-2xl bg-background-default">
            <CardContent className="page-transition flex flex-col justify-end h-full p-0">
              <div className="flex flex-col justify-end">
                <p className="text-4xl font-mono font-light flex items-end">
                  {Math.max(insights?.totalSessions ?? 0, 0)}
                </p>
                <span className="text-xs text-text-muted">Total sessions</span>
              </div>
            </CardContent>
          </Card>

          {/* Average Duration Card */}
          {/*<Card className="w-full py-6 px-6 border-none rounded-2xl bg-background-default">*/}
          {/*  <CardContent className="page-transition flex flex-col justify-end h-full p-0">*/}
          {/*    <div className="flex flex-col justify-end">*/}
          {/*      <p className="text-4xl font-mono font-light flex items-end">*/}
          {/*        {insights?.avgSessionDuration*/}
          {/*          ? `${insights.avgSessionDuration.toFixed(1)}m`*/}
          {/*          : '0.0m'}*/}
          {/*      </p>*/}
          {/*      <span className="text-xs text-text-muted">Avg. chat length</span>*/}
          {/*    </div>*/}
          {/*  </CardContent>*/}
          {/*</Card>*/}

          {/* Total Tokens Card */}
          <Card className="w-full py-6 px-6 border-none rounded-2xl bg-background-default">
            <CardContent className="page-transition flex flex-col justify-end h-full p-0">
              <div className="flex flex-col justify-end">
                <p className="text-4xl font-mono font-light flex items-end">
                  {insights?.totalTokens && insights.totalTokens > 0
                    ? `${(insights.totalTokens / 1000000).toFixed(2)}M`
                    : '0.00M'}
                </p>
                <span className="text-xs text-text-muted">Total tokens</span>
              </div>
            </CardContent>
          </Card>
        </div>

        {/* Recent Chats Card */}
        <div className="grid grid-cols-1 gap-0.5">
          {/* Recent Projects Card */}
          {/*<Card className="w-full py-6 px-4 border-none rounded-tl-none rounded-bl-none">*/}
          {/*  <CardContent className="animate-in fade-in duration-500 px-4">*/}
          {/*    <div className="flex justify-between items-center mb-2 px-2">*/}
          {/*      <CardDescription className="mb-0">*/}
          {/*        <span className="text-lg text-text-default">Recent projects</span>*/}
          {/*      </CardDescription>*/}
          {/*      <Button*/}
          {/*        variant="ghost"*/}
          {/*        size="sm"*/}
          {/*        className="text-xs text-text-muted flex items-center gap-1 !px-0 hover:bg-transparent hover:underline hover:text-text-default"*/}
          {/*        onClick={navigateToProjects}*/}
          {/*      >*/}
          {/*        See all*/}
          {/*      </Button>*/}
          {/*    </div>*/}
          {/*    <div className="space-y-1 min-h-[96px] transition-all duration-300 ease-in-out">*/}
          {/*      <AnimatePresence>*/}
          {/*        {recentProjects.length > 0 ? (*/}
          {/*          recentProjects.map((project, index) => (*/}
          {/*            <motion.div*/}
          {/*              key={project.id}*/}
          {/*              className="flex items-center justify-between text-sm py-1 px-2 rounded-md hover:bg-background-muted cursor-pointer transition-colors"*/}
          {/*              onClick={() => handleProjectClick(project.id)}*/}
          {/*              role="button"*/}
          {/*              tabIndex={0}*/}
          {/*              initial={{ opacity: 0, y: 5 }}*/}
          {/*              animate={{ opacity: 1, y: 0 }}*/}
          {/*              transition={{ duration: 0.3, delay: index * 0.1 }}*/}
          {/*              onKeyDown={(e) => {*/}
          {/*                if (e.key === 'Enter' || e.key === ' ') {*/}
          {/*                  handleProjectClick(project.id);*/}
          {/*                }*/}
          {/*              }}*/}
          {/*            >*/}
          {/*              <div className="flex items-center space-x-2">*/}
          {/*                <Folder className="h-4 w-4 text-text-muted" />*/}
          {/*                <span className="truncate max-w-[200px]">{project.name}</span>*/}
          {/*              </div>*/}
          {/*              <span className="text-text-muted font-mono font-light">*/}
          {/*                {formatDateOnly(project.updatedAt)}*/}
          {/*              </span>*/}
          {/*            </motion.div>*/}
          {/*          ))*/}
          {/*        ) : (*/}
          {/*          <div className="text-text-muted text-sm py-2 px-2">*/}
          {/*            No recent projects found.*/}
          {/*          </div>*/}
          {/*        )}*/}
          {/*      </AnimatePresence>*/}
          {/*    </div>*/}
          {/*  </CardContent>*/}
          {/*</Card>*/}

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
                      onClick={() => handleSessionClick(session.id)}
                      role="button"
                      tabIndex={0}
                      style={{ animationDelay: `${index * 0.1}s` }}
                      onKeyDown={async (e) => {
                        if (e.key === 'Enter' || e.key === ' ') {
                          await handleSessionClick(session.id);
                        }
                      }}
                    >
                      <div className="flex items-center space-x-2">
                        <ChatSmart className="h-4 w-4 text-text-muted" />
                        <span className="truncate max-w-[300px]">
                          {session.metadata.description || session.id}
                        </span>
                      </div>
                      <span className="text-text-muted font-mono font-light">
                        {formatDateOnly(session.modified)}
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
