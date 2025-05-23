import React, { useState, useEffect, useCallback } from 'react';
import { Button } from '../ui/button';
import { ScrollArea } from '../ui/scroll-area';
import BackButton from '../ui/BackButton';
import { Card } from '../ui/card';
import MoreMenuLayout from '../more_menu/MoreMenuLayout';

// Interface based on Rust's SessionMetadata and observed frontend data
interface SessionMeta {
  session_id: string; // From current frontend usage, likely added by backend/main.ts
  created_at: string; // From current frontend usage, likely added by backend/main.ts
  working_dir?: string; // Will be string representation of PathBuf
  description?: string;
  schedule_id?: string | null; // Matches Option<String>
  message_count?: number; // Matches usize
  total_tokens?: number | null;
  input_tokens?: number | null;
  output_tokens?: number | null;
  accumulated_total_tokens?: number | null;
  accumulated_input_tokens?: number | null;
  accumulated_output_tokens?: number | null;
}

interface ScheduleDetailViewProps {
  scheduleId: string | null;
  onNavigateBack: () => void;
  onNavigateToSession: (sessionId: string) => void;
}

const ScheduleDetailView: React.FC<ScheduleDetailViewProps> = ({
  scheduleId,
  onNavigateBack,
  onNavigateToSession,
}) => {
  const [sessions, setSessions] = useState<SessionMeta[]>([]);
  const [isLoadingSessions, setIsLoadingSessions] = useState(false);
  const [sessionsError, setSessionsError] = useState<string | null>(null);
  const [runNowLoading, setRunNowLoading] = useState(false);
  const [runNowError, setRunNowError] = useState<string | null>(null);
  const [runNowSuccessMessage, setRunNowSuccessMessage] = useState<string | null>(null);

  const fetchSessions = useCallback(async (id: string) => {
    if (!id) return;
    setIsLoadingSessions(true);
    setSessionsError(null);
    try {
      const fetchedSessions = (await window.schedule.sessions(id, 20)) as SessionMeta[];
      setSessions(fetchedSessions);
    } catch (err) {
      console.error('Failed to fetch sessions:', err);
      setSessionsError(err instanceof Error ? err.message : 'Failed to fetch sessions');
    } finally {
      setIsLoadingSessions(false);
    }
  }, []);

  useEffect(() => {
    if (scheduleId) {
      fetchSessions(scheduleId);
      setRunNowSuccessMessage(null);
      setRunNowError(null);
    } else {
      setSessions([]);
      setSessionsError(null);
      setRunNowLoading(false);
      setRunNowError(null);
      setRunNowSuccessMessage(null);
    }
  }, [scheduleId, fetchSessions]);

  const handleRunNow = async () => {
    if (!scheduleId) return;
    setRunNowLoading(true);
    setRunNowError(null);
    setRunNowSuccessMessage(null);
    try {
      const newSessionId = await window.schedule.runNow(scheduleId);
      setRunNowSuccessMessage(`Schedule triggered successfully. New session ID: ${newSessionId}`);
      setTimeout(() => fetchSessions(scheduleId), 1000);
    } catch (err) {
      console.error('Failed to run schedule now:', err);
      setRunNowError(err instanceof Error ? err.message : 'Failed to trigger schedule');
    } finally {
      setRunNowLoading(false);
    }
  };

  const handleSessionClick = (sessionId: string) => {
    onNavigateToSession(sessionId);
  };

  if (!scheduleId) {
    return (
      <div className="h-screen w-full flex flex-col items-center justify-center bg-app text-textStandard p-8">
        <MoreMenuLayout showMenu={false} />
        <BackButton onClick={onNavigateBack} />
        <h1 className="text-2xl font-medium text-gray-900 dark:text-white mt-4">
          Schedule Not Found
        </h1>
        <p className="text-gray-600 dark:text-gray-400 mt-2">
          No schedule ID was provided. Please return to the schedules list and select a schedule.
        </p>
      </div>
    );
  }

  return (
    <div className="h-screen w-full flex flex-col bg-app text-textStandard">
      <MoreMenuLayout showMenu={false} />
      <div className="px-8 pt-6 pb-4 border-b border-borderSubtle flex-shrink-0">
        <BackButton onClick={onNavigateBack} />
        <h1 className="text-3xl font-medium text-gray-900 dark:text-white mt-1">
          Schedule Details
        </h1>
        <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">ID: {scheduleId}</p>
      </div>

      <ScrollArea className="flex-grow">
        <div className="p-8 space-y-6">
          <section>
            <h2 className="text-xl font-semibold text-gray-900 dark:text-white mb-3">Actions</h2>
            <Button onClick={handleRunNow} disabled={runNowLoading} className="w-full md:w-auto">
              {runNowLoading ? 'Triggering...' : 'Run Schedule Now'}
            </Button>
            {runNowError && (
              <p className="mt-2 text-red-500 dark:text-red-400 text-sm p-3 bg-red-100 dark:bg-red-900/30 border border-red-500 dark:border-red-700 rounded-md">
                Error: {runNowError}
              </p>
            )}
            {runNowSuccessMessage && (
              <p className="mt-2 text-green-600 dark:text-green-400 text-sm p-3 bg-green-100 dark:bg-green-900/30 border border-green-500 dark:border-green-700 rounded-md">
                {runNowSuccessMessage}
              </p>
            )}
          </section>

          <section>
            <h2 className="text-xl font-semibold text-gray-900 dark:text-white mb-4">
              Recent Sessions
            </h2>
            {isLoadingSessions && (
              <p className="text-gray-500 dark:text-gray-400">Loading sessions...</p>
            )}
            {sessionsError && (
              <p className="text-red-500 dark:text-red-400 text-sm p-3 bg-red-100 dark:bg-red-900/30 border border-red-500 dark:border-red-700 rounded-md">
                Error: {sessionsError}
              </p>
            )}
            {!isLoadingSessions && !sessionsError && sessions.length === 0 && (
              <p className="text-gray-500 dark:text-gray-400 text-center py-4">
                No sessions found for this schedule.
              </p>
            )}

            {!isLoadingSessions && sessions.length > 0 && (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {sessions.map((session) => (
                  <Card
                    key={session.session_id}
                    className="p-4 bg-white dark:bg-gray-800 shadow cursor-pointer hover:shadow-lg transition-shadow duration-200"
                    onClick={() => handleSessionClick(session.session_id)}
                    role="button"
                    tabIndex={0}
                    onKeyPress={(e) => {
                      if (e.key === 'Enter' || e.key === ' ') {
                        handleSessionClick(session.session_id);
                      }
                    }}
                  >
                    <h3
                      className="text-sm font-semibold text-gray-900 dark:text-white truncate"
                      title={session.session_id}
                    >
                      ID: {session.session_id}
                    </h3>
                    <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                      Created: {new Date(session.created_at).toLocaleString()}
                    </p>
                    {session.description && (
                      <p
                        className="text-xs text-gray-500 dark:text-gray-400 mt-1 truncate"
                        title={session.description}
                      >
                        Desc: {session.description}
                      </p>
                    )}
                    {session.message_count !== undefined && (
                      <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                        Messages: {session.message_count}
                      </p>
                    )}
                    {session.working_dir && (
                      <p
                        className="text-xs text-gray-500 dark:text-gray-400 mt-1 truncate"
                        title={session.working_dir}
                      >
                        Dir: {session.working_dir}
                      </p>
                    )}
                    {session.accumulated_total_tokens !== undefined &&
                      session.accumulated_total_tokens !== null && (
                        <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                          Tokens: {session.accumulated_total_tokens}
                        </p>
                      )}
                  </Card>
                ))}
              </div>
            )}
          </section>
        </div>
      </ScrollArea>
    </div>
  );
};

export default ScheduleDetailView;
