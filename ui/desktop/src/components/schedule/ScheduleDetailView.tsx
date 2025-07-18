import React, { useState, useEffect, useCallback, useMemo } from 'react';
import { Button } from '../ui/button';
import { ScrollArea } from '../ui/scroll-area';
import BackButton from '../ui/BackButton';
import { Card } from '../ui/card';
import { fetchSessionDetails, SessionDetails } from '../../sessions';
import {
  getScheduleSessions,
  runScheduleNow,
  pauseSchedule,
  unpauseSchedule,
  updateSchedule,
  listSchedules,
  killRunningJob,
  inspectRunningJob,
  ScheduledJob,
} from '../../schedule';
import SessionHistoryView from '../sessions/SessionHistoryView';
import { EditScheduleModal } from './EditScheduleModal';
import { toastError, toastSuccess } from '../../toasts';
import { Loader2, Pause, Play, Edit, Square, Eye } from 'lucide-react';
import cronstrue from 'cronstrue';
import { formatToLocalDateWithTimezone } from '../../utils/date';

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

// Memoized ScheduleInfoCard component to prevent unnecessary re-renders of static content
const ScheduleInfoCard = React.memo<{
  scheduleDetails: ScheduledJob;
}>(({ scheduleDetails }) => {
  const readableCron = useMemo(() => {
    try {
      return cronstrue.toString(scheduleDetails.cron);
    } catch (e) {
      console.warn(`Could not parse cron string "${scheduleDetails.cron}":`, e);
      return scheduleDetails.cron;
    }
  }, [scheduleDetails.cron]);

  const formattedLastRun = useMemo(() => {
    return formatToLocalDateWithTimezone(scheduleDetails.last_run);
  }, [scheduleDetails.last_run]);

  const formattedProcessStartTime = useMemo(() => {
    return scheduleDetails.process_start_time
      ? formatToLocalDateWithTimezone(scheduleDetails.process_start_time)
      : null;
  }, [scheduleDetails.process_start_time]);

  return (
    <Card className="p-4 bg-background-card shadow mb-6">
      <div className="space-y-2">
        <div className="flex flex-col md:flex-row md:items-center justify-between">
          <h3 className="text-base font-semibold text-text-prominent">{scheduleDetails.id}</h3>
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
        <p className="text-sm text-text-default">
          <span className="font-semibold">Schedule:</span> {readableCron}
        </p>
        <p className="text-sm text-text-default">
          <span className="font-semibold">Cron Expression:</span> {scheduleDetails.cron}
        </p>
        <p className="text-sm text-text-default">
          <span className="font-semibold">Recipe Source:</span> {scheduleDetails.source}
        </p>
        <p className="text-sm text-text-default">
          <span className="font-semibold">Last Run:</span> {formattedLastRun}
        </p>
        {scheduleDetails.execution_mode && (
          <p className="text-sm text-text-default">
            <span className="font-semibold">Execution Mode:</span>{' '}
            <span
              className={`inline-flex items-center px-2 py-1 rounded-full text-xs font-medium ${
                scheduleDetails.execution_mode === 'foreground'
                  ? 'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300'
                  : 'bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300'
              }`}
            >
              {scheduleDetails.execution_mode === 'foreground' ? '🖥️ Foreground' : '⚡ Background'}
            </span>
          </p>
        )}
        {scheduleDetails.currently_running && scheduleDetails.current_session_id && (
          <p className="text-sm text-text-default">
            <span className="font-semibold">Current Session:</span>{' '}
            {scheduleDetails.current_session_id}
          </p>
        )}
        {scheduleDetails.currently_running && formattedProcessStartTime && (
          <p className="text-sm text-text-default">
            <span className="font-semibold">Process Started:</span> {formattedProcessStartTime}
          </p>
        )}
      </div>
    </Card>
  );
});

ScheduleInfoCard.displayName = 'ScheduleInfoCard';

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
  const [killJobLoading, setKillJobLoading] = useState(false);
  const [inspectJobLoading, setInspectJobLoading] = useState(false);

  // Track if we explicitly killed a job to distinguish from natural completion
  const [jobWasKilled, setJobWasKilled] = useState(false);

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
      const fetchedSessions = await getScheduleSessions(sId, 20);
      setSessions((prevSessions) => {
        // Only update if sessions actually changed to prevent unnecessary re-renders
        if (JSON.stringify(prevSessions) !== JSON.stringify(fetchedSessions)) {
          return fetchedSessions as ScheduleSessionMeta[];
        }
        return prevSessions;
      });
    } catch (err) {
      console.error('Failed to fetch schedule sessions:', err);
      setSessionsError(err instanceof Error ? err.message : 'Failed to fetch schedule sessions');
    } finally {
      setIsLoadingSessions(false);
    }
  }, []);

  const fetchScheduleDetails = useCallback(
    async (sId: string, isRefresh = false) => {
      if (!sId) return;
      if (!isRefresh) setIsLoadingSchedule(true);
      setScheduleError(null);
      try {
        const allSchedules = await listSchedules();
        const schedule = allSchedules.find((s) => s.id === sId);
        if (schedule) {
          setScheduleDetails((prevDetails) => {
            // Only update if schedule details actually changed
            if (!prevDetails || JSON.stringify(prevDetails) !== JSON.stringify(schedule)) {
              // Only reset runNowLoading if we explicitly killed the job
              if (!schedule.currently_running && runNowLoading && jobWasKilled) {
                setRunNowLoading(false);
                setJobWasKilled(false);
              }
              return schedule;
            }
            return prevDetails;
          });
        } else {
          setScheduleError('Schedule not found');
        }
      } catch (err) {
        console.error('Failed to fetch schedule details:', err);
        setScheduleError(err instanceof Error ? err.message : 'Failed to fetch schedule details');
      } finally {
        if (!isRefresh) setIsLoadingSchedule(false);
      }
    },
    [runNowLoading, jobWasKilled]
  );

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
      setJobWasKilled(false); // Reset kill flag when changing schedules
    }
  }, [scheduleId, fetchScheduleSessions, fetchScheduleDetails, selectedSessionDetails]);

  const handleRunNow = async () => {
    if (!scheduleId) return;
    setRunNowLoading(true);
    try {
      const newSessionId = await runScheduleNow(scheduleId);
      if (newSessionId === 'CANCELLED') {
        toastSuccess({
          title: 'Job Cancelled',
          msg: 'The job was cancelled while starting up.',
        });
      } else {
        toastSuccess({
          title: 'Schedule Triggered',
          msg: `Successfully triggered schedule. New session ID: ${newSessionId}`,
        });
      }
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

  const handleKillRunningJob = async () => {
    if (!scheduleId) return;
    setKillJobLoading(true);
    try {
      const result = await killRunningJob(scheduleId);
      toastSuccess({
        title: 'Job Killed',
        msg: result.message,
      });
      // Mark that we explicitly killed this job
      setJobWasKilled(true);
      // Clear the runNowLoading state immediately when job is killed
      setRunNowLoading(false);
      fetchScheduleDetails(scheduleId);
    } catch (err) {
      console.error('Failed to kill running job:', err);
      const errorMsg = err instanceof Error ? err.message : 'Failed to kill running job';
      toastError({ title: 'Kill Job Error', msg: errorMsg });
    } finally {
      setKillJobLoading(false);
    }
  };

  const handleInspectRunningJob = async () => {
    if (!scheduleId) return;
    setInspectJobLoading(true);
    try {
      const result = await inspectRunningJob(scheduleId);
      if (result.sessionId) {
        const duration = result.runningDurationSeconds
          ? `${Math.floor(result.runningDurationSeconds / 60)}m ${result.runningDurationSeconds % 60}s`
          : 'Unknown';
        toastSuccess({
          title: 'Job Inspection',
          msg: `Session: ${result.sessionId}\nRunning for: ${duration}`,
        });
      } else {
        toastSuccess({
          title: 'Job Inspection',
          msg: 'No detailed information available for this job',
        });
      }
    } catch (err) {
      console.error('Failed to inspect running job:', err);
      const errorMsg = err instanceof Error ? err.message : 'Failed to inspect running job';
      toastError({ title: 'Inspect Job Error', msg: errorMsg });
    } finally {
      setInspectJobLoading(false);
    }
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

  // Optimized periodic refresh for schedule details to keep the running status up to date
  useEffect(() => {
    if (!scheduleId) return;

    // Initial fetch
    fetchScheduleDetails(scheduleId);

    // Set up periodic refresh every 8 seconds (longer to reduce flashing)
    const intervalId = setInterval(() => {
      if (
        scheduleId &&
        !selectedSessionDetails &&
        !runNowLoading &&
        !pauseUnpauseLoading &&
        !killJobLoading &&
        !inspectJobLoading &&
        !isEditSubmitting
      ) {
        fetchScheduleDetails(scheduleId, true); // Pass true to indicate this is a refresh
      }
    }, 8000);

    // Clean up on unmount or when scheduleId changes
    return () => {
      clearInterval(intervalId);
    };
  }, [
    scheduleId,
    fetchScheduleDetails,
    selectedSessionDetails,
    runNowLoading,
    pauseUnpauseLoading,
    killJobLoading,
    inspectJobLoading,
    isEditSubmitting,
  ]);

  // Monitor schedule state changes and reset loading states appropriately
  useEffect(() => {
    if (scheduleDetails) {
      // Only reset runNowLoading if we explicitly killed the job
      // This prevents interfering with natural job completion
      if (!scheduleDetails.currently_running && runNowLoading && jobWasKilled) {
        setRunNowLoading(false);
        setJobWasKilled(false); // Reset the flag
      }
    }
  }, [scheduleDetails, runNowLoading, jobWasKilled]);

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
        onRetry={() => loadAndShowSessionDetails(selectedSessionDetails.session_id)}
        showActionButtons={true}
      />
    );
  }

  if (!scheduleId) {
    return (
      <div className="h-screen w-full flex flex-col items-center justify-center bg-white dark:bg-gray-900 text-text-default p-8">
        <BackButton onClick={onNavigateBack} />
        <h1 className="text-2xl font-medium text-text-prominent mt-4">Schedule Not Found</h1>
        <p className="text-text-subtle mt-2">
          No schedule ID was provided. Please return to the schedules list and select a schedule.
        </p>
      </div>
    );
  }

  return (
    <div className="h-screen w-full flex flex-col bg-background-default text-text-default">
      <div className="px-8 pt-6 pb-4 border-b border-border-subtle flex-shrink-0">
        <BackButton onClick={onNavigateBack} />
        <h1 className="text-4xl font-light mt-1 mb-1 pt-8">Schedule Details</h1>
        <p className="text-sm text-text-muted mb-1">Viewing Schedule ID: {scheduleId}</p>
      </div>

      <ScrollArea className="flex-grow">
        <div className="p-8 space-y-6">
          <section>
            <h2 className="text-xl font-semibold text-text-prominent mb-3">Schedule Information</h2>
            {isLoadingSchedule && (
              <div className="flex items-center text-text-subtle">
                <Loader2 className="mr-2 h-4 w-4 animate-spin" /> Loading schedule details...
              </div>
            )}
            {scheduleError && (
              <p className="text-text-error text-sm p-3 bg-background-error border border-border-error rounded-md">
                Error: {scheduleError}
              </p>
            )}
            {!isLoadingSchedule && !scheduleError && scheduleDetails && (
              <ScheduleInfoCard scheduleDetails={scheduleDetails} />
            )}
          </section>

          <section>
            <h2 className="text-xl font-semibold text-text-prominent mb-3">Actions</h2>
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

              {scheduleDetails && scheduleDetails.currently_running && (
                <>
                  <Button
                    onClick={handleInspectRunningJob}
                    variant="outline"
                    className="w-full md:w-auto flex items-center gap-2 text-blue-600 dark:text-blue-400 border-blue-300 dark:border-blue-600 hover:bg-blue-50 dark:hover:bg-blue-900/20"
                    disabled={inspectJobLoading}
                  >
                    <Eye className="w-4 h-4" />
                    {inspectJobLoading ? 'Inspecting...' : 'Inspect Running Job'}
                  </Button>
                  <Button
                    onClick={handleKillRunningJob}
                    variant="outline"
                    className="w-full md:w-auto flex items-center gap-2 text-red-600 dark:text-red-400 border-red-300 dark:border-red-600 hover:bg-red-50 dark:hover:bg-red-900/20"
                    disabled={killJobLoading}
                  >
                    <Square className="w-4 h-4" />
                    {killJobLoading ? 'Killing...' : 'Kill Running Job'}
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
                This schedule is paused and will not run automatically. Use "Run Schedule Now" to
                trigger it manually or unpause to resume automatic execution.
              </p>
            )}
          </section>

          <section>
            <h2 className="text-xl font-semibold text-text-prominent mb-4">
              Recent Sessions for this Schedule
            </h2>
            {isLoadingSessions && <p className="text-text-subtle">Loading sessions...</p>}
            {sessionsError && (
              <p className="text-text-error text-sm p-3 bg-background-error border border-border-error rounded-md">
                Error: {sessionsError}
              </p>
            )}
            {!isLoadingSessions && !sessionsError && sessions.length === 0 && (
              <p className="text-text-subtle text-center py-4">
                No sessions found for this schedule.
              </p>
            )}

            {!isLoadingSessions && sessions.length > 0 && (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {sessions.map((session) => (
                  <Card
                    key={session.id}
                    className="p-4 bg-background-card shadow cursor-pointer hover:shadow-lg transition-shadow duration-200"
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
                      className="text-sm font-semibold text-text-prominent truncate"
                      title={session.name || session.id}
                    >
                      {session.name || `Session ID: ${session.id}`}{' '}
                    </h3>
                    <p className="text-xs text-text-subtle mt-1">
                      Created:{' '}
                      {session.createdAt ? new Date(session.createdAt).toLocaleString() : 'N/A'}
                    </p>
                    {session.messageCount !== undefined && (
                      <p className="text-xs text-text-subtle mt-1">
                        Messages: {session.messageCount}
                      </p>
                    )}
                    {session.workingDir && (
                      <p
                        className="text-xs text-text-subtle mt-1 truncate"
                        title={session.workingDir}
                      >
                        Dir: {session.workingDir}
                      </p>
                    )}
                    {session.accumulatedTotalTokens !== undefined &&
                      session.accumulatedTotalTokens !== null && (
                        <p className="text-xs text-text-subtle mt-1">
                          Tokens: {session.accumulatedTotalTokens}
                        </p>
                      )}
                    <p className="text-xs text-text-muted mt-1">
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
