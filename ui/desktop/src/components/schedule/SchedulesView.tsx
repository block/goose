import React, { useState, useEffect } from 'react';
import { listSchedules, createSchedule, deleteSchedule, ScheduledJob } from '../../schedule';
import BackButton from '../ui/BackButton';
import { ScrollArea } from '../ui/scroll-area';
import MoreMenuLayout from '../more_menu/MoreMenuLayout';
import { Card } from '../ui/card';
import { Button } from '../ui/button';
import { TrashIcon } from '../icons/TrashIcon';
import Plus from '../ui/Plus';
import { CreateScheduleModal, NewSchedulePayload } from './CreateScheduleModal';
import ScheduleDetailView from './ScheduleDetailView';
import cronstrue from 'cronstrue';

interface SchedulesViewProps {
  onClose: () => void;
}

const SchedulesView: React.FC<SchedulesViewProps> = ({ onClose }) => {
  const [schedules, setSchedules] = useState<ScheduledJob[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [apiError, setApiError] = useState<string | null>(null);
  const [submitApiError, setSubmitApiError] = useState<string | null>(null);
  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);

  const [viewingScheduleId, setViewingScheduleId] = useState<string | null>(null);

  const fetchSchedules = async () => {
    setIsLoading(true);
    setApiError(null);
    try {
      const fetchedSchedules = await listSchedules();
      setSchedules(fetchedSchedules);
    } catch (error) {
      console.error('Failed to fetch schedules:', error);
      setApiError(
        error instanceof Error
          ? error.message
          : 'An unknown error occurred while fetching schedules.'
      );
    } finally {
      setIsLoading(false);
    }
  };

  useEffect(() => {
    if (viewingScheduleId === null) {
      fetchSchedules();
    }
  }, [viewingScheduleId]);

  const handleOpenCreateModal = () => {
    setSubmitApiError(null);
    setIsCreateModalOpen(true);
  };

  const handleCloseCreateModal = () => {
    setIsCreateModalOpen(false);
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

  const handleDeleteSchedule = async (idToDelete: string) => {
    if (!window.confirm(`Are you sure you want to delete schedule "${idToDelete}"?`)) return;
    if (viewingScheduleId === idToDelete) {
      setViewingScheduleId(null);
    }
    setIsLoading(true);
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
      setIsLoading(false);
    }
  };

  const handleNavigateToScheduleDetail = (scheduleId: string) => {
    setViewingScheduleId(scheduleId);
  };

  const handleNavigateBackFromDetail = () => {
    setViewingScheduleId(null);
  };

  const getReadableCron = (cronString: string) => {
    try {
      return cronstrue.toString(cronString);
    } catch (e) {
      console.warn(`Could not parse cron string "${cronString}":`, e);
      return cronString;
    }
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
          <Button
            onClick={handleOpenCreateModal}
            className="w-full md:w-auto flex items-center gap-2 justify-center text-white dark:text-black bg-bgAppInverse hover:bg-bgStandardInverse [&>svg]:!size-4 mb-8"
          >
            <Plus className="h-4 w-4" /> Create New Schedule
          </Button>

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
                  <Card
                    key={job.id}
                    className="p-4 bg-white dark:bg-gray-800 shadow cursor-pointer hover:shadow-lg transition-shadow duration-200"
                    onClick={() => handleNavigateToScheduleDetail(job.id)}
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
                        <p
                          className="text-xs text-gray-500 dark:text-gray-400 mt-1"
                          title={getReadableCron(job.cron)}
                        >
                          Schedule: {getReadableCron(job.cron)}
                        </p>
                        <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                          Last Run:{' '}
                          {job.last_run ? new Date(job.last_run).toLocaleString() : 'Never'}
                        </p>
                      </div>
                      <div className="flex-shrink-0">
                        <Button
                          variant="ghost"
                          size="icon"
                          onClick={(e) => {
                            e.stopPropagation();
                            handleDeleteSchedule(job.id);
                          }}
                          className="text-gray-500 dark:text-gray-400 hover:text-red-500 dark:hover:text-red-400 hover:bg-red-100/50 dark:hover:bg-red-900/30"
                          title={`Delete schedule ${job.id}`}
                          disabled={isLoading}
                        >
                          <TrashIcon className="w-5 h-5" />
                        </Button>
                      </div>
                    </div>
                  </Card>
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
    </div>
  );
};

export default SchedulesView;
