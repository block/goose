import React, { useState, useEffect, useCallback } from 'react';
import { Input } from '../ui/input';
import { Select } from '../ui/Select';
import cronstrue from 'cronstrue';

export type FrequencyValue = 'once' | 'every' | 'daily' | 'weekly' | 'monthly';
export type CustomIntervalUnit = 'minute' | 'hour' | 'day';

interface FrequencyOption {
  value: FrequencyValue;
  label: string;
}

export interface CronExpressionBuilderProps {
  value?: string;
  onChange: (cron: string, readable: string, isValid: boolean) => void;
  className?: string;
  defaultFrequency?: FrequencyValue;
  defaultTime?: string;
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

const labelClassName = 'block text-sm font-medium text-text-prominent mb-1';
const cronPreviewTextColor = 'text-xs text-text-subtle mt-1';
const cronPreviewWarningColor = 'text-xs text-text-warning mt-1';
const checkboxLabelClassName = 'flex items-center text-sm text-text-default';
const checkboxInputClassName =
  'h-4 w-4 text-accent-default border-border-subtle rounded focus:ring-accent-default mr-2';

export const CronExpressionBuilder: React.FC<CronExpressionBuilderProps> = ({
  value,
  onChange,
  className = '',
  defaultFrequency = 'daily',
  defaultTime = '09:00',
}) => {
  const [frequency, setFrequency] = useState<FrequencyValue>(defaultFrequency);
  const [customIntervalValue, setCustomIntervalValue] = useState<number>(1);
  const [customIntervalUnit, setCustomIntervalUnit] = useState<CustomIntervalUnit>('minute');
  const [selectedDate, setSelectedDate] = useState<string>(
    new Date(Date.now() + 24 * 60 * 60 * 1000).toISOString().split('T')[0]
  );
  const [selectedTime, setSelectedTime] = useState<string>(defaultTime);
  const [selectedDaysOfWeek, setSelectedDaysOfWeek] = useState<Set<string>>(new Set(['1']));
  const [selectedDayOfMonth, setSelectedDayOfMonth] = useState<string>('1');
  const [derivedCronExpression, setDerivedCronExpression] = useState<string>('');
  const [readableCronExpression, setReadableCronExpression] = useState<string>('');

  const generateCronExpression = useCallback((): string => {
    const timeParts = selectedTime.split(':');
    const minutePart = timeParts.length > 1 ? String(parseInt(timeParts[1], 10)) : '0';
    const hourPart = timeParts.length > 0 ? String(parseInt(timeParts[0], 10)) : '0';
    if (isNaN(parseInt(minutePart)) || isNaN(parseInt(hourPart))) {
      return 'Invalid time format.';
    }

    // Temporal uses 5-field cron: minute hour day month dayofweek (no seconds)
    switch (frequency) {
      case 'once':
        if (selectedDate && selectedTime) {
          try {
            const dateObj = new Date(`${selectedDate}T${selectedTime}`);
            if (isNaN(dateObj.getTime())) return "Invalid date/time for 'once'.";
            return `${dateObj.getMinutes()} ${dateObj.getHours()} ${dateObj.getDate()} ${
              dateObj.getMonth() + 1
            } *`;
          } catch {
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
  }, [
    frequency,
    customIntervalValue,
    customIntervalUnit,
    selectedDate,
    selectedTime,
    selectedDaysOfWeek,
    selectedDayOfMonth,
  ]);

  // Update cron expression when dependencies change
  useEffect(() => {
    const cron = generateCronExpression();
    setDerivedCronExpression(cron);

    let readable = '';
    let isValid = true;

    try {
      if (
        cron.includes('Invalid') ||
        cron.includes('required') ||
        cron.includes('Error') ||
        cron.includes('Select at least one')
      ) {
        readable = 'Invalid cron details provided.';
        isValid = false;
      } else {
        readable = cronstrue.toString(cron);
      }
    } catch {
      readable = 'Could not parse cron string.';
      isValid = false;
    }

    setReadableCronExpression(readable);
    onChange(cron, readable, isValid);
  }, [
    frequency,
    customIntervalValue,
    customIntervalUnit,
    selectedDate,
    selectedTime,
    selectedDaysOfWeek,
    selectedDayOfMonth,
    generateCronExpression,
    onChange,
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

  return (
    <div className={`space-y-4 ${className}`}>
      {/* Frequency Selector */}
      <div>
        <label htmlFor="frequency-select" className={labelClassName}>
          Frequency:
        </label>
        <Select
          id="frequency-select"
          value={frequency}
          onChange={(e) => setFrequency(e.target.value as FrequencyValue)}
        >
          {frequencies.map((freq) => (
            <option key={freq.value} value={freq.value}>
              {freq.label}
            </option>
          ))}
        </Select>
      </div>

      {/* Once - Date and Time */}
      {frequency === 'once' && (
        <>
          <div>
            <label htmlFor="once-date" className={labelClassName}>
              Date:
            </label>
            <Input
              type="date"
              id="once-date"
              value={selectedDate}
              onChange={(e) => setSelectedDate(e.target.value)}
              min={new Date().toISOString().split('T')[0]}
            />
          </div>
          <div>
            <label htmlFor="once-time" className={labelClassName}>
              Time:
            </label>
            <Input
              type="time"
              id="once-time"
              value={selectedTime}
              onChange={(e) => setSelectedTime(e.target.value)}
            />
          </div>
        </>
      )}

      {/* Every - Custom Interval */}
      {frequency === 'every' && (
        <div className="flex gap-2">
          <div className="flex-1">
            <label htmlFor="interval-value" className={labelClassName}>
              Every:
            </label>
            <Input
              type="number"
              id="interval-value"
              value={customIntervalValue}
              onChange={(e) => setCustomIntervalValue(parseInt(e.target.value, 10) || 1)}
              min={1}
            />
          </div>
          <div className="flex-1">
            <label htmlFor="interval-unit" className={labelClassName}>
              Unit:
            </label>
            <Select
              id="interval-unit"
              value={customIntervalUnit}
              onChange={(e) => setCustomIntervalUnit(e.target.value as CustomIntervalUnit)}
            >
              {customIntervalUnits.map((unit) => (
                <option key={unit.value} value={unit.value}>
                  {unit.label}
                </option>
              ))}
            </Select>
          </div>
        </div>
      )}

      {/* Daily - Time */}
      {frequency === 'daily' && (
        <div>
          <label htmlFor="daily-time" className={labelClassName}>
            Time:
          </label>
          <Input
            type="time"
            id="daily-time"
            value={selectedTime}
            onChange={(e) => setSelectedTime(e.target.value)}
          />
        </div>
      )}

      {/* Weekly - Days and Time */}
      {frequency === 'weekly' && (
        <>
          <div>
            <label className={labelClassName}>Days of Week:</label>
            <div className="flex flex-wrap gap-2 mt-2">
              {daysOfWeekOptions.map((day) => (
                <label key={day.value} className={checkboxLabelClassName}>
                  <input
                    type="checkbox"
                    className={checkboxInputClassName}
                    checked={selectedDaysOfWeek.has(day.value)}
                    onChange={() => handleDayOfWeekChange(day.value)}
                  />
                  {day.label}
                </label>
              ))}
            </div>
          </div>
          <div>
            <label htmlFor="weekly-time" className={labelClassName}>
              Time:
            </label>
            <Input
              type="time"
              id="weekly-time"
              value={selectedTime}
              onChange={(e) => setSelectedTime(e.target.value)}
            />
          </div>
        </>
      )}

      {/* Monthly - Day and Time */}
      {frequency === 'monthly' && (
        <>
          <div>
            <label htmlFor="monthly-day" className={labelClassName}>
              Day of Month (1-31):
            </label>
            <Input
              type="number"
              id="monthly-day"
              value={selectedDayOfMonth}
              onChange={(e) => setSelectedDayOfMonth(e.target.value)}
              min={1}
              max={31}
            />
          </div>
          <div>
            <label htmlFor="monthly-time" className={labelClassName}>
              Time:
            </label>
            <Input
              type="time"
              id="monthly-time"
              value={selectedTime}
              onChange={(e) => setSelectedTime(e.target.value)}
            />
          </div>
        </>
      )}

      {/* Cron Preview */}
      <div className="border-t border-border-subtle pt-3">
        <p className="text-sm font-medium text-text-prominent mb-1">Preview:</p>
        <p
          className={
            readableCronExpression.includes('Invalid') ||
            readableCronExpression.includes('Could not parse')
              ? cronPreviewWarningColor
              : cronPreviewTextColor
          }
        >
          {readableCronExpression || 'Generating preview...'}
        </p>
        {derivedCronExpression && !derivedCronExpression.includes('Invalid') && (
          <p className={cronPreviewTextColor}>Cron: {derivedCronExpression}</p>
        )}
      </div>
    </div>
  );
};

export default CronExpressionBuilder;
