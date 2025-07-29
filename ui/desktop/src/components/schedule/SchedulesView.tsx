import React, { useState, useEffect, useCallback, useMemo } from 'react';
import {
  listSchedules,
  createSchedule,
  deleteSchedule,
  pauseSchedule,
  unpauseSchedule,
  updateSchedule,
  killRunningJob,
  inspectRunningJob,
  ScheduledJob,
} from '../../schedule';
import { ScrollArea } from '../ui/scroll-area';
import { Card } from '../ui/card';
import { Button } from '../ui/button';
import { TrashIcon } from '../icons/TrashIcon';
import { Plus, RefreshCw, Pause, Play, Edit, Square, Eye, CircleDotDashed } from 'lucide-react';
import { CreateScheduleModal, NewSchedulePayload } from './CreateScheduleModal';
import { EditScheduleModal } from './EditScheduleModal';
import ScheduleDetailView from './ScheduleDetailView';
import { toastError, toastSuccess } from '../../toasts';
import cronstrue from 'cronstrue';
import { formatToLocalDateWithTimezone } from '../../utils/date';
import { MainPanelLayout } from '../Layout/MainPanelLayout';

interface SchedulesViewProps {
  onClose?: () => void;
}

// Memoized ScheduleCard component to prevent unnecessary re-renders
const ScheduleCard = React.memo<{
  job: ScheduledJob;
  onNavigateToDetail: (id: string) => void;
  onEdit: (job: ScheduledJob) => void;
  onPause: (id: string) => void;
  onUnpause: (id: string) => void;
  onKill: (id: string) => void;
  onInspect: (id: string) => void;
  onDelete: (id: string) => void;
  isPausing: boolean;
  isDeleting: boolean;
  isKilling: boolean;
  isInspecting: boolean;
  isSubmitting: boolean;
}>(
  ({
    job,
    onNavigateToDetail,
    onEdit,
    onPause,
    onUnpause,
    onKill,
    onInspect,
    onDelete,
    isPausing,
    isDeleting,
    isKilling,
    isInspecting,
    isSubmitting,
  }) => {
    const readableCron = useMemo(() => {
      try {
        return cronstrue.toString(job.cron);
      } catch (e) {
        console.warn(`Could not parse cron string "${job.cron}":`, e);
        return job.cron;
      }
    }, [job.cron]);

    const formattedLastRun = useMemo(() => {
      return formatToLocalDateWithTimezone(job.last_run);
    }, [job.last_run]);

    return (
      <Card
        className="py-2 px-4 mb-2 bg-background-default border-none hover:bg-background-muted cursor-pointer transition-all duration-150"
        onClick={() => onNavigateToDetail(job.id)}
      >
        <div className="flex justify-between items-start gap-4">
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2 mb-1">
              <h3 className="text-base truncate max-w-[50vw]" title={job.id}>
                {job.id}
              </h3>
              {job.execution_mode && (
                <span
                  className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium ${
                    job.execution_mode === 'foreground'
                      ? 'bg-background-accent text-text-on-accent'
                      : 'bg-background-medium text-text-default'
                  }`}
                >
                  {job.execution_mode === 'foreground' ? '🖥️' : '⚡'}
                </span>
              )}
              {job.currently_running && (
                <span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300">
                  <span className="inline-block w-2 h-2 bg-green-500 rounded-full mr-1 animate-pulse"></span>
                  Running
                </span>
              )}
              {job.paused && (
                <span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-orange-100 text-orange-800 dark:bg-orange-900/30 dark:text-orange-300">
                  <Pause className="w-3 h-3 mr-1" />
                  Paused
                </span>
              )}
            </div>
            <p className="text-text-muted text-sm mb-2 line-clamp-2" title={readableCron}>
              {readableCron}
            </p>
            <div className="flex items-center text-xs text-text-muted">
              <span>Last run: {formattedLastRun}</span>
            </div>
          </div>

          <div className="flex items-center gap-2 shrink-0">
            {!job.currently_running && (
              <>
                <Button
                  onClick={(e) => {
                    e.stopPropagation();
                    onEdit(job);
                  }}
                  disabled={isPausing || isDeleting || isSubmitting}
                  variant="outline"
                  size="sm"
                  className="h-8"
                >
                  <Edit className="w-4 h-4 mr-1" />
                  Edit
                </Button>
                <Button
                  onClick={(e) => {
                    e.stopPropagation();
                    if (job.paused) {
                      onUnpause(job.id);
                    } else {
                      onPause(job.id);
                    }
                  }}
                  disabled={isPausing || isDeleting}
                  variant="outline"
                  size="sm"
                  className="h-8"
                >
                  {job.paused ? (
                    <>
                      <Play className="w-4 h-4 mr-1" />
                      Resume
                    </>
                  ) : (
                    <>
                      <Pause className="w-4 h-4 mr-1" />
                      Pause
                    </>
                  )}
                </Button>
              </>
            )}
            {job.currently_running && (
              <>
                <Button
                  onClick={(e) => {
                    e.stopPropagation();
                    onInspect(job.id);
                  }}
                  disabled={isInspecting || isKilling}
                  variant="outline"
                  size="sm"
                  className="h-8"
                >
                  <Eye className="w-4 h-4 mr-1" />
                  Inspect
                </Button>
                <Button
                  onClick={(e) => {
                    e.stopPropagation();
                    onKill(job.id);
                  }}
                  disabled={isKilling || isInspecting}
                  variant="outline"
                  size="sm"
                  className="h-8"
                >
                  <Square className="w-4 h-4 mr-1" />
                  Kill
                </Button>
              </>
            )}
            <Button
              onClick={(e) => {
                e.stopPropagation();
                onDelete(job.id);
              }}
              disabled={isPausing || isDeleting || isKilling || isInspecting}
              variant="ghost"
              size="sm"
              className="h-8 text-red-500 hover:text-red-600 hover:bg-red-50 dark:hover:bg-red-900/20"
            >
              <TrashIcon className="w-4 h-4" />
            </Button>
          </div>
        </div>
      </Card>
    );
  }
);

ScheduleCard.displayName = 'ScheduleCard';

const SchedulesView: React.FC<SchedulesViewProps> = ({ onClose: _onClose }) => {
  const [schedules, setSchedules] = useState<ScheduledJob[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [apiError, setApiError] = useState<string | null>(null);
  const [submitApiError, setSubmitApiError] = useState<string | null>(null);
  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);
  const [isEditModalOpen, setIsEditModalOpen] = useState(false);
  const [editingSchedule, setEditingSchedule] = useState<ScheduledJob | null>(null);
  const [isRefreshing, setIsRefreshing] = useState(false);

  // Individual loading states for each action to prevent double-clicks
  const [pausingScheduleIds, setPausingScheduleIds] = useState<Set<string>>(new Set());
  const [deletingScheduleIds, setDeletingScheduleIds] = useState<Set<string>>(new Set());
  const [killingScheduleIds, setKillingScheduleIds] = useState<Set<string>>(new Set());
  const [inspectingScheduleIds, setInspectingScheduleIds] = useState<Set<string>>(new Set());

  const [viewingScheduleId, setViewingScheduleId] = useState<string | null>(null);

  // Memoized fetch function to prevent unnecessary re-creation
  const fetchSchedules = useCallback(async (isRefresh = false) => {
    if (!isRefresh) setIsLoading(true);
    setApiError(null);
    try {
      const fetchedSchedules = await listSchedules();
      setSchedules((prevSchedules) => {
        // Only update if schedules actually changed to prevent unnecessary re-renders
        if (JSON.stringify(prevSchedules) !== JSON.stringify(fetchedSchedules)) {
          return fetchedSchedules;
        }
        return prevSchedules;
      });
    } catch (error) {
      console.error('Failed to fetch schedules:', error);
      setApiError(
        error instanceof Error
          ? error.message
          : 'An unknown error occurred while fetching schedules.'
      );
    } finally {
      if (!isRefresh) setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    if (viewingScheduleId === null) {
      fetchSchedules();

      // Check for pending deep link from recipe editor
      const pendingDeepLink = localStorage.getItem('pendingScheduleDeepLink');
      if (pendingDeepLink) {
        localStorage.removeItem('pendingScheduleDeepLink');
        setIsCreateModalOpen(true);
        // The CreateScheduleModal will handle the deep link
      }
    }
  }, [viewingScheduleId, fetchSchedules]);

  // Optimized periodic refresh - only refresh if not actively doing something
  useEffect(() => {
    if (viewingScheduleId !== null) return;

    // Set up periodic refresh every 15 seconds (increased from 8 to reduce flashing)
    const intervalId = setInterval(() => {
      if (
        viewingScheduleId === null &&
        !isRefreshing &&
        !isLoading &&
        !isSubmitting &&
        pausingScheduleIds.size === 0 &&
        deletingScheduleIds.size === 0 &&
        killingScheduleIds.size === 0 &&
        inspectingScheduleIds.size === 0
      ) {
        fetchSchedules(true); // Pass true to indicate this is a refresh
      }
    }, 15000); // Increased from 8000 to 15000 (15 seconds)

    // Clean up on unmount
    return () => {
      clearInterval(intervalId);
    };
  }, [
    viewingScheduleId,
    isRefreshing,
    isLoading,
    isSubmitting,
    pausingScheduleIds.size,
    deletingScheduleIds.size,
    killingScheduleIds.size,
    inspectingScheduleIds.size,
    fetchSchedules,
  ]);

  const handleOpenCreateModal = () => {
    setSubmitApiError(null);
    setIsCreateModalOpen(true);
  };

  const handleRefresh = useCallback(async () => {
    setIsRefreshing(true);
    try {
      await fetchSchedules();
    } finally {
      setIsRefreshing(false);
    }
  }, [fetchSchedules]);

  const handleCloseCreateModal = () => {
    setIsCreateModalOpen(false);
    setSubmitApiError(null);
  };

  const handleOpenEditModal = (schedule: ScheduledJob) => {
    setEditingSchedule(schedule);
    setSubmitApiError(null);
    setIsEditModalOpen(true);
  };

  const handleCloseEditModal = () => {
    setIsEditModalOpen(false);
    setEditingSchedule(null);
    setSubmitApiError(null);
  };

  const handleCreateScheduleSubmit = async (payload: NewSchedulePayload) => {
    setIsSubmitting(true);
    setSubmitApiError(null);
    try {
      await createSchedule(payload);
      await fetchSchedules();
      setIsCreateModalOpen(false);
    } catch (error) {
      console.error('Failed to create schedule:', error);
      const errorMessage =
        error instanceof Error ? error.message : 'Unknown error creating schedule.';
      setSubmitApiError(errorMessage);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleEditScheduleSubmit = async (cron: string) => {
    if (!editingSchedule) return;

    setIsSubmitting(true);
    setSubmitApiError(null);
    try {
      await updateSchedule(editingSchedule.id, cron);
      toastSuccess({
        title: 'Schedule Updated',
        msg: `Successfully updated schedule "${editingSchedule.id}"`,
      });
      await fetchSchedules();
      setIsEditModalOpen(false);
      setEditingSchedule(null);
    } catch (error) {
      console.error('Failed to update schedule:', error);
      const errorMessage =
        error instanceof Error ? error.message : 'Unknown error updating schedule.';
      setSubmitApiError(errorMessage);
      toastError({
        title: 'Update Schedule Error',
        msg: errorMessage,
      });
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleDeleteSchedule = async (idToDelete: string) => {
    if (!window.confirm(`Are you sure you want to delete schedule "${idToDelete}"?`)) return;

    // Immediately add to deleting set to disable button
    setDeletingScheduleIds((prev) => new Set(prev).add(idToDelete));

    if (viewingScheduleId === idToDelete) {
      setViewingScheduleId(null);
    }
    setApiError(null);
    try {
      await deleteSchedule(idToDelete);
      await fetchSchedules();
    } catch (error) {
      console.error(`Failed to delete schedule "${idToDelete}":`, error);
      setApiError(
        error instanceof Error ? error.message : `Unknown error deleting "${idToDelete}".`
      );
    } finally {
      // Remove from deleting set
      setDeletingScheduleIds((prev) => {
        const newSet = new Set(prev);
        newSet.delete(idToDelete);
        return newSet;
      });
    }
  };

  const handlePauseSchedule = async (idToPause: string) => {
    // Immediately add to pausing set to disable button
    setPausingScheduleIds((prev) => new Set(prev).add(idToPause));

    setApiError(null);
    try {
      await pauseSchedule(idToPause);
      toastSuccess({
        title: 'Schedule Paused',
        msg: `Successfully paused schedule "${idToPause}"`,
      });
      await fetchSchedules();
    } catch (error) {
      console.error(`Failed to pause schedule "${idToPause}":`, error);
      const errorMsg =
        error instanceof Error ? error.message : `Unknown error pausing "${idToPause}".`;
      setApiError(errorMsg);
      toastError({
        title: 'Pause Schedule Error',
        msg: errorMsg,
      });
    } finally {
      // Remove from pausing set
      setPausingScheduleIds((prev) => {
        const newSet = new Set(prev);
        newSet.delete(idToPause);
        return newSet;
      });
    }
  };

  const handleUnpauseSchedule = async (idToUnpause: string) => {
    // Immediately add to pausing set to disable button
    setPausingScheduleIds((prev) => new Set(prev).add(idToUnpause));

    setApiError(null);
    try {
      await unpauseSchedule(idToUnpause);
      toastSuccess({
        title: 'Schedule Unpaused',
        msg: `Successfully unpaused schedule "${idToUnpause}"`,
      });
      await fetchSchedules();
    } catch (error) {
      console.error(`Failed to unpause schedule "${idToUnpause}":`, error);
      const errorMsg =
        error instanceof Error ? error.message : `Unknown error unpausing "${idToUnpause}".`;
      setApiError(errorMsg);
      toastError({
        title: 'Unpause Schedule Error',
        msg: errorMsg,
      });
    } finally {
      // Remove from pausing set
      setPausingScheduleIds((prev) => {
        const newSet = new Set(prev);
        newSet.delete(idToUnpause);
        return newSet;
      });
    }
  };

  const handleKillRunningJob = async (scheduleId: string) => {
    // Immediately add to killing set to disable button
    setKillingScheduleIds((prev) => new Set(prev).add(scheduleId));

    setApiError(null);
    try {
      const result = await killRunningJob(scheduleId);
      toastSuccess({
        title: 'Job Killed',
        msg: result.message,
      });
      await fetchSchedules();
    } catch (error) {
      console.error(`Failed to kill running job "${scheduleId}":`, error);
      const errorMsg =
        error instanceof Error ? error.message : `Unknown error killing job "${scheduleId}".`;
      setApiError(errorMsg);
      toastError({
        title: 'Kill Job Error',
        msg: errorMsg,
      });
    } finally {
      // Remove from killing set
      setKillingScheduleIds((prev) => {
        const newSet = new Set(prev);
        newSet.delete(scheduleId);
        return newSet;
      });
    }
  };

  const handleInspectRunningJob = async (scheduleId: string) => {
    // Immediately add to inspecting set to disable button
    setInspectingScheduleIds((prev) => new Set(prev).add(scheduleId));

    setApiError(null);
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
    } catch (error) {
      console.error(`Failed to inspect running job "${scheduleId}":`, error);
      const errorMsg =
        error instanceof Error ? error.message : `Unknown error inspecting job "${scheduleId}".`;
      setApiError(errorMsg);
      toastError({
        title: 'Inspect Job Error',
        msg: errorMsg,
      });
    } finally {
      // Remove from inspecting set
      setInspectingScheduleIds((prev) => {
        const newSet = new Set(prev);
        newSet.delete(scheduleId);
        return newSet;
      });
    }
  };

  const handleNavigateToScheduleDetail = (scheduleId: string) => {
    setViewingScheduleId(scheduleId);
  };

  const handleNavigateBackFromDetail = () => {
    setViewingScheduleId(null);
  };

  if (viewingScheduleId) {
    return (
      <ScheduleDetailView
        scheduleId={viewingScheduleId}
        onNavigateBack={handleNavigateBackFromDetail}
      />
    );
  }

  return (
    <>
      <MainPanelLayout>
        <div className="flex-1 flex flex-col min-h-0">
          <div className="bg-background-default px-8 pb-8 pt-16">
            <div className="flex flex-col page-transition">
              <div className="flex justify-between items-center mb-1">
                <h1 className="text-4xl font-light">Scheduler</h1>
                <div className="flex gap-2">
                  <Button
                    onClick={handleRefresh}
                    disabled={isRefreshing || isLoading}
                    variant="outline"
                    size="sm"
                    className="flex items-center gap-2"
                  >
                    <RefreshCw className={`h-4 w-4 ${isRefreshing ? 'animate-spin' : ''}`} />
                    {isRefreshing ? 'Refreshing...' : 'Refresh'}
                  </Button>
                  <Button
                    onClick={handleOpenCreateModal}
                    size="sm"
                    className="flex items-center gap-2"
                  >
                    <Plus className="h-4 w-4" />
                    Create Schedule
                  </Button>
                </div>
              </div>
              <p className="text-sm text-text-muted mb-1">
                Create and manage scheduled tasks to run recipes automatically at specified times.
              </p>
            </div>
          </div>

          <div className="flex-1 min-h-0 relative px-8">
            <ScrollArea className="h-full">
              <div className="h-full relative">
                {apiError && (
                  <div className="mb-4 p-4 bg-background-error border border-border-error rounded-md">
                    <p className="text-text-error text-sm">Error: {apiError}</p>
                  </div>
                )}

                {isLoading && schedules.length === 0 && (
                  <div className="flex justify-center items-center py-12">
                    <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-text-default"></div>
                  </div>
                )}

                {!isLoading && !apiError && schedules.length === 0 && (
                  <div className="flex flex-col pt-4 pb-12">
                    <CircleDotDashed className="h-5 w-5 text-text-muted mb-3.5" />
                    <p className="text-base text-text-muted font-light mb-2">No schedules yet</p>
                  </div>
                )}

                {!isLoading && schedules.length > 0 && (
                  <div className="space-y-2 pb-8">
                    {schedules.map((job) => (
                      <ScheduleCard
                        key={job.id}
                        job={job}
                        onNavigateToDetail={handleNavigateToScheduleDetail}
                        onEdit={handleOpenEditModal}
                        onPause={handlePauseSchedule}
                        onUnpause={handleUnpauseSchedule}
                        onKill={handleKillRunningJob}
                        onInspect={handleInspectRunningJob}
                        onDelete={handleDeleteSchedule}
                        isPausing={pausingScheduleIds.has(job.id)}
                        isDeleting={deletingScheduleIds.has(job.id)}
                        isKilling={killingScheduleIds.has(job.id)}
                        isInspecting={inspectingScheduleIds.has(job.id)}
                        isSubmitting={isSubmitting}
                      />
                    ))}
                  </div>
                )}
              </div>
            </ScrollArea>
          </div>
        </div>
      </MainPanelLayout>

      <CreateScheduleModal
        isOpen={isCreateModalOpen}
        onClose={handleCloseCreateModal}
        onSubmit={handleCreateScheduleSubmit}
        isLoadingExternally={isSubmitting}
        apiErrorExternally={submitApiError}
      />
      <EditScheduleModal
        isOpen={isEditModalOpen}
        onClose={handleCloseEditModal}
        onSubmit={handleEditScheduleSubmit}
        schedule={editingSchedule}
        isLoadingExternally={isSubmitting}
        apiErrorExternally={submitApiError}
      />
    </>
  );
};

export default SchedulesView;
