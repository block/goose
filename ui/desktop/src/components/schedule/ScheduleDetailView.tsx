import React, { useState, useEffect, useCallback } from 'react';
import { Button } from '../ui/button';
import { ScrollArea } from '../ui/scroll-area';
import BackButton from '../ui/BackButton';
import { Card } from '../ui/card';
import MoreMenuLayout from '../more_menu/MoreMenuLayout';

// Updated to match SessionDisplayInfo from Rust backend (camelCase)
interface SessionMeta {
  id: string; // Was session_id, now 'id' (from session_name)
  name: string; // New: from metadata.description
  createdAt: string; // Was created_at, now 'createdAt' (from session_name, ISO 8601)
  workingDir?: string;
  scheduleId?: string | null; // This is the ID of the parent schedule
  messageCount?: number;
  totalTokens?: number | null;
  inputTokens?: number | null;
  outputTokens?: number | null;
  accumulatedTotalTokens?: number | null;
  accumulatedInputTokens?: number | null;
  accumulatedOutputTokens?: number | null;
}

interface ScheduleDetailViewProps {
  scheduleId: string | null; // This is the ID of the schedule being viewed
  onNavigateBack: () => void;
  onNavigateToSession: (sessionId: string) => void; // Parameter is the session's unique 'id'
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

  const fetchSessions = useCallback(async (sId: string) => {
    // sId is scheduleId here
    if (!sId) return;
    setIsLoadingSessions(true);
    setSessionsError(null);
    try {
      // window.schedule.sessions expects the *scheduleId*
      const fetchedSessions = (await window.schedule.sessions(sId, 20)) as SessionMeta[];
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
      setTimeout(() => {
        if (scheduleId) fetchSessions(scheduleId);
      }, 1000);
    } catch (err) {
      console.error('Failed to run schedule now:', err);
      setRunNowError(err instanceof Error ? err.message : 'Failed to trigger schedule');
    } finally {
      setRunNowLoading(false);
    }
  };

  const handleSessionClick = (sessionIdFromCard: string) => {
    // This is session.id
    onNavigateToSession(sessionIdFromCard);
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
        <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
          Viewing Schedule ID: {scheduleId}
        </p>
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
                    key={session.id} // Use session.id for key
                    className="p-4 bg-white dark:bg-gray-800 shadow cursor-pointer hover:shadow-lg transition-shadow duration-200"
                    onClick={() => handleSessionClick(session.id)} // Use session.id
                    role="button"
                    tabIndex={0}
                    onKeyPress={(e) => {
                      if (e.key === 'Enter' || e.key === ' ') {
                        handleSessionClick(session.id); // Use session.id
                      }
                    }}
                  >
                    <h3
                      className="text-sm font-semibold text-gray-900 dark:text-white truncate"
                      title={session.name || session.id} // Show full name (description) on hover, fallback to id
                    >
                      {session.name || `Session ID: ${session.id}`}{' '}
                      {/* Display name (description) or ID */}
                    </h3>
                    <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                      Created:{' '}
                      {session.createdAt ? new Date(session.createdAt).toLocaleString() : 'N/A'}
                    </p>
                    {session.messageCount !== undefined && (
                      <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                        Messages: {session.messageCount}
                      </p>
                    )}
                    {session.workingDir && (
                      <p
                        className="text-xs text-gray-500 dark:text-gray-400 mt-1 truncate"
                        title={session.workingDir}
                      >
                        Dir: {session.workingDir}
                      </p>
                    )}
                    {session.accumulatedTotalTokens !== undefined &&
                      session.accumulatedTotalTokens !== null && (
                        <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                          Tokens: {session.accumulatedTotalTokens}
                        </p>
                      )}
                    <p className="text-xs text-gray-600 dark:text-gray-500 mt-1">
                      ID: <span className="font-mono">{session.id}</span>
                    </p>
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
