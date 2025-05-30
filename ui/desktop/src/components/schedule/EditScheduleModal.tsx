import React, { useState, useEffect, FormEvent } from 'react';
import { Card } from '../ui/card';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Select } from '../ui/Select';
import { ScheduledJob } from '../../schedule';
import cronstrue from 'cronstrue';

type FrequencyValue = 'once' | 'hourly' | 'daily' | 'weekly' | 'monthly';

interface FrequencyOption {
  value: FrequencyValue;
  label: string;
}

interface EditScheduleModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (cron: string) => Promise<void>;
  schedule: ScheduledJob | null;
  isLoadingExternally?: boolean;
  apiErrorExternally?: string | null;
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

// Helper function to parse cron expression and determine frequency
const parseCronExpression = (cron: string) => {
  const parts = cron.split(' ');
  if (parts.length !== 6) return null;

  const [_seconds, minutes, hours, dayOfMonth, month, dayOfWeek] = parts;

  // Check for specific patterns
  if (dayOfMonth !== '*' && month !== '*' && dayOfWeek === '*') {
    return { frequency: 'once' as FrequencyValue, minutes, hours, dayOfMonth, month };
  }
  if (minutes !== '*' && hours === '*' && dayOfMonth === '*' && month === '*' && dayOfWeek === '*') {
    return { frequency: 'hourly' as FrequencyValue, minutes };
  }
  if (minutes !== '*' && hours !== '*' && dayOfMonth === '*' && month === '*' && dayOfWeek === '*') {
    return { frequency: 'daily' as FrequencyValue, minutes, hours };
  }
  if (minutes !== '*' && hours !== '*' && dayOfMonth === '*' && month === '*' && dayOfWeek !== '*') {
    return { frequency: 'weekly' as FrequencyValue, minutes, hours, dayOfWeek };
  }
  if (minutes !== '*' && hours !== '*' && dayOfMonth !== '*' && month === '*' && dayOfWeek === '*') {
    return { frequency: 'monthly' as FrequencyValue, minutes, hours, dayOfMonth };
  }

  return null;
};

export const EditScheduleModal: React.FC<EditScheduleModalProps> = ({
  isOpen,
  onClose,
  onSubmit,
  schedule,
  isLoadingExternally = false,
  apiErrorExternally = null,
}) => {
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

  // Initialize form from existing schedule
  useEffect(() => {
    if (schedule && isOpen) {
      const parsed = parseCronExpression(schedule.cron);
      
      if (parsed) {
        setFrequency(parsed.frequency);
        
        switch (parsed.frequency) {
          case 'once':
            // For 'once', we'd need to reconstruct the date from cron parts
            // This is complex, so we'll default to current date/time for now
            setSelectedDate(new Date().toISOString().split('T')[0]);
            setSelectedTime(`${parsed.hours?.padStart(2, '0')}:${parsed.minutes?.padStart(2, '0')}`);
            break;
          case 'hourly':
            setSelectedMinute(parsed.minutes || '0');
            break;
          case 'daily':
            setSelectedTime(`${parsed.hours?.padStart(2, '0')}:${parsed.minutes?.padStart(2, '0')}`);
            break;
          case 'weekly':
            setSelectedTime(`${parsed.hours?.padStart(2, '0')}:${parsed.minutes?.padStart(2, '0')}`);
            if (parsed.dayOfWeek) {
              const days = parsed.dayOfWeek.split(',').map(d => d.trim());
              setSelectedDaysOfWeek(new Set(days));
            }
            break;
          case 'monthly':
            setSelectedTime(`${parsed.hours?.padStart(2, '0')}:${parsed.minutes?.padStart(2, '0')}`);
            setSelectedDayOfMonth(parsed.dayOfMonth || '1');
            break;
        }
      } else {
        // If we can't parse the cron, default to daily at 9 AM
        setFrequency('daily');
        setSelectedTime('09:00');
      }
      
      setInternalValidationError(null);
    }
  }, [schedule, isOpen]);

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

    await onSubmit(derivedCronExpression);
  };

  const handleClose = () => {
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/20 backdrop-blur-sm z-40 flex items-center justify-center p-4">
      <Card className="w-full max-w-md bg-bgApp shadow-xl rounded-lg z-50 flex flex-col max-h-[90vh] overflow-hidden">
        <div className="px-6 pt-6 pb-4 flex-shrink-0">
          <h2 className="text-xl font-semibold text-gray-900 dark:text-white">
            Edit Schedule: {schedule?.id || ''}
          </h2>
        </div>

        <form
          id="edit-schedule-form"
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
            form="edit-schedule-form"
            variant="default"
            disabled={isLoadingExternally}
            className="w-full h-[60px] rounded-none border-t dark:border-gray-600 text-lg dark:text-white dark:border-gray-600 font-regular"
          >
            {isLoadingExternally ? 'Updating...' : 'Update Schedule'}
          </Button>
        </div>
      </Card>
    </div>
  );
};