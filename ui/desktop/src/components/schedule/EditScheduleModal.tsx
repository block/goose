import React, { useState, useEffect, FormEvent } from 'react';
import { Card } from '../ui/card';
import { Button } from '../ui/button';
import { Input } from '../ui/input';
import { Select } from '../ui/Select';
import { ScheduledJob } from '../../schedule';
import cronstrue from 'cronstrue';

type FrequencyValue = 'once' | 'every' | 'daily' | 'weekly' | 'monthly';

type CustomIntervalUnit = 'minute' | 'hour' | 'day';

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
  { value: 'every', label: 'Every...' },
  { value: 'daily', label: 'Daily (at specific time)' },
  { value: 'weekly', label: 'Weekly (at specific time/days)' },
  { value: 'monthly', label: 'Monthly (at specific time/day)' },
];

const customIntervalUnits: { value: CustomIntervalUnit; label: string }[] = [
  { value: 'minute', label: 'minute(s)' },
  { value: 'hour', label: 'hour(s)' },
  { value: 'day', label: 'day(s)' },
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
  if (parts.length !== 5 && parts.length !== 6) return null;

  // Handle both 5-field and 6-field cron expressions
  const [minutes, hours, dayOfMonth, month, dayOfWeek] =
    parts.length === 5 ? parts : parts.slice(1); // Skip seconds if present

  // Check for custom intervals (every X minutes/hours/days)
  if (
    minutes.startsWith('*/') &&
    hours === '*' &&
    dayOfMonth === '*' &&
    month === '*' &&
    dayOfWeek === '*'
  ) {
    const intervalValue = parseInt(minutes.substring(2));
    return {
      frequency: 'every' as FrequencyValue,
      customIntervalValue: intervalValue,
      customIntervalUnit: 'minute' as CustomIntervalUnit,
    };
  }
  if (
    minutes === '0' &&
    hours.startsWith('*/') &&
    dayOfMonth === '*' &&
    month === '*' &&
    dayOfWeek === '*'
  ) {
    const intervalValue = parseInt(hours.substring(2));
    return {
      frequency: 'every' as FrequencyValue,
      customIntervalValue: intervalValue,
      customIntervalUnit: 'hour' as CustomIntervalUnit,
    };
  }
  if (
    minutes === '0' &&
    hours === '0' &&
    dayOfMonth.startsWith('*/') &&
    month === '*' &&
    dayOfWeek === '*'
  ) {
    const intervalValue = parseInt(dayOfMonth.substring(2));
    return {
      frequency: 'every' as FrequencyValue,
      customIntervalValue: intervalValue,
      customIntervalUnit: 'day' as CustomIntervalUnit,
    };
  }

  // Check for specific patterns
  if (dayOfMonth !== '*' && month !== '*' && dayOfWeek === '*') {
    return { frequency: 'once' as FrequencyValue, minutes, hours, dayOfMonth, month };
  }
  if (
    minutes !== '*' &&
    hours !== '*' &&
    dayOfMonth === '*' &&
    month === '*' &&
    dayOfWeek === '*'
  ) {
    return { frequency: 'daily' as FrequencyValue, minutes, hours };
  }
  if (
    minutes !== '*' &&
    hours !== '*' &&
    dayOfMonth === '*' &&
    month === '*' &&
    dayOfWeek !== '*'
  ) {
    return { frequency: 'weekly' as FrequencyValue, minutes, hours, dayOfWeek };
  }
  if (
    minutes !== '*' &&
    hours !== '*' &&
    dayOfMonth !== '*' &&
    month === '*' &&
    dayOfWeek === '*'
  ) {
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
  const [customIntervalValue, setCustomIntervalValue] = useState<number>(1);
  const [customIntervalUnit, setCustomIntervalUnit] = useState<CustomIntervalUnit>('minute');
  const [selectedDate, setSelectedDate] = useState<string>(
    () => new Date().toISOString().split('T')[0]
  );
  const [selectedTime, setSelectedTime] = useState<string>('09:00');
  const [selectedMinute] = useState<string>('0');
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
            setSelectedTime(
              `${parsed.hours?.padStart(2, '0')}:${parsed.minutes?.padStart(2, '0')}`
            );
            break;
          case 'every':
            if (parsed.customIntervalValue) {
              setCustomIntervalValue(parsed.customIntervalValue);
            }
            if (parsed.customIntervalUnit) {
              setCustomIntervalUnit(parsed.customIntervalUnit);
            }
            break;
          case 'daily':
            setSelectedTime(
              `${parsed.hours?.padStart(2, '0')}:${parsed.minutes?.padStart(2, '0')}`
            );
            break;
          case 'weekly':
            setSelectedTime(
              `${parsed.hours?.padStart(2, '0')}:${parsed.minutes?.padStart(2, '0')}`
            );
            if (parsed.dayOfWeek) {
              const days = parsed.dayOfWeek.split(',').map((d) => d.trim());
              setSelectedDaysOfWeek(new Set(days));
            }
            break;
          case 'monthly':
            setSelectedTime(
              `${parsed.hours?.padStart(2, '0')}:${parsed.minutes?.padStart(2, '0')}`
            );
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
      switch (frequency) {
        case 'once':
          if (selectedDate && selectedTime) {
            try {
              const dateObj = new Date(`${selectedDate}T${selectedTime}`);
              if (isNaN(dateObj.getTime())) return "Invalid date/time for 'once'.";
              return `${dateObj.getMinutes()} ${dateObj.getHours()} ${dateObj.getDate()} ${
                dateObj.getMonth() + 1
              } *`;
            } catch (e) {
              return "Error parsing date/time for 'once'.";
            }
          }
          return 'Date and Time are required for "Once" frequency.';
        case 'every': {
          if (customIntervalValue <= 0) {
            return 'Custom interval value must be greater than 0.';
          }
          switch (customIntervalUnit) {
            case 'minute':
              return `*/${customIntervalValue} * * * *`;
            case 'hour':
              return `0 */${customIntervalValue} * * *`;
            case 'day':
              return `0 0 */${customIntervalValue} * *`;
            default:
              return 'Invalid custom interval unit.';
          }
        }
        case 'daily':
          return `${minutePart} ${hourPart} * * *`;
        case 'weekly': {
          if (selectedDaysOfWeek.size === 0) {
            return 'Select at least one day for weekly frequency.';
          }
          const days = Array.from(selectedDaysOfWeek)
            .sort((a, b) => parseInt(a) - parseInt(b))
            .join(',');
          return `${minutePart} ${hourPart} * * ${days}`;
        }
        case 'monthly': {
          const sDayOfMonth = parseInt(selectedDayOfMonth, 10);
          if (isNaN(sDayOfMonth) || sDayOfMonth < 1 || sDayOfMonth > 31) {
            return 'Invalid day of month (1-31) for monthly frequency.';
          }
          return `${minutePart} ${hourPart} ${sDayOfMonth} * *`;
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
    customIntervalValue,
    customIntervalUnit,
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
    <div className="fixed inset-0 bg-black/50 z-40 flex items-center justify-center p-4">
      <Card className="w-full max-w-md bg-background-default shadow-xl rounded-lg z-50 flex flex-col max-h-[90vh] overflow-hidden">
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
              onChange={(newValue: unknown) => {
                const selectedOption = newValue as FrequencyOption | null;
                if (selectedOption) setFrequency(selectedOption.value);
              }}
              placeholder="Select frequency..."
            />
          </div>

          {frequency === 'every' && (
            <div className="grid grid-cols-2 gap-4">
              <div>
                <label htmlFor="customIntervalValue-modal" className={modalLabelClassName}>
                  Every:
                </label>
                <Input
                  type="number"
                  id="customIntervalValue-modal"
                  min="1"
                  max="999"
                  value={customIntervalValue}
                  onChange={(e) => setCustomIntervalValue(parseInt(e.target.value) || 1)}
                  required
                />
              </div>
              <div>
                <label htmlFor="customIntervalUnit-modal" className={modalLabelClassName}>
                  Unit:
                </label>
                <Select
                  instanceId="custom-interval-unit-select-modal"
                  options={customIntervalUnits}
                  value={customIntervalUnits.find((u) => u.value === customIntervalUnit)}
                  onChange={(newValue: unknown) => {
                    const selectedUnit = newValue as {
                      value: CustomIntervalUnit;
                      label: string;
                    } | null;
                    if (selectedUnit) setCustomIntervalUnit(selectedUnit.value);
                  }}
                  placeholder="Select unit..."
                />
              </div>
            </div>
          )}

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
            <p className={cronPreviewTextColor}>
              Syntax: M H D M DoW (M=minute, H=hour, D=day, M=month, DoW=day of week: 0/7=Sun)
            </p>
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
            variant="ghost"
            disabled={isLoadingExternally}
            className="w-full h-[60px] rounded-none border-t text-gray-900 dark:text-white hover:bg-gray-50 dark:border-gray-600 text-lg font-medium"
          >
            {isLoadingExternally ? 'Updating...' : 'Update Schedule'}
          </Button>
        </div>
      </Card>
    </div>
  );
};
