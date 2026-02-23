import React, { useState, useEffect } from 'react';
import { Button } from '../ui/button';
import { ScrollArea } from '../ui/scroll-area';
import BackButton from '../ui/BackButton';
import { Card } from '../ui/card';
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
import { ScheduleModal, NewSchedulePayload } from './ScheduleModal';
import { toastError, toastSuccess } from '../../toasts';
import { Loader2, Pause, Play, Edit, Square, Eye } from 'lucide-react';
import cronstrue from 'cronstrue';
import { formatToLocalDateWithTimezone } from '../../utils/date';
import { getSession, Session } from '../../api';
import { trackScheduleRunNow, getErrorType } from '../../utils/analytics';
import { errorMessage } from '../../utils/conversionUtils';
import { t } from '../../i18n';

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

  const [scheduleDetails, setScheduleDetails] = useState<ScheduledJob | null>(null);
  const [isLoadingSchedule, setIsLoadingSchedule] = useState(false);
  const [scheduleError, setScheduleError] = useState<string | null>(null);

  const [isActionLoading, setIsActionLoading] = useState(false);

  const [selectedSession, setSelectedSession] = useState<Session | null>(null);
  const [isLoadingSession, setIsLoadingSession] = useState(false);
  const [sessionError, setSessionError] = useState<string | null>(null);

  const [isModalOpen, setIsModalOpen] = useState(false);

  const fetchSessions = async (sId: string) => {
    setIsLoadingSessions(true);
    setSessionsError(null);
    try {
      const data = await getScheduleSessions(sId, 20);
      setSessions(data);
    } catch (err) {
      setSessionsError(errorMessage(err, 'Failed to fetch sessions'));
    } finally {
      setIsLoadingSessions(false);
    }
  };

  const fetchSchedule = async (sId: string) => {
    setIsLoadingSchedule(true);
    setScheduleError(null);
    try {
      const allSchedules = await listSchedules();
      const schedule = allSchedules.find((s) => s.id === sId);
      if (schedule) {
        setScheduleDetails(schedule);
      } else {
        setScheduleError(t('schedule.not_found', 'Schedule not found'));
      }
    } catch (err) {
      setScheduleError(errorMessage(err, 'Failed to fetch schedule'));
    } finally {
      setIsLoadingSchedule(false);
    }
  };

  useEffect(() => {
    if (scheduleId && !selectedSession) {
      fetchSessions(scheduleId);
      fetchSchedule(scheduleId);
    }
  }, [scheduleId, selectedSession]);

  const handleRunNow = async () => {
    if (!scheduleId) return;
    setIsActionLoading(true);
    try {
      const newSessionId = await runScheduleNow(scheduleId);
      trackScheduleRunNow(true);
      if (newSessionId === 'CANCELLED') {
        toastSuccess({
          title: t('schedule.job_cancelled', 'Job Cancelled'),
          msg: t('schedule.job_cancelled_msg', 'The job was cancelled while starting up.'),
        });
      } else {
        toastSuccess({
          title: t('schedule.triggered', 'Schedule Triggered'),
          msg: t('schedule.new_session', 'New session: {id}', { id: newSessionId }),
        });
      }
      await fetchSessions(scheduleId);
      await fetchSchedule(scheduleId);
    } catch (err) {
      const errorMsg = errorMessage(err, 'Failed to trigger schedule');
      trackScheduleRunNow(false, getErrorType(err));
      toastError({
        title: t('schedule.run_error', 'Run Schedule Error'),
        msg: errorMsg,
      });
    } finally {
      setIsActionLoading(false);
    }
  };

  const handlePauseToggle = async () => {
    if (!scheduleId || !scheduleDetails) return;
    setIsActionLoading(true);
    try {
      if (scheduleDetails.paused) {
        await unpauseSchedule(scheduleId);
        toastSuccess({
          title: t('schedule.unpaused', 'Schedule Unpaused'),
          msg: t('schedule.unpaused_msg', 'Unpaused "{id}"', { id: scheduleId }),
        });
      } else {
        await pauseSchedule(scheduleId);
        toastSuccess({
          title: t('schedule.paused', 'Schedule Paused'),
          msg: t('schedule.paused_msg', 'Paused "{id}"', { id: scheduleId }),
        });
      }
      await fetchSchedule(scheduleId);
    } catch (err) {
      const errorMsg = errorMessage(err, 'Operation failed');
      toastError({
        title: t('schedule.pause_error', 'Pause/Unpause Error'),
        msg: errorMsg,
      });
    } finally {
      setIsActionLoading(false);
    }
  };

  const handleKill = async () => {
    if (!scheduleId) return;
    setIsActionLoading(true);
    try {
      const result = await killRunningJob(scheduleId);
      toastSuccess({ title: t('schedule.killed', 'Job Killed'), msg: result.message });
      await fetchSchedule(scheduleId);
    } catch (err) {
      const errorMsg = errorMessage(err, 'Failed to kill job');
      toastError({
        title: t('schedule.kill_error', 'Kill Job Error'),
        msg: errorMsg,
      });
    } finally {
      setIsActionLoading(false);
    }
  };

  const handleInspect = async () => {
    if (!scheduleId) return;
    setIsActionLoading(true);
    try {
      const result = await inspectRunningJob(scheduleId);
      if (result.sessionId) {
        const duration = result.runningDurationSeconds
          ? `${Math.floor(result.runningDurationSeconds / 60)}m ${result.runningDurationSeconds % 60}s`
          : 'Unknown';
        toastSuccess({
          title: t('schedule.job_inspection', 'Job Inspection'),
          msg: t('schedule.job_inspection_msg', 'Session: {session}\nRunning for: {duration}', {
            session: result.sessionId,
            duration,
          }),
        });
      } else {
        toastSuccess({
          title: t('schedule.job_inspection', 'Job Inspection'),
          msg: t('schedule.no_details', 'No detailed information available'),
        });
      }
    } catch (err) {
      const errorMsg = errorMessage(err, 'Failed to inspect job');
      toastError({
        title: t('schedule.inspect_error', 'Inspect Job Error'),
        msg: errorMsg,
      });
    } finally {
      setIsActionLoading(false);
    }
  };

  const handleModalSubmit = async (payload: NewSchedulePayload | string) => {
    if (!scheduleId) return;
    setIsActionLoading(true);
    try {
      await updateSchedule(scheduleId, payload as string);
      toastSuccess({
        title: t('schedule.updated', 'Schedule Updated'),
        msg: t('schedule.updated_msg', 'Updated "{id}"', { id: scheduleId }),
      });
      await fetchSchedule(scheduleId);
      setIsModalOpen(false);
    } catch (err) {
      const errorMsg = errorMessage(err, 'Failed to update schedule');
      toastError({
        title: t('schedule.update_error', 'Update Schedule Error'),
        msg: errorMsg,
      });
    } finally {
      setIsActionLoading(false);
    }
  };

  const loadSession = async (sessionId: string) => {
    setIsLoadingSession(true);
    setSessionError(null);
    try {
      const response = await getSession<true>({
        path: { session_id: sessionId },
        throwOnError: true,
      });
      setSelectedSession(response.data);
    } catch (err) {
      const msg = errorMessage(err, 'Failed to load session');
      setSessionError(msg);
      toastError({ title: t('schedule.load_session_failed', 'Failed to load session'), msg });
    } finally {
      setIsLoadingSession(false);
    }
  };

  if (selectedSession) {
    return (
      <SessionHistoryView
        session={selectedSession}
        isLoading={isLoadingSession}
        error={sessionError}
        onBack={() => setSelectedSession(null)}
        onRetry={() => loadSession(selectedSession.id)}
        showActionButtons={true}
      />
    );
  }

  if (!scheduleId) {
    return (
      <div className="h-screen w-full flex flex-col items-center justify-center bg-white dark:bg-gray-900 text-text-default p-8">
        <BackButton onClick={onNavigateBack} />
        <h1 className="text-2xl font-medium text-text-default mt-4">
          {t('schedule.not_found', 'Schedule Not Found')}
        </h1>
        <p className="text-text-muted mt-2">{t('schedule.no_id', 'No schedule ID provided. Return to schedules list.')}</p>
      </div>
    );
  }

  const readableCron = scheduleDetails
    ? (() => {
        try {
          return cronstrue.toString(scheduleDetails.cron);
        } catch {
          return scheduleDetails.cron;
        }
      })()
    : '';

  return (
    <div className="h-screen w-full flex flex-col bg-background-default text-text-default">
      <div className="px-8 pt-6 pb-4 border-b border-border-default flex-shrink-0">
        <BackButton onClick={onNavigateBack} />
        <h1 className="text-4xl font-light mt-1 mb-1 pt-8">
          {t('schedule.details', 'Schedule Details')}
        </h1>
        <p className="text-sm text-text-muted mb-1">
          {t('schedule.viewing_id', 'Viewing Schedule ID: {id}', { id: scheduleId })}
        </p>
      </div>

      <ScrollArea className="flex-grow">
        <div className="p-8 space-y-6">
          <section>
            <h2 className="text-xl font-semibold text-text-default mb-3">
              {t('schedule.info', 'Schedule Information')}
            </h2>
            {isLoadingSchedule && (
              <div className="flex items-center text-text-muted">
                <Loader2 className="mr-2 h-4 w-4 animate-spin" /> {t('schedule.loading', 'Loading schedule...')}
              </div>
            )}
            {scheduleError && (
              <p className="text-text-danger text-sm p-3 bg-background-danger border border-border-danger rounded-md">
                Error: {scheduleError}
              </p>
            )}
            {scheduleDetails && (
              <Card className="p-4 bg-background-default shadow mb-6">
                <div className="space-y-2">
                  <div className="flex flex-col md:flex-row md:items-center justify-between">
                    <h3 className="text-base font-semibold text-text-default">
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
                    <span className="font-semibold">Last Run:</span>{' '}
                    {formatToLocalDateWithTimezone(scheduleDetails.last_run)}
                  </p>
                  {scheduleDetails.currently_running && scheduleDetails.current_session_id && (
                    <p className="text-sm text-text-default">
                      <span className="font-semibold">Current Session:</span>{' '}
                      {scheduleDetails.current_session_id}
                    </p>
                  )}
                  {scheduleDetails.currently_running && scheduleDetails.process_start_time && (
                    <p className="text-sm text-text-default">
                      <span className="font-semibold">Process Started:</span>{' '}
                      {formatToLocalDateWithTimezone(scheduleDetails.process_start_time)}
                    </p>
                  )}
                </div>
              </Card>
            )}
          </section>

          <section>
            <h2 className="text-xl font-semibold text-text-default mb-3">
              {t('schedule.actions', 'Actions')}
            </h2>
            <div className="flex flex-col md:flex-row gap-2">
              <Button
                onClick={handleRunNow}
                disabled={isActionLoading || scheduleDetails?.currently_running}
                className="w-full md:w-auto"
              >
                {t('schedule.run_now', 'Run Schedule Now')}
              </Button>

              {scheduleDetails && !scheduleDetails.currently_running && (
                <>
                  <Button
                    onClick={() => setIsModalOpen(true)}
                    variant="outline"
                    className="w-full md:w-auto flex items-center gap-2 text-blue-600 dark:text-blue-400 border-blue-300 dark:border-blue-600 hover:bg-blue-50 dark:hover:bg-blue-900/20"
                    disabled={isActionLoading}
                  >
                    <Edit className="w-4 h-4" />
                    {t('schedule.edit', 'Edit Schedule')}
                  </Button>
                  <Button
                    onClick={handlePauseToggle}
                    variant="outline"
                    className={`w-full md:w-auto flex items-center gap-2 ${
                      scheduleDetails.paused
                        ? 'text-green-600 dark:text-green-400 border-green-300 dark:border-green-600 hover:bg-green-50 dark:hover:bg-green-900/20'
                        : 'text-orange-600 dark:text-orange-400 border-orange-300 dark:border-orange-600 hover:bg-orange-50 dark:hover:bg-orange-900/20'
                    }`}
                    disabled={isActionLoading}
                  >
                    {scheduleDetails.paused ? (
                      <>
                        <Play className="w-4 h-4" />
                        {t('schedule.unpause', 'Unpause Schedule')}
                      </>
                    ) : (
                      <>
                        <Pause className="w-4 h-4" />
                        {t('schedule.pause', 'Pause Schedule')}
                      </>
                    )}
                  </Button>
                </>
              )}

              {scheduleDetails?.currently_running && (
                <>
                  <Button
                    onClick={handleInspect}
                    variant="outline"
                    className="w-full md:w-auto flex items-center gap-2 text-blue-600 dark:text-blue-400 border-blue-300 dark:border-blue-600 hover:bg-blue-50 dark:hover:bg-blue-900/20"
                    disabled={isActionLoading}
                  >
                    <Eye className="w-4 h-4" />
                    {t('schedule.inspect', 'Inspect Running Job')}
                  </Button>
                  <Button
                    onClick={handleKill}
                    variant="outline"
                    className="w-full md:w-auto flex items-center gap-2 text-red-600 dark:text-red-400 border-red-300 dark:border-red-600 hover:bg-red-50 dark:hover:bg-red-900/20"
                    disabled={isActionLoading}
                  >
                    <Square className="w-4 h-4" />
                    {t('schedule.kill', 'Kill Running Job')}
                  </Button>
                </>
              )}
            </div>

            {scheduleDetails?.currently_running && (
              <p className="text-sm text-amber-600 dark:text-amber-400 mt-2">
                {t(
                  'schedule.cannot_modify_running',
                  "Cannot trigger or modify a schedule while it's already running."
                )}
              </p>
            )}

            {scheduleDetails?.paused && (
              <p className="text-sm text-orange-600 dark:text-orange-400 mt-2">
                {t(
                  'schedule.paused_note',
                  'This schedule is paused and will not run automatically. Use "Run Schedule Now" to trigger it manually or unpause to resume automatic execution.'
                )}
              </p>
            )}
          </section>

          <section>
            <h2 className="text-xl font-semibold text-text-default mb-4">
              {t('schedule.recent_sessions', 'Recent Sessions')}
            </h2>
            {isLoadingSessions && (
              <p className="text-text-muted">{t('schedule.loading_sessions', 'Loading sessions...')}</p>
            )}
            {sessionsError && (
              <p className="text-text-danger text-sm p-3 bg-background-danger border border-border-danger rounded-md">
                Error: {sessionsError}
              </p>
            )}
            {!isLoadingSessions && sessions.length === 0 && (
              <p className="text-text-muted text-center py-4">
                {t('schedule.no_sessions', 'No sessions found for this schedule.')}
              </p>
            )}

            {sessions.length > 0 && (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {sessions.map((session) => (
                  <Card
                    key={session.id}
                    className="p-4 bg-background-default shadow cursor-pointer hover:shadow-lg transition-shadow duration-200"
                    onClick={() => loadSession(session.id)}
                  >
                    <h3
                      className="text-sm font-semibold text-text-default truncate"
                      title={session.name || session.id}
                    >
                      {session.name || `Session ID: ${session.id}`}
                    </h3>
                    <p className="text-xs text-text-muted mt-1">
                      Created:{' '}
                      {session.createdAt ? formatToLocalDateWithTimezone(session.createdAt) : 'N/A'}
                    </p>
                    {session.messageCount !== undefined && (
                      <p className="text-xs text-text-muted mt-1">
                        Messages: {session.messageCount}
                      </p>
                    )}
                    {session.workingDir && (
                      <p
                        className="text-xs text-text-muted mt-1 truncate"
                        title={session.workingDir}
                      >
                        Dir: {session.workingDir}
                      </p>
                    )}
                    {session.accumulatedTotalTokens !== undefined &&
                      session.accumulatedTotalTokens !== null && (
                        <p className="text-xs text-text-muted mt-1">
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

      <ScheduleModal
        isOpen={isModalOpen}
        onClose={() => setIsModalOpen(false)}
        onSubmit={handleModalSubmit}
        schedule={scheduleDetails}
        isLoadingExternally={isActionLoading}
        apiErrorExternally={null}
        initialDeepLink={null}
      />
    </div>
  );
};

export default ScheduleDetailView;
