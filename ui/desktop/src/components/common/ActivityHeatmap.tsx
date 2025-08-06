import { useEffect, useState } from 'react';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '../ui/Tooltip';
import { getApiUrl } from '../../config';

interface ActivityHeatmapCell {
  week: number;
  day: number;
  count: number;
  date?: string; // Add date for better display in tooltips
}

// Days of the week for labeling
const DAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
// Number of weeks in a year
const WEEKS_IN_YEAR = 52;

export function ActivityHeatmap() {
  const [heatmapData, setHeatmapData] = useState<ActivityHeatmapCell[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [currentYear] = useState(new Date().getFullYear());

  // Calculate the intensity for coloring cells
  const getColorIntensity = (count: number, maxCount: number) => {
    if (count === 0) return 'bg-background-muted/30';

    const normalizedCount = count / maxCount;

    if (normalizedCount < 0.25) return 'bg-background-accent/20';
    if (normalizedCount < 0.5) return 'bg-background-accent/40';
    if (normalizedCount < 0.75) return 'bg-background-accent/60';
    return 'bg-background-accent/80';
  };

  useEffect(() => {
    const fetchHeatmapData = async () => {
      try {
        setLoading(true);
        const response = await fetch(getApiUrl('/sessions/activity-heatmap'), {
          headers: {
            Accept: 'application/json',
            'Content-Type': 'application/json',
            'X-Secret-Key': await window.electron.getSecretKey(),
          },
        });

        if (!response.ok) {
          throw new Error(`Failed to fetch heatmap data: ${response.status}`);
        }

        const data = await response.json();
        setHeatmapData(data);
        setError(null);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to load heatmap data');
      } finally {
        setLoading(false);
      }
    };

    fetchHeatmapData();
  }, []);

  // Find the maximum count for scaling
  const maxCount = Math.max(
    1, // Avoid division by zero
    ...heatmapData.map((cell) => cell.count)
  );

  // Create a calendar grid from Jan 1st of current year to today
  const prepareGridData = () => {
    // Get current date
    const now = new Date();
    const startOfYear = new Date(currentYear, 0, 1); // Jan 1st of current year

    // Calculate weeks to display - now showing full year (52 weeks)
    // const weeksToDisplay = Math.ceil((daysSinceStartOfYear + getStartDayOfYear()) / 7);
    const weeksToDisplay = WEEKS_IN_YEAR;

    // Create a map to lookup counts easily
    const dataMap = new Map<string, number>();
    heatmapData.forEach((cell) => {
      dataMap.set(`${cell.week}-${cell.day}`, cell.count);
    });

    // Build the grid
    const grid = [];

    // Fill grid with dates and activity data
    for (let week = 0; week < weeksToDisplay; week++) {
      const weekCells = [];

      for (let day = 0; day < 7; day++) {
        // Convert week and day to a real date
        const cellDate = new Date(startOfYear);
        cellDate.setDate(cellDate.getDate() + week * 7 + day - getStartDayOfYear());

        // Only include dates up to today for real data
        const isFuture = cellDate > now;

        // Format the date string
        const dateStr = cellDate.toLocaleDateString(undefined, {
          month: 'short',
          day: 'numeric',
        });

        // Get count from data if available
        let count = 0;

        // Try to find a matching date in our data
        // This requires matching the specific week number (from ISO week) and day
        if (!isFuture) {
          for (const cell of heatmapData) {
            if (cell.week === getWeekNumber(cellDate) && cell.day === day) {
              count = cell.count;
              break;
            }
          }
        }

        weekCells.push({
          week,
          day,
          count,
          date: dateStr,
        });
      }

      grid.push(weekCells);
    }

    return grid;
  };

  // Helper to get day of week (0-6) of Jan 1st for current year
  const getStartDayOfYear = () => {
    return new Date(currentYear, 0, 1).getDay();
  };

  // Get ISO week number for a date
  const getWeekNumber = (date: Date) => {
    const d = new Date(date);
    d.setHours(0, 0, 0, 0);
    d.setDate(d.getDate() + 3 - ((d.getDay() + 6) % 7));
    const week1 = new Date(d.getFullYear(), 0, 4);
    return (
      1 +
      Math.round(((d.getTime() - week1.getTime()) / 86400000 - 3 + ((week1.getDay() + 6) % 7)) / 7)
    );
  };

  const grid = prepareGridData();

  if (loading) {
    return (
      <div className="h-[120px] flex items-center justify-center">
        <div className="text-text-muted">Loading activity data...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="h-[120px] flex items-center justify-center">
        <div className="text-red-500">Error loading activity data</div>
      </div>
    );
  }

  // Get month labels - now showing all months
  const getMonthLabels = () => {
    const allMonths = [
      'Jan',
      'Feb',
      'Mar',
      'Apr',
      'May',
      'Jun',
      'Jul',
      'Aug',
      'Sep',
      'Oct',
      'Nov',
      'Dec',
    ];

    return allMonths.map((month, i) => {
      // Calculate position based on days in month and start day of year
      const monthIndex = i;
      const daysBeforeMonth = getDaysBeforeMonth(monthIndex);
      const position = (daysBeforeMonth - getStartDayOfYear()) / 7;

      return (
        <div
          key={month}
          className="text-[10px] text-text-muted absolute"
          style={{
            left: `${(position / WEEKS_IN_YEAR) * 100}%`,
            transform: 'translateX(-50%)',
          }}
        >
          {month}
        </div>
      );
    });
  };

  // Helper to calculate days before a month in current year
  const getDaysBeforeMonth = (monthIndex: number) => {
    const days = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    // Adjust for leap year
    if (monthIndex > 1 && isLeapYear(currentYear)) {
      return days[monthIndex] + 1;
    }
    return days[monthIndex];
  };

  // Helper to check if year is a leap year
  const isLeapYear = (year: number) => {
    return (year % 4 === 0 && year % 100 !== 0) || year % 400 === 0;
  };

  return (
    <div className="w-full px-4">
      {/* Month labels */}
      <div className="relative h-4 ml-12 mb-2 mr-4">{getMonthLabels()}</div>

      <div className="flex w-full">
        {/* Day labels - now right-aligned */}
        <div className="flex flex-col pt-1 pr-2 w-10">
          {DAYS.map((day, index) => (
            <div
              key={day}
              className="h-3 text-[10px] text-text-muted flex items-center justify-end"
            >
              {index % 2 === 0 ? day : ''}
            </div>
          ))}
        </div>

        {/* Grid - with smaller squares */}
        <div className="flex gap-[1px] flex-1 mr-4">
          {grid.map((week, weekIndex) => (
            <div
              key={weekIndex}
              className="flex flex-col gap-[1px] flex-1"
              style={{ maxWidth: `calc(100% / ${WEEKS_IN_YEAR})` }}
            >
              {week.map((cell) => (
                <TooltipProvider key={`${cell.week}-${cell.day}`}>
                  <Tooltip>
                    <TooltipTrigger asChild>
                      <div
                        className={`aspect-square w-full h-2 rounded-[1px] ${getColorIntensity(cell.count, maxCount)}`}
                        role="gridcell"
                      />
                    </TooltipTrigger>
                    <TooltipContent side="top">
                      {cell.date ? (
                        <p className="text-xs">
                          {cell.count} sessions on {cell.date}
                        </p>
                      ) : (
                        <p className="text-xs">No data</p>
                      )}
                    </TooltipContent>
                  </Tooltip>
                </TooltipProvider>
              ))}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
