"use client";

import * as React from "react";
import { Calendar as CalendarIcon, ChevronDownIcon } from "lucide-react";
import { format } from "date-fns";
import { DateRange } from "react-day-picker";

import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Calendar } from "@/components/ui/calendar";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

interface DateRangePickerProps {
  date?: DateRange;
  onDateChange?: (date: DateRange | undefined) => void;
  className?: string;
  placeholder?: string;
}

interface PresetRange {
  label: string;
  getValue: () => DateRange;
}

const PRESET_RANGES: PresetRange[] = [
  {
    label: "Past 1 min",
    getValue: () => {
      const now = new Date();
      const from = new Date(now.getTime() - 1 * 60 * 1000);
      return { from, to: now };
    },
  },
  {
    label: "Past 30 min",
    getValue: () => {
      const now = new Date();
      const from = new Date(now.getTime() - 30 * 60 * 1000);
      return { from, to: now };
    },
  },
  {
    label: "Past 1 hour",
    getValue: () => {
      const now = new Date();
      const from = new Date(now.getTime() - 60 * 60 * 1000);
      return { from, to: now };
    },
  },
  {
    label: "Past 6 hours",
    getValue: () => {
      const now = new Date();
      const from = new Date(now.getTime() - 6 * 60 * 60 * 1000);
      return { from, to: now };
    },
  },
  {
    label: "Past 12 hours",
    getValue: () => {
      const now = new Date();
      const from = new Date(now.getTime() - 12 * 60 * 60 * 1000);
      return { from, to: now };
    },
  },
  {
    label: "Past 1 day",
    getValue: () => {
      const now = new Date();
      const from = new Date(now.getTime() - 24 * 60 * 60 * 1000);
      return { from, to: now };
    },
  },
  {
    label: "Past 3 days",
    getValue: () => {
      const now = new Date();
      const from = new Date(now.getTime() - 3 * 24 * 60 * 60 * 1000);
      return { from, to: now };
    },
  },
  {
    label: "Past 7 days",
    getValue: () => {
      const now = new Date();
      const from = new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000);
      return { from, to: now };
    },
  },
];

export function DateRangePicker({
  date,
  onDateChange,
  className,
  placeholder = "Pick a date range",
}: DateRangePickerProps) {
  const [open, setOpen] = React.useState(false);
  const [fromTime, setFromTime] = React.useState("00:00:00");
  const [toTime, setToTime] = React.useState("23:59:59");

  const handlePresetClick = (preset: PresetRange) => {
    const range = preset.getValue();

    // Extract times from the preset dates
    if (range.from) {
      const hours = range.from.getHours().toString().padStart(2, "0");
      const minutes = range.from.getMinutes().toString().padStart(2, "0");
      const seconds = range.from.getSeconds().toString().padStart(2, "0");
      setFromTime(`${hours}:${minutes}:${seconds}`);
    }

    if (range.to) {
      const hours = range.to.getHours().toString().padStart(2, "0");
      const minutes = range.to.getMinutes().toString().padStart(2, "0");
      const seconds = range.to.getSeconds().toString().padStart(2, "0");
      setToTime(`${hours}:${minutes}:${seconds}`);
    }

    onDateChange?.(range);
  };

  const getTimezone = () => {
    const timezone = Intl.DateTimeFormat().resolvedOptions().timeZone;
    const offset = new Date().toTimeString().match(/GMT([+-]\d{4})/)?.[1];
    return `${timezone} (UTC${offset?.slice(0, 3)}:${offset?.slice(3)})`;
  };

  const setTimeOnDate = (date: Date, time: string) => {
    const [hours, minutes, seconds] = time.split(":").map(Number);
    const newDate = new Date(date);
    newDate.setHours(hours, minutes, seconds || 0, 0);
    return newDate;
  };

  const handleDateSelect = (selectedDate: DateRange | undefined) => {
    if (!selectedDate) {
      onDateChange?.(undefined);
      return;
    }

    const newDate: DateRange = {
      from: selectedDate.from ? setTimeOnDate(selectedDate.from, fromTime) : undefined,
      to: selectedDate.to ? setTimeOnDate(selectedDate.to, toTime) : undefined,
    };

    onDateChange?.(newDate);
  };

  const handleTimeChange = (type: "from" | "to", time: string) => {
    if (type === "from") {
      setFromTime(time);
      if (date?.from) {
        const newDate = { ...date, from: setTimeOnDate(date.from, time) };
        onDateChange?.(newDate);
      }
    } else {
      setToTime(time);
      if (date?.to) {
        const newDate = { ...date, to: setTimeOnDate(date.to, time) };
        onDateChange?.(newDate);
      }
    }
  };

  const formatDisplay = (d: Date) => {
    return format(d, "MMM d, yyyy HH:mm:ss");
  };

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger asChild>
        <Button
          variant="outline"
          className={cn(
            "h-9 justify-between font-normal",
            !date && "text-muted-foreground",
            className
          )}
        >
          <div className="flex items-center">
            <CalendarIcon className="mr-2 h-4 w-4" />
            {date?.from ? (
              date.to ? (
                <>
                  {formatDisplay(date.from)} - {formatDisplay(date.to)}
                </>
              ) : (
                formatDisplay(date.from)
              )
            ) : (
              <span>{placeholder}</span>
            )}
          </div>
          <ChevronDownIcon className="ml-2 h-4 w-4" />
        </Button>
      </PopoverTrigger>
      <PopoverContent className="w-auto overflow-hidden p-0" align="start">
        <div className="flex">
          {/* Preset options sidebar - Section 1 */}
          <div className="flex w-[140px] flex-col border-r">
            <div className="border-b p-3 text-sm font-semibold">Quick select</div>
            <div className="flex flex-col gap-1 p-2">
              {PRESET_RANGES.map((preset) => (
                <Button
                  key={preset.label}
                  variant="ghost"
                  size="sm"
                  onClick={() => handlePresetClick(preset)}
                  className="h-8 justify-start px-2 text-xs font-normal hover:bg-accent"
                >
                  {preset.label}
                </Button>
              ))}
            </div>
          </div>

          {/* Right side - Section 2 & 3 */}
          <div className="flex flex-col">
            {/* Calendar - Section 2 */}
            <Calendar
              mode="range"
              defaultMonth={date?.from}
              selected={date}
              captionLayout="dropdown"
              onSelect={handleDateSelect}
              numberOfMonths={2}
            />

            {/* Time inputs - Section 3 */}
            <div className="border-t p-3">
              <div className="flex justify-between gap-4">
                <div className="flex flex-col gap-2">
                  <Label className="text-xs text-muted-foreground">From time</Label>
                  <Input
                    type="time"
                    step="1"
                    value={fromTime}
                    onChange={(e) => handleTimeChange("from", e.target.value)}
                    className="h-8 appearance-none [&::-webkit-calendar-picker-indicator]:hidden [&::-webkit-calendar-picker-indicator]:appearance-none"
                  />
                </div>
                <div className="flex flex-col gap-2">
                  <Label className="text-xs text-muted-foreground">To time</Label>
                  <Input
                    type="time"
                    step="1"
                    value={toTime}
                    onChange={(e) => handleTimeChange("to", e.target.value)}
                    className="h-8  appearance-none [&::-webkit-calendar-picker-indicator]:hidden [&::-webkit-calendar-picker-indicator]:appearance-none"
                  />
                </div>
              </div>
              <div className="mb-2 text-center text-xs text-muted-foreground">{getTimezone()}</div>
            </div>
          </div>
        </div>
      </PopoverContent>
    </Popover>
  );
}
