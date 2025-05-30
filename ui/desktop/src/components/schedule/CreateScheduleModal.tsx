import React, { useState, useEffect, FormEvent } from 'react';
import { Card } from '../ui/card';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Select } from '../ui/Select';
import cronstrue from 'cronstrue';

type FrequencyValue = 'once' | 'hourly' | 'daily' | 'weekly' | 'monthly';

interface FrequencyOption {
  value: FrequencyValue;
  label: string;
}

export interface NewSchedulePayload {
  id: string;
  recipe_source: string;
  cron: string;
}

interface CreateScheduleModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (payload: NewSchedulePayload) => Promise<void>;
  isLoadingExternally: boolean;
  apiErrorExternally: string | null;
}

const frequencies: FrequencyOption[] = [
  { value: 'once', label: 'Once' },
  { value: 'hourly', label: 'Hourly' },
  { value: 'daily', label: 'Daily' },
  { value: 'weekly', label: 'Weekly' },
  { value: 'monthly', label: 'Monthly' },
];

const daysOfWeekOptions: { value: string; label: string }[] = [
  { value: '1', label: 'Mon' },
  { value: '2', label: 'Tue' },
  { value: '3', label: 'Wed' },
  { value: '4', label: 'Thu' },
  { value: '5', label: 'Fri' },
  { value: '6', label: 'Sat' },
  { value: '0', label: 'Sun' },
];

const modalLabelClassName = 'block text-sm font-medium text-gray-700 dark:text-gray-300 mb-1';
const cronPreviewTextColor = 'text-xs text-gray-500 dark:text-gray-400 mt-1';
const cronPreviewSpecialNoteColor = 'text-xs text-yellow-600 dark:text-yellow-500 mt-1';
const checkboxLabelClassName = 'flex items-center text-sm text-textStandard dark:text-gray-300';
const checkboxInputClassName =
  'h-4 w-4 text-indigo-600 border-gray-300 dark:border-gray-600 rounded focus:ring-indigo-500 mr-2';

export const CreateScheduleModal: React.FC<CreateScheduleModalProps> = ({
  isOpen,
  onClose,
  onSubmit,
  isLoadingExternally,
  apiErrorExternally,
}) => {
  const [scheduleId, setScheduleId] = useState<string>('');
  const [recipeSourcePath, setRecipeSourcePath] = useState<string>('');
  const [frequency, setFrequency] = useState<FrequencyValue>('daily');
  const [selectedDate, setSelectedDate] = useState<string>(
    () => new Date().toISOString().split('T')[0]
  );
  const [selectedTime, setSelectedTime] = useState<string>('09:00');
  const [selectedMinute, setSelectedMinute] = useState<string>('0');
  const [selectedDaysOfWeek, setSelectedDaysOfWeek] = useState<Set<string>>(new Set(['1']));
  const [selectedDayOfMonth, setSelectedDayOfMonth] = useState<string>('1');
  const [derivedCronExpression, setDerivedCronExpression] = useState<string>('');
  const [readableCronExpression, setReadableCronExpression] = useState<string>('');
  const [internalValidationError, setInternalValidationError] = useState<string | null>(null);

  const resetForm = () => {
    setScheduleId('');
    setRecipeSourcePath('');
    setFrequency('daily');
    setSelectedDate(new Date().toISOString().split('T')[0]);
    setSelectedTime('09:00');
    setSelectedMinute('0');
    setSelectedDaysOfWeek(new Set(['1']));
    setSelectedDayOfMonth('1');
    setInternalValidationError(null);
    setReadableCronExpression('');
  };

  const handleBrowseFile = async () => {
    const filePath = await window.electron.selectFileOrDirectory();
    if (filePath) {
      if (filePath.endsWith('.yaml') || filePath.endsWith('.yml')) {
        setRecipeSourcePath(filePath);
        setInternalValidationError(null);
      } else {
        setInternalValidationError('Invalid file type: Please select a YAML file (.yaml or .yml)');
        console.warn('Invalid file type: Please select a YAML file (.yaml or .yml)');
      }
    }
  };

  useEffect(() => {
    const generateCronExpression = (): string => {
      const timeParts = selectedTime.split(':');
      const minutePart = timeParts.length > 1 ? String(parseInt(timeParts[1], 10)) : '0';
      const hourPart = timeParts.length > 0 ? String(parseInt(timeParts[0], 10)) : '0';
      if (isNaN(parseInt(minutePart)) || isNaN(parseInt(hourPart))) {
        return 'Invalid time format.';
      }
      const secondsPart = '0';
      switch (frequency) {
        case 'once':
          if (selectedDate && selectedTime) {
            try {
              const dateObj = new Date(`${selectedDate}T${selectedTime}`);
              if (isNaN(dateObj.getTime())) return "Invalid date/time for 'once'.";
              return `${secondsPart} ${dateObj.getMinutes()} ${dateObj.getHours()} ${dateObj.getDate()} ${
                dateObj.getMonth() + 1
              } *`;
            } catch (e) {
              return "Error parsing date/time for 'once'.";
            }
          }
          return 'Date and Time are required for "Once" frequency.';
        case 'hourly': {
          const sMinute = parseInt(selectedMinute, 10);
          if (isNaN(sMinute) || sMinute < 0 || sMinute > 59) {
            return 'Invalid minute (0-59) for hourly frequency.';
          }
          return `${secondsPart} ${sMinute} * * * *`;
        }
        case 'daily':
          return `${secondsPart} ${minutePart} ${hourPart} * * *`;
        case 'weekly': {
          if (selectedDaysOfWeek.size === 0) {
            return 'Select at least one day for weekly frequency.';
          }
          const days = Array.from(selectedDaysOfWeek)
            .sort((a, b) => parseInt(a) - parseInt(b))
            .join(',');
          return `${secondsPart} ${minutePart} ${hourPart} * * ${days}`;
        }
        case 'monthly': {
          const sDayOfMonth = parseInt(selectedDayOfMonth, 10);
          if (isNaN(sDayOfMonth) || sDayOfMonth < 1 || sDayOfMonth > 31) {
            return 'Invalid day of month (1-31) for monthly frequency.';
          }
          return `${secondsPart} ${minutePart} ${hourPart} ${sDayOfMonth} * *`;
        }
        default:
          return 'Invalid frequency selected.';
      }
    };
    const cron = generateCronExpression();
    setDerivedCronExpression(cron);
    try {
      if (
        cron.includes('Invalid') ||
        cron.includes('required') ||
        cron.includes('Error') ||
        cron.includes('Select at least one')
      ) {
        setReadableCronExpression('Invalid cron details provided.');
      } else {
        setReadableCronExpression(cronstrue.toString(cron));
      }
    } catch (e) {
      setReadableCronExpression('Could not parse cron string.');
    }
  }, [
    frequency,
    selectedDate,
    selectedTime,
    selectedMinute,
    selectedDaysOfWeek,
    selectedDayOfMonth,
  ]);

  const handleDayOfWeekChange = (dayValue: string) => {
    setSelectedDaysOfWeek((prev) => {
      const newSet = new Set(prev);
      if (newSet.has(dayValue)) {
        newSet.delete(dayValue);
      } else {
        newSet.add(dayValue);
      }
      return newSet;
    });
  };

  const handleLocalSubmit = async (event: FormEvent) => {
    event.preventDefault();
    setInternalValidationError(null);

    if (!scheduleId.trim()) {
      setInternalValidationError('Schedule ID is required.');
      return;
    }
    if (!recipeSourcePath) {
      setInternalValidationError('Recipe source file is required.');
      return;
    }
    if (
      !derivedCronExpression ||
      derivedCronExpression.includes('Invalid') ||
      derivedCronExpression.includes('required') ||
      derivedCronExpression.includes('Error') ||
      derivedCronExpression.includes('Select at least one')
    ) {
      setInternalValidationError(`Invalid cron expression: ${derivedCronExpression}`);
      return;
    }
    if (frequency === 'weekly' && selectedDaysOfWeek.size === 0) {
      setInternalValidationError('For weekly frequency, select at least one day.');
      return;
    }

    const newSchedulePayload: NewSchedulePayload = {
      id: scheduleId.trim(),
      recipe_source: recipeSourcePath,
      cron: derivedCronExpression,
    };

    await onSubmit(newSchedulePayload);
  };

  const handleClose = () => {
    resetForm();
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/20 backdrop-blur-sm z-40 flex items-center justify-center p-4">
      <Card className="w-full max-w-md  bg-bgApp shadow-xl rounded-lg z-50 flex flex-col max-h-[90vh] overflow-hidden">
        <div className="px-6 pt-6 pb-4 flex-shrink-0">
          <h2 className="text-xl font-semibold text-gray-900 dark:text-white">
            Create New Schedule
          </h2>
        </div>

        <form
          id="new-schedule-form"
          onSubmit={handleLocalSubmit}
          className="px-6 py-4 space-y-4 flex-grow overflow-y-auto"
        >
          {apiErrorExternally && (
            <p className="text-red-500 text-sm mb-3 p-2 bg-red-100 dark:bg-red-900/30 rounded-md border border-red-500/50">
              {apiErrorExternally}
            </p>
          )}
          {internalValidationError && (
            <p className="text-red-500 text-sm mb-3 p-2 bg-red-100 dark:bg-red-900/30 rounded-md border border-red-500/50">
              {internalValidationError}
            </p>
          )}

          <div>
            <label htmlFor="scheduleId-modal" className={modalLabelClassName}>
              Schedule ID:
            </label>
            <Input
              type="text"
              id="scheduleId-modal"
              value={scheduleId}
              onChange={(e) => setScheduleId(e.target.value)}
              placeholder="e.g., daily-summary-job"
              required
            />
          </div>
          <div>
            <label className={modalLabelClassName}>Recipe Source (YAML File):</label>
            <Button
              type="button"
              variant="outline"
              onClick={handleBrowseFile}
              className="w-full justify-center"
            >
              Browse...
            </Button>
            {recipeSourcePath && (
              <p className="mt-2 text-xs text-gray-500 dark:text-gray-400 italic">
                Selected: {recipeSourcePath}
              </p>
            )}
          </div>
          <div>
            <label htmlFor="frequency-modal" className={modalLabelClassName}>
              Frequency:
            </label>
            <Select
              instanceId="frequency-select-modal"
              options={frequencies}
              value={frequencies.find((f) => f.value === frequency)}
              onChange={(selectedOption: FrequencyOption | null) => {
                if (selectedOption) setFrequency(selectedOption.value);
              }}
              placeholder="Select frequency..."
            />
          </div>

          {frequency === 'once' && (
            <>
              <div>
                <label htmlFor="onceDate-modal" className={modalLabelClassName}>
                  Date:
                </label>
                <Input
                  type="date"
                  id="onceDate-modal"
                  value={selectedDate}
                  onChange={(e) => setSelectedDate(e.target.value)}
                  required
                />
              </div>
              <div>
                <label htmlFor="onceTime-modal" className={modalLabelClassName}>
                  Time:
                </label>
                <Input
                  type="time"
                  id="onceTime-modal"
                  value={selectedTime}
                  onChange={(e) => setSelectedTime(e.target.value)}
                  required
                />
              </div>
            </>
          )}
          {frequency === 'hourly' && (
            <div>
              <label htmlFor="hourlyMinute-modal" className={modalLabelClassName}>
                Minute of the hour (0-59):
              </label>
              <Input
                type="number"
                id="hourlyMinute-modal"
                min="0"
                max="59"
                value={selectedMinute}
                onChange={(e) => setSelectedMinute(e.target.value)}
                required
              />
            </div>
          )}
          {(frequency === 'daily' || frequency === 'weekly' || frequency === 'monthly') && (
            <div>
              <label htmlFor="commonTime-modal" className={modalLabelClassName}>
                Time:
              </label>
              <Input
                type="time"
                id="commonTime-modal"
                value={selectedTime}
                onChange={(e) => setSelectedTime(e.target.value)}
                required
              />
            </div>
          )}
          {frequency === 'weekly' && (
            <div>
              <label className={modalLabelClassName}>Days of Week:</label>
              <div className="grid grid-cols-3 sm:grid-cols-4 gap-2 mt-1">
                {daysOfWeekOptions.map((day) => (
                  <label key={day.value} className={checkboxLabelClassName}>
                    <input
                      type="checkbox"
                      value={day.value}
                      checked={selectedDaysOfWeek.has(day.value)}
                      onChange={() => handleDayOfWeekChange(day.value)}
                      className={checkboxInputClassName}
                    />
                    {day.label}
                  </label>
                ))}
              </div>
            </div>
          )}
          {frequency === 'monthly' && (
            <div>
              <label htmlFor="monthlyDay-modal" className={modalLabelClassName}>
                Day of Month (1-31):
              </label>
              <Input
                type="number"
                id="monthlyDay-modal"
                min="1"
                max="31"
                value={selectedDayOfMonth}
                onChange={(e) => setSelectedDayOfMonth(e.target.value)}
                required
              />
            </div>
          )}
          <div className="mt-4 p-3 bg-gray-100 dark:bg-gray-700/50 rounded-md border border-gray-200 dark:border-gray-600">
            <p className="text-sm font-medium text-gray-700 dark:text-gray-300">
              Generated Cron:{' '}
              <code className="text-xs bg-gray-200 dark:bg-gray-600 p-1 rounded">
                {derivedCronExpression}
              </code>
            </p>
            <p className={`${cronPreviewTextColor} mt-2`}>
              <b>Human Readable:</b> {readableCronExpression}
            </p>
            <p className={cronPreviewTextColor}>Syntax: S M H D M DoW. (S=0, DoW: 0/7=Sun)</p>
            {frequency === 'once' && (
              <p className={cronPreviewSpecialNoteColor}>
                Note: "Once" schedules recur annually. True one-time tasks may need backend deletion
                after execution.
              </p>
            )}
          </div>
        </form>

        {/* Actions */}
        <div className="mt-[8px] ml-[-24px] mr-[-24px] pt-[16px]">
          <Button
            type="button"
            variant="ghost"
            onClick={handleClose}
            disabled={isLoadingExternally}
            className="w-full h-[60px] rounded-none border-t text-gray-400 hover:bg-gray-50 dark:border-gray-600 text-lg font-regular"
          >
            Cancel
          </Button>
          <Button
            type="submit"
            form="new-schedule-form"
            variant="default"
            disabled={isLoadingExternally}
            className="w-full h-[60px] rounded-none border-t dark:border-gray-600 text-lg dark:text-white dark:border-gray-600 font-regular"
          >
            {isLoadingExternally ? 'Creating...' : 'Create Schedule'}
          </Button>
        </div>
      </Card>
    </div>
  );
};
