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
import BackButton from '../ui/BackButton';
import { ScrollArea } from '../ui/scroll-area';
import MoreMenuLayout from '../more_menu/MoreMenuLayout';
import { Card } from '../ui/card';
import { Button } from '../ui/button';
import { TrashIcon } from '../icons/TrashIcon';
import { Plus, RefreshCw, Pause, Play, Edit, Square, Eye, MoreHorizontal } from 'lucide-react';
import { CreateScheduleModal, NewSchedulePayload } from './CreateScheduleModal';
import { EditScheduleModal } from './EditScheduleModal';
import ScheduleDetailView from './ScheduleDetailView';
import { toastError, toastSuccess } from '../../toasts';
import { Popover, PopoverContent, PopoverTrigger } from '../ui/popover';
import cronstrue from 'cronstrue';
import { formatToLocalDateWithTimezone } from '../../utils/date';

interface SchedulesViewProps {
  onClose: () => void;
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
        className="p-4 bg-white dark:bg-gray-800 shadow cursor-pointer hover:shadow-lg transition-shadow duration-200"
        onClick={() => onNavigateToDetail(job.id)}
      >
        <div className="flex justify-between items-start">
          <div className="flex-grow mr-2 overflow-hidden">
            <h3
              className="text-base font-semibold text-gray-900 dark:text-white truncate"
              title={job.id}
            >
              {job.id}
            </h3>
            <p
              className="text-xs text-gray-500 dark:text-gray-400 mt-1 break-all"
              title={job.source}
            >
              Source: {job.source}
            </p>
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-1" title={readableCron}>
              Schedule: {readableCron}
            </p>
            <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
              Last Run: {formattedLastRun}
            </p>
            {job.execution_mode && (
              <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                Mode:{' '}
                <span
                  className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium ${
                    job.execution_mode === 'foreground'
                      ? 'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300'
                      : 'bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-300'
                  }`}
                >
                  {job.execution_mode === 'foreground' ? '🖥️ Foreground' : '⚡ Background'}
                </span>
              </p>
            )}
            {job.currently_running && (
              <p className="text-xs text-green-500 dark:text-green-400 mt-1 font-semibold flex items-center">
                <span className="inline-block w-2 h-2 bg-green-500 dark:bg-green-400 rounded-full mr-1 animate-pulse"></span>
                Currently Running
              </p>
            )}
            {job.paused && (
              <p className="text-xs text-orange-500 dark:text-orange-400 mt-1 font-semibold flex items-center">
                <Pause className="w-3 h-3 mr-1" />
                Paused
              </p>
            )}
          </div>
          <div className="flex-shrink-0">
            <Popover>
              <PopoverTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  onClick={(e) => {
                    e.stopPropagation();
                  }}
                  className="text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-300 hover:bg-gray-100/50 dark:hover:bg-gray-800/50"
                >
                  <MoreHorizontal className="w-4 h-4" />
                </Button>
              </PopoverTrigger>
              <PopoverContent
                className="w-48 p-1 bg-white dark:bg-gray-800 border border-gray-200 dark:border-gray-600 shadow-lg"
                align="end"
              >
                <div className="space-y-1">
                  {!job.currently_running && (
                    <>
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          onEdit(job);
                        }}
                        disabled={isPausing || isDeleting || isSubmitting}
                        className="w-full flex items-center justify-between px-3 py-2 text-sm text-gray-900 dark:text-white hover:bg-gray-100 dark:hover:bg-gray-700 rounded-md disabled:opacity-50 disabled:cursor-not-allowed"
                      >
                        <span>Edit</span>
                        <Edit className="w-4 h-4" />
                      </button>
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          if (job.paused) {
                            onUnpause(job.id);
                          } else {
                            onPause(job.id);
                          }
                        }}
                        disabled={isPausing || isDeleting}
                        className="w-full flex items-center justify-between px-3 py-2 text-sm text-gray-900 dark:text-white hover:bg-gray-100 dark:hover:bg-gray-700 rounded-md disabled:opacity-50 disabled:cursor-not-allowed"
                      >
                        <span>{job.paused ? 'Resume schedule' : 'Stop schedule'}</span>
                        {job.paused ? <Play className="w-4 h-4" /> : <Pause className="w-4 h-4" />}
                      </button>
                    </>
                  )}
                  {job.currently_running && (
                    <>
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          onInspect(job.id);
                        }}
                        disabled={isInspecting || isKilling}
                        className="w-full flex items-center justify-between px-3 py-2 text-sm text-gray-900 dark:text-white hover:bg-gray-100 dark:hover:bg-gray-700 rounded-md disabled:opacity-50 disabled:cursor-not-allowed"
                      >
                        <span>Inspect</span>
                        <Eye className="w-4 h-4" />
                      </button>
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          onKill(job.id);
                        }}
                        disabled={isKilling || isInspecting}
                        className="w-full flex items-center justify-between px-3 py-2 text-sm text-gray-900 dark:text-white hover:bg-gray-100 dark:hover:bg-gray-700 rounded-md disabled:opacity-50 disabled:cursor-not-allowed"
                      >
                        <span>Kill job</span>
                        <Square className="w-4 h-4" />
                      </button>
                    </>
                  )}
                  <hr className="border-gray-200 dark:border-gray-600 my-1" />
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      onDelete(job.id);
                    }}
                    disabled={isPausing || isDeleting || isKilling || isInspecting}
                    className="w-full flex items-center justify-between px-3 py-2 text-sm text-red-600 dark:text-red-400 hover:bg-red-50 dark:hover:bg-red-900/20 rounded-md disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    <span>Delete</span>
                    <TrashIcon className="w-4 h-4" />
                  </button>
                </div>
              </PopoverContent>
            </Popover>
          </div>
        </div>
      </Card>
    );
  }
);

ScheduleCard.displayName = 'ScheduleCard';

const SchedulesView: React.FC<SchedulesViewProps> = ({ onClose }) => {
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
    <div className="h-screen w-full flex flex-col bg-app text-textStandard">
      <MoreMenuLayout showMenu={false} />
      <div className="px-8 pt-6 pb-4 border-b border-borderSubtle flex-shrink-0">
        <BackButton onClick={onClose} />
        <h1 className="text-2xl font-semibold text-gray-900 dark:text-white mt-2">
          Schedules Management
        </h1>
      </div>

      <ScrollArea className="flex-grow">
        <div className="p-8">
          <div className="flex flex-col md:flex-row gap-2 mb-8">
            <Button
              onClick={handleOpenCreateModal}
              className="w-full md:w-auto flex items-center gap-2 justify-center text-white dark:text-black bg-bgAppInverse hover:bg-bgStandardInverse [&>svg]:!size-4"
            >
              <Plus className="h-4 w-4" /> Create New Schedule
            </Button>

            <Button
              onClick={handleRefresh}
              disabled={isRefreshing || isLoading}
              variant="outline"
              className="w-full md:w-auto flex items-center gap-2 justify-center rounded-full [&>svg]:!size-4"
            >
              <RefreshCw className={`h-4 w-4 ${isRefreshing ? 'animate-spin' : ''}`} />
              {isRefreshing ? 'Refreshing...' : 'Refresh'}
            </Button>
          </div>

          {apiError && (
            <p className="text-red-500 dark:text-red-400 text-sm p-4 bg-red-100 dark:bg-red-900/30 border border-red-500 dark:border-red-700 rounded-md">
              Error: {apiError}
            </p>
          )}

          <section>
            <h2 className="text-xl font-semibold text-gray-900 dark:text-white mb-4">
              Existing Schedules
            </h2>
            {isLoading && schedules.length === 0 && (
              <p className="text-gray-500 dark:text-gray-400">Loading schedules...</p>
            )}
            {!isLoading && !apiError && schedules.length === 0 && (
              <p className="text-gray-500 dark:text-gray-400 text-center py-4">
                No schedules found. Create one to get started!
              </p>
            )}

            {!isLoading && schedules.length > 0 && (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
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
          </section>
        </div>
      </ScrollArea>
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
    </div>
  );
};

export default SchedulesView;
