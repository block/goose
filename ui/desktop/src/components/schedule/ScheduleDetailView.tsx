import React, { useState, useEffect, useCallback } from 'react';
import { Button } from '../ui/button';
import { ScrollArea } from '../ui/scroll-area';
import BackButton from '../ui/BackButton';
import { Card } from '../ui/card';
import MoreMenuLayout from '../more_menu/MoreMenuLayout';
import { fetchSessionDetails, SessionDetails } from '../../sessions';
import { getScheduleSessions, runScheduleNow, pauseSchedule, unpauseSchedule, updateSchedule, listSchedules, ScheduledJob } from '../../schedule';
import SessionHistoryView from '../sessions/SessionHistoryView';
import { EditScheduleModal } from './EditScheduleModal';
import { toastError, toastSuccess } from '../../toasts';
import { Loader2, Pause, Play, Edit } from 'lucide-react';
import cronstrue from 'cronstrue';

interface ScheduleSessionMeta {
  id: string;
  name: string;
  createdAt: string;
  workingDir?: string;
  scheduleId?: string | null;
  messageCount?: number;
  totalTokens?: number | null;
  inputTokens?: number | null;
  outputTokens?: number | null;
  accumulatedTotalTokens?: number | null;
  accumulatedInputTokens?: number | null;
  accumulatedOutputTokens?: number | null;
}

interface ScheduleDetailViewProps {
  scheduleId: string | null;
  onNavigateBack: () => void;
}

const ScheduleDetailView: React.FC<ScheduleDetailViewProps> = ({ scheduleId, onNavigateBack }) => {
  const [sessions, setSessions] = useState<ScheduleSessionMeta[]>([]);
  const [isLoadingSessions, setIsLoadingSessions] = useState(false);
  const [sessionsError, setSessionsError] = useState<string | null>(null);
  const [runNowLoading, setRunNowLoading] = useState(false);
  const [scheduleDetails, setScheduleDetails] = useState<ScheduledJob | null>(null);
  const [isLoadingSchedule, setIsLoadingSchedule] = useState(false);
  const [scheduleError, setScheduleError] = useState<string | null>(null);

  // Individual loading states for each action to prevent double-clicks
  const [pauseUnpauseLoading, setPauseUnpauseLoading] = useState(false);

  const [selectedSessionDetails, setSelectedSessionDetails] = useState<SessionDetails | null>(null);
  const [isLoadingSessionDetails, setIsLoadingSessionDetails] = useState(false);
  const [sessionDetailsError, setSessionDetailsError] = useState<string | null>(null);
  const [isEditModalOpen, setIsEditModalOpen] = useState(false);
  const [editApiError, setEditApiError] = useState<string | null>(null);
  const [isEditSubmitting, setIsEditSubmitting] = useState(false);

  const fetchScheduleSessions = useCallback(async (sId: string) => {
    if (!sId) return;
    setIsLoadingSessions(true);
    setSessionsError(null);
    try {
      const fetchedSessions = await getScheduleSessions(sId, 20); // MODIFIED
      // Assuming ScheduleSession from ../../schedule can be cast or mapped to ScheduleSessionMeta
      // You may need to transform/map fields if they differ significantly
      setSessions(fetchedSessions as ScheduleSessionMeta[]);
    } catch (err) {
      console.error('Failed to fetch schedule sessions:', err);
      setSessionsError(err instanceof Error ? err.message : 'Failed to fetch schedule sessions');
    } finally {
      setIsLoadingSessions(false);
    }
  }, []);

  const fetchScheduleDetails = useCallback(async (sId: string) => {
    if (!sId) return;
    setIsLoadingSchedule(true);
    setScheduleError(null);
    try {
      const allSchedules = await listSchedules();
      const schedule = allSchedules.find((s) => s.id === sId);
      if (schedule) {
        setScheduleDetails(schedule);
      } else {
        setScheduleError('Schedule not found');
      }
    } catch (err) {
      console.error('Failed to fetch schedule details:', err);
      setScheduleError(err instanceof Error ? err.message : 'Failed to fetch schedule details');
    } finally {
      setIsLoadingSchedule(false);
    }
  }, []);

  const getReadableCron = (cronString: string) => {
    try {
      return cronstrue.toString(cronString);
    } catch (e) {
      console.warn(`Could not parse cron string "${cronString}":`, e);
      return cronString;
    }
  };

  useEffect(() => {
    if (scheduleId && !selectedSessionDetails) {
      fetchScheduleSessions(scheduleId);
      fetchScheduleDetails(scheduleId);
    } else if (!scheduleId) {
      setSessions([]);
      setSessionsError(null);
      setRunNowLoading(false);
      setSelectedSessionDetails(null);
      setScheduleDetails(null);
      setScheduleError(null);
    }
  }, [scheduleId, fetchScheduleSessions, fetchScheduleDetails, selectedSessionDetails]);

  const handleRunNow = async () => {
    if (!scheduleId) return;
    setRunNowLoading(true);
    try {
      const newSessionId = await runScheduleNow(scheduleId); // MODIFIED
      toastSuccess({
        title: 'Schedule Triggered',
        msg: `Successfully triggered schedule. New session ID: ${newSessionId}`,
      });
      setTimeout(() => {
        if (scheduleId) {
          fetchScheduleSessions(scheduleId);
          fetchScheduleDetails(scheduleId);
        }
      }, 1000);
    } catch (err) {
      console.error('Failed to run schedule now:', err);
      const errorMsg = err instanceof Error ? err.message : 'Failed to trigger schedule';
      toastError({ title: 'Run Schedule Error', msg: errorMsg });
    } finally {
      setRunNowLoading(false);
    }
  };

  const handlePauseSchedule = async () => {
    if (!scheduleId) return;
    setPauseUnpauseLoading(true);
    try {
      await pauseSchedule(scheduleId);
      toastSuccess({
        title: 'Schedule Paused',
        msg: `Successfully paused schedule "${scheduleId}"`,
      });
      fetchScheduleDetails(scheduleId);
    } catch (err) {
      console.error('Failed to pause schedule:', err);
      const errorMsg = err instanceof Error ? err.message : 'Failed to pause schedule';
      toastError({ title: 'Pause Schedule Error', msg: errorMsg });
    } finally {
      setPauseUnpauseLoading(false);
    }
  };

  const handleUnpauseSchedule = async () => {
    if (!scheduleId) return;
    setPauseUnpauseLoading(true);
    try {
      await unpauseSchedule(scheduleId);
      toastSuccess({
        title: 'Schedule Unpaused',
        msg: `Successfully unpaused schedule "${scheduleId}"`,
      });
      fetchScheduleDetails(scheduleId);
    } catch (err) {
      console.error('Failed to unpause schedule:', err);
      const errorMsg = err instanceof Error ? err.message : 'Failed to unpause schedule';
      toastError({ title: 'Unpause Schedule Error', msg: errorMsg });
    } finally {
      setPauseUnpauseLoading(false);
    }
  };

  const handleOpenEditModal = () => {
    setEditApiError(null);
    setIsEditModalOpen(true);
  };

  const handleCloseEditModal = () => {
    setIsEditModalOpen(false);
    setEditApiError(null);
  };

  const handleEditScheduleSubmit = async (cron: string) => {
    if (!scheduleId) return;

    setIsEditSubmitting(true);
    setEditApiError(null);
    try {
      await updateSchedule(scheduleId, cron);
      toastSuccess({
        title: 'Schedule Updated',
        msg: `Successfully updated schedule "${scheduleId}"`,
      });
      fetchScheduleDetails(scheduleId);
      setIsEditModalOpen(false);
    } catch (err) {
      console.error('Failed to update schedule:', err);
      const errorMsg = err instanceof Error ? err.message : 'Failed to update schedule';
      setEditApiError(errorMsg);
      toastError({ title: 'Update Schedule Error', msg: errorMsg });
    } finally {
      setIsEditSubmitting(false);
    }
  };

  const handleOpenEditModal = () => {
    setEditApiError(null);
    setIsEditModalOpen(true);
  };

  const handleCloseEditModal = () => {
    setIsEditModalOpen(false);
    setEditApiError(null);
  };

  const handleEditScheduleSubmit = async (cron: string) => {
    if (!scheduleId) return;

    setIsEditSubmitting(true);
    setEditApiError(null);
    try {
      await updateSchedule(scheduleId, cron);
      toastSuccess({
        title: 'Schedule Updated',
        msg: `Successfully updated schedule "${scheduleId}"`,
      });
      fetchScheduleDetails(scheduleId);
      setIsEditModalOpen(false);
    } catch (err) {
      console.error('Failed to update schedule:', err);
      const errorMsg = err instanceof Error ? err.message : 'Failed to update schedule';
      setEditApiError(errorMsg);
      toastError({ title: 'Update Schedule Error', msg: errorMsg });
    } finally {
      setIsEditSubmitting(false);
    }
  };

  // Add a periodic refresh for schedule details to keep the running status up to date
  useEffect(() => {
    if (!scheduleId) return;

    // Initial fetch
    fetchScheduleDetails(scheduleId);

    // Set up periodic refresh every 5 seconds
    const intervalId = setInterval(() => {
      if (scheduleId) {
        fetchScheduleDetails(scheduleId);
      }
    }, 5000);

    // Clean up on unmount or when scheduleId changes
    return () => {
      clearInterval(intervalId);
    };
  }, [scheduleId, fetchScheduleDetails]);

  const loadAndShowSessionDetails = async (sessionId: string) => {
    setIsLoadingSessionDetails(true);
    setSessionDetailsError(null);
    setSelectedSessionDetails(null);
    try {
      const details = await fetchSessionDetails(sessionId);
      setSelectedSessionDetails(details);
    } catch (err) {
      console.error(`Failed to load session details for ${sessionId}:`, err);
      const errorMsg = err instanceof Error ? err.message : 'Failed to load session details.';
      setSessionDetailsError(errorMsg);
      toastError({
        title: 'Failed to load session details',
        msg: errorMsg,
      });
    } finally {
      setIsLoadingSessionDetails(false);
    }
  };

  const handleSessionCardClick = (sessionIdFromCard: string) => {
    loadAndShowSessionDetails(sessionIdFromCard);
  };

  const handleResumeViewedSession = () => {
    if (selectedSessionDetails) {
      const { session_id, metadata } = selectedSessionDetails;
      if (metadata.working_dir) {
        console.log(
          `Resuming session ID ${session_id} in new chat window. Dir: ${metadata.working_dir}`
        );
        window.electron.createChatWindow(undefined, metadata.working_dir, undefined, session_id);
      } else {
        console.error('Cannot resume session: working directory is missing.');
        toastError({ title: 'Cannot Resume Session', msg: 'Working directory is missing.' });
      }
    }
  };

  if (selectedSessionDetails) {
    return (
      <SessionHistoryView
        session={selectedSessionDetails}
        isLoading={isLoadingSessionDetails}
        error={sessionDetailsError}
        onBack={() => {
          setSelectedSessionDetails(null);
          setSessionDetailsError(null);
        }}
        onResume={handleResumeViewedSession}
        onRetry={() => loadAndShowSessionDetails(selectedSessionDetails.session_id)}
        showActionButtons={true}
      />
    );
  }

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
            <h2 className="text-xl font-semibold text-gray-900 dark:text-white mb-3">
              Schedule Information
            </h2>
            {isLoadingSchedule && (
              <div className="flex items-center text-gray-500 dark:text-gray-400">
                <Loader2 className="mr-2 h-4 w-4 animate-spin" /> Loading schedule details...
              </div>
            )}
            {scheduleError && (
              <p className="text-red-500 dark:text-red-400 text-sm p-3 bg-red-100 dark:bg-red-900/30 border border-red-500 dark:border-red-700 rounded-md">
                Error: {scheduleError}
              </p>
            )}
            {!isLoadingSchedule && !scheduleError && scheduleDetails && (
              <Card className="p-4 bg-white dark:bg-gray-800 shadow mb-6">
                <div className="space-y-2">
                  <div className="flex flex-col md:flex-row md:items-center justify-between">
                    <h3 className="text-base font-semibold text-gray-900 dark:text-white">
                      {scheduleDetails.id}
                    </h3>
                    <div className="mt-2 md:mt-0 flex items-center gap-2">
                      {scheduleDetails.currently_running && (
                        <div className="text-sm text-green-500 dark:text-green-400 font-semibold flex items-center">
                          <span className="inline-block w-2 h-2 bg-green-500 dark:bg-green-400 rounded-full mr-1 animate-pulse"></span>
                          Currently Running
                        </div>
                      )}
                      {scheduleDetails.paused && (
                        <div className="text-sm text-orange-500 dark:text-orange-400 font-semibold flex items-center">
                          <Pause className="w-3 h-3 mr-1" />
                          Paused
                        </div>
                      )}
                    </div>
                  </div>
                  <p className="text-sm text-gray-600 dark:text-gray-300">
                    <span className="font-semibold">Schedule:</span>{' '}
                    {getReadableCron(scheduleDetails.cron)}
                  </p>
                  <p className="text-sm text-gray-600 dark:text-gray-300">
                    <span className="font-semibold">Cron Expression:</span> {scheduleDetails.cron}
                  </p>
                  <p className="text-sm text-gray-600 dark:text-gray-300">
                    <span className="font-semibold">Recipe Source:</span> {scheduleDetails.source}
                  </p>
                  <p className="text-sm text-gray-600 dark:text-gray-300">
                    <span className="font-semibold">Last Run:</span>{' '}
                    {scheduleDetails.last_run
                      ? new Date(scheduleDetails.last_run).toLocaleString()
                      : 'Never'}
                  </p>
                </div>
              </Card>
            )}
          </section>

          <section>
            <h2 className="text-xl font-semibold text-gray-900 dark:text-white mb-3">Actions</h2>
            <div className="flex flex-col md:flex-row gap-2">
              <Button
                onClick={handleRunNow}
                disabled={runNowLoading || scheduleDetails?.currently_running === true}
                className="w-full md:w-auto"
              >
                {runNowLoading ? 'Triggering...' : 'Run Schedule Now'}
              </Button>
              
              {scheduleDetails && !scheduleDetails.currently_running && (
                <>
                  <Button
                    onClick={handleOpenEditModal}
                    variant="outline"
                    className="w-full md:w-auto flex items-center gap-2 text-blue-600 dark:text-blue-400 border-blue-300 dark:border-blue-600 hover:bg-blue-50 dark:hover:bg-blue-900/20"
                    disabled={runNowLoading || pauseUnpauseLoading || isEditSubmitting}
                  >
                    <Edit className="w-4 h-4" />
                    Edit Schedule
                  </Button>
                  <Button
                    onClick={scheduleDetails.paused ? handleUnpauseSchedule : handlePauseSchedule}
                    variant="outline"
                    className={`w-full md:w-auto flex items-center gap-2 ${
                      scheduleDetails.paused
                        ? 'text-green-600 dark:text-green-400 border-green-300 dark:border-green-600 hover:bg-green-50 dark:hover:bg-green-900/20'
                        : 'text-orange-600 dark:text-orange-400 border-orange-300 dark:border-orange-600 hover:bg-orange-50 dark:hover:bg-orange-900/20'
                    }`}
                    disabled={runNowLoading || pauseUnpauseLoading || isEditSubmitting}
                  >
                    {scheduleDetails.paused ? (
                      <>
                        <Play className="w-4 h-4" />
                        {pauseUnpauseLoading ? 'Unpausing...' : 'Unpause Schedule'}
                      </>
                    ) : (
                      <>
                        <Pause className="w-4 h-4" />
                        {pauseUnpauseLoading ? 'Pausing...' : 'Pause Schedule'}
                      </>
                    )}
                  </Button>
                </>
              )}
            </div>
            
            {scheduleDetails?.currently_running && (
              <p className="text-sm text-amber-600 dark:text-amber-400 mt-2">
                Cannot trigger or modify a schedule while it's already running.
              </p>
            )}
            
            {scheduleDetails?.paused && (
              <p className="text-sm text-orange-600 dark:text-orange-400 mt-2">
                This schedule is paused and will not run automatically. Use "Run Schedule Now" to trigger it manually or unpause to resume automatic execution.
              </p>
            )}
          </section>

          <section>
            <h2 className="text-xl font-semibold text-gray-900 dark:text-white mb-4">
              Recent Sessions for this Schedule
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
                    key={session.id}
                    className="p-4 bg-white dark:bg-gray-800 shadow cursor-pointer hover:shadow-lg transition-shadow duration-200"
                    onClick={() => handleSessionCardClick(session.id)}
                    role="button"
                    tabIndex={0}
                    onKeyPress={(e) => {
                      if (e.key === 'Enter' || e.key === ' ') {
                        handleSessionCardClick(session.id);
                      }
                    }}
                  >
                    <h3
                      className="text-sm font-semibold text-gray-900 dark:text-white truncate"
                      title={session.name || session.id}
                    >
                      {session.name || `Session ID: ${session.id}`}{' '}
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
      <EditScheduleModal
        isOpen={isEditModalOpen}
        onClose={handleCloseEditModal}
        onSubmit={handleEditScheduleSubmit}
        schedule={scheduleDetails}
        isLoadingExternally={isEditSubmitting}
        apiErrorExternally={editApiError}
      />
    </div>
  );
};

export default ScheduleDetailView;
