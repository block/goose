import { useEffect, useState } from 'react';
import { Card, CardContent, CardDescription } from '../ui/card';
// import { Folder } from 'lucide-react';
import { getApiUrl, getSecretKey } from '../../config';
import { Greeting } from '../common/Greeting';
import { fetchSessions, type Session } from '../../sessions';
// import { fetchProjects, type ProjectMetadata } from '../../projects';
import { useNavigate } from 'react-router-dom';
import { Button } from '../ui/button';
import { ChatSmart } from '../icons/';
import { motion, AnimatePresence } from 'framer-motion';
import { Goose } from '../icons/Goose';

interface SessionInsightsType {
  totalSessions: number;
  mostActiveDirs: [string, number][];
  avgSessionDuration: number;
  totalTokens: number;
}

export function SessionInsights() {
  const [insights, setInsights] = useState<SessionInsightsType | null>(null);
  const [_error, setError] = useState<string | null>(null);
  const [recentSessions, setRecentSessions] = useState<Session[]>([]);
  // const [recentProjects, setRecentProjects] = useState<ProjectMetadata[]>([]);
  const navigate = useNavigate();

  useEffect(() => {
    const loadInsights = async () => {
      try {
        const response = await fetch(getApiUrl('/sessions/insights'), {
          headers: {
            Accept: 'application/json',
            'Content-Type': 'application/json',
            'X-Secret-Key': getSecretKey(),
          },
        });

        if (!response.ok) {
          const errorText = await response.text();
          throw new Error(`Failed to fetch insights: ${response.status} ${errorText}`);
        }

        const data = await response.json();
        setInsights(data);
      } catch (error) {
        setError(error instanceof Error ? error.message : 'Failed to load insights');
      }
    };

    const loadRecentSessions = async () => {
      try {
        const sessions = await fetchSessions();
        setRecentSessions(sessions.slice(0, 3));
      } catch (error) {
        console.error('Failed to load recent sessions:', error);
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

    loadInsights();
    loadRecentSessions();
    // loadRecentProjects();
  }, []);

  const handleSessionClick = (sessionId: string) => {
    navigate('/sessions', {
      state: { selectedSessionId: sessionId },
      replace: true,
    });
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

  if (!insights) {
    return <></>;
  }

  return (
    <div className="bg-background-muted flex flex-col h-full">
      {/* Header container with rounded bottom */}
      <div className="bg-background-default rounded-b-2xl mb-0.5">
        <div className="px-8 pb-12 pt-19 space-y-4">
          <motion.div
            initial={{ opacity: 0, scale: 0.25, x: -5, y: 5, rotate: -20 }}
            animate={{ opacity: 1, scale: 1, x: 0, y: 0, rotate: 0 }}
            transition={{ type: 'spring', stiffness: 300, damping: 25 }}
            className="origin-bottom-left"
          >
            <Goose className="size-8" />
          </motion.div>
          <Greeting />
        </div>
      </div>

      {/* Stats containers - full bleed with 2px gaps */}
      <div className="flex flex-col flex-1 space-y-0.5">
        {/* Top row with three equal columns */}
        <div className="grid grid-cols-3 gap-0.5">
          {/* Total Sessions Card */}
          <Card className="w-full py-6 px-6 border-none rounded-2xl bg-background-default">
            <CardContent className="animate-in fade-in duration-500 flex flex-col justify-end h-full p-0">
              <div className="flex flex-col justify-end">
                <p className="text-4xl font-mono font-light flex items-end">
                  {insights?.totalSessions}
                </p>
                <span className="text-xs text-text-muted">Total sessions</span>
              </div>
            </CardContent>
          </Card>

          {/* Average Duration Card */}
          <Card className="w-full py-6 px-6 border-none rounded-2xl bg-background-default">
            <CardContent className="animate-in fade-in duration-500 flex flex-col justify-end h-full p-0">
              <div className="flex flex-col justify-end">
                <p className="text-4xl font-mono font-light flex items-end">
                  {insights?.avgSessionDuration?.toFixed(1)}m
                </p>
                <span className="text-xs text-text-muted">Avg. chat length</span>
              </div>
            </CardContent>
          </Card>

          {/* Total Tokens Card */}
          <Card className="w-full py-6 px-6 border-none rounded-2xl bg-background-default">
            <CardContent className="animate-in fade-in duration-500 flex flex-col justify-end h-full p-0">
              <div className="flex flex-col justify-end">
                <p className="text-4xl font-mono font-light flex items-end">
                  {insights?.totalTokens ? `${(insights.totalTokens / 1000000).toFixed(2)}M` : ''}
                </p>
                <span className="text-xs text-text-muted">Total tokens</span>
              </div>
            </CardContent>
          </Card>
        </div>

        {/* Projects and Chats row */}
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
            <CardContent className="animate-in fade-in duration-500 p-0">
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
                <AnimatePresence>
                  {recentSessions.length > 0 ? (
                    recentSessions.map((session, index) => (
                      <motion.div
                        key={session.id}
                        className="flex items-center justify-between text-sm py-1 px-2 rounded-md hover:bg-background-muted cursor-pointer transition-colors"
                        onClick={() => handleSessionClick(session.id)}
                        role="button"
                        tabIndex={0}
                        initial={{ opacity: 0, y: 5 }}
                        animate={{ opacity: 1, y: 0 }}
                        transition={{ duration: 0.3, delay: index * 0.1 }}
                        onKeyDown={(e) => {
                          if (e.key === 'Enter' || e.key === ' ') {
                            handleSessionClick(session.id);
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
                      </motion.div>
                    ))
                  ) : (
                    <div className="text-text-muted text-sm py-2">
                      No recent chat sessions found.
                    </div>
                  )}
                </AnimatePresence>
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
