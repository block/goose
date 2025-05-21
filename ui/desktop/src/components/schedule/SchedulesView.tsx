import React, { useState, useEffect } from 'react';
import { listSchedules, createSchedule, deleteSchedule, ScheduledJob } from '../../schedule';
import BackButton from '../ui/BackButton';
import { ScrollArea } from '../ui/scroll-area';
import MoreMenuLayout from '../more_menu/MoreMenuLayout';
import { Card } from '../ui/card';
import { Button } from '../ui/button';
import { TrashIcon } from '../icons/TrashIcon';
import Plus from '../ui/Plus';
import { CreateScheduleModal, NewSchedulePayload } from './CreateScheduleModal'; // Import the new modal

interface SchedulesViewProps {
  onClose: () => void;
}

const SchedulesView: React.FC<SchedulesViewProps> = ({ onClose }) => {
  const [schedules, setSchedules] = useState<ScheduledJob[]>([]);
  const [isLoading, setIsLoading] = useState(false); // Generic loading for list/delete
  const [isSubmitting, setIsSubmitting] = useState(false); // Specific loading for create
  const [apiError, setApiError] = useState<string | null>(null); // Generic error for list/delete
  const [submitApiError, setSubmitApiError] = useState<string | null>(null); // Specific error for create modal
  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);

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
    fetchSchedules();
  }, []);

  const handleOpenCreateModal = () => {
    setSubmitApiError(null); // Clear previous submission errors
    setIsCreateModalOpen(true);
  };

  const handleCloseCreateModal = () => {
    setIsCreateModalOpen(false);
    setSubmitApiError(null); // Clear errors when modal is closed
  };

  const handleCreateScheduleSubmit = async (payload: NewSchedulePayload) => {
    setIsSubmitting(true);
    setSubmitApiError(null);
    try {
      await createSchedule(payload);
      await fetchSchedules(); // Refresh the list
      setIsCreateModalOpen(false); // Close modal on success
      // The form reset is handled inside CreateScheduleModal or on its close action
    } catch (error) {
      console.error('Failed to create schedule:', error);
      const errorMessage =
        error instanceof Error ? error.message : 'Unknown error creating schedule.';
      setSubmitApiError(errorMessage);
      // Keep modal open if there's an error, so user can see message / retry
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleDeleteSchedule = async (idToDelete: string) => {
    if (!window.confirm(`Are you sure you want to delete schedule "${idToDelete}"?`)) return;
    setIsLoading(true); // Use generic isLoading for delete as it affects the list
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

  return (
    <div className="h-screen w-full flex flex-col bg-app text-textStandard">
      <MoreMenuLayout showMenu={false} />
      <div className="px-8 pt-6 pb-4 border-b border-borderSubtle flex-shrink-0">
        <BackButton onClick={onClose} />
        <h1 className="text-3xl font-medium text-gray-900 dark:text-white mt-1">
          Schedules Management
        </h1>
      </div>

      <ScrollArea className="flex-grow">
        <div className="p-8 space-y-8">
          <Button
            variant="outline"
            onClick={handleOpenCreateModal}
            className="w-full !h-auto p-6 border-dashed border-2 text-lg hover:border-indigo-500 flex items-center justify-center text-gray-600 dark:text-gray-300 hover:text-indigo-600 dark:hover:text-indigo-400 hover:border-indigo-600 dark:hover:border-indigo-400"
          >
            <Plus className="w-5 h-5 mr-2" /> Create New Schedule
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
                  <Card key={job.id} className="p-4 bg-white dark:bg-gray-800 shadow">
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
                        <p className="text-xs text-gray-500 dark:text-gray-400 mt-1">
                          Cron:{' '}
                          <code className="text-xs bg-gray-100 dark:bg-gray-700 p-0.5 rounded">
                            {job.cron}
                          </code>
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
                          onClick={() => handleDeleteSchedule(job.id)}
                          className="text-gray-500 dark:text-gray-400 hover:text-red-500 dark:hover:text-red-400 hover:bg-red-100/50 dark:hover:bg-red-900/30"
                          title={`Delete schedule ${job.id}`}
                          disabled={isLoading} // Disable delete if list is loading or another delete is in progress
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
