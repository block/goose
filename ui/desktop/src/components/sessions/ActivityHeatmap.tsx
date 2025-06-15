import React, { useEffect, useState } from 'react';

interface ActivityHeatmapCell {
  week: number;
  day: number;
  count: number;
}

const DAYS = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
const NUM_WEEKS = 9; // Show only the last two months (approx. 9 weeks)

const getColorIntensity = (count: number, maxCount: number) => {
  if (count === 0) return 'bg-gray-100';
  if (count <= maxCount * 0.25) return 'bg-green-200';
  if (count <= maxCount * 0.5) return 'bg-green-400';
  if (count <= maxCount * 0.75) return 'bg-green-600';
  return 'bg-green-800';
};

export const ActivityHeatmap: React.FC = () => {
  const [data, setData] = useState<ActivityHeatmapCell[] | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetch('/sessions/activity-heatmap')
      .then((res) => res.json())
      .then((json) => {
        setData(json);
        setLoading(false);
      })
      .catch(() => {
        setData([]);
        setLoading(false);
      });
  }, []);

  if (loading) {
    return <div className="p-4 bg-white rounded-lg shadow">Loading activity heatmap...</div>;
  }

  const safeData = data || [];
  const maxCount = Math.max(0, ...safeData.map((d) => d.count));

  // Build a 2D array: [day][week] = count
  const heatmapGrid: number[][] = Array(7)
    .fill(null)
    .map(() => Array(NUM_WEEKS).fill(0));
  safeData.forEach(({ week, day, count }) => {
    // Only include the last NUM_WEEKS
    if (
      week >= 0 &&
      week < 52 &&
      day >= 0 &&
      day < 7 &&
      week >= 52 - NUM_WEEKS // Only last NUM_WEEKS
    ) {
      heatmapGrid[day][week - (52 - NUM_WEEKS)] = count;
    }
  });

  return (
    <>
      <div className="flex">
        {/* Day labels */}
        <div className="flex flex-col mr-2">
          {DAYS.map((day, i) => (
            <div key={day} className="h-4 flex items-center text-xs text-gray-600">
              {['Mon', 'Wed', 'Fri'].includes(day) ? day : ''}
            </div>
          ))}
        </div>
        {/* Heatmap grid */}
        <div className="flex-1 overflow-x-auto">
          <div className="grid grid-rows-7 grid-cols-9 gap-0.5">
            {DAYS.map((day, dayIndex) => (
              <React.Fragment key={dayIndex}>
                {Array.from({ length: NUM_WEEKS }).map((_, weekIndex) => (
                  <div
                    key={`${dayIndex}-${weekIndex}`}
                    className={`w-4 h-4 rounded-full ${getColorIntensity(heatmapGrid[dayIndex][weekIndex], maxCount)}`}
                    title={`${DAYS[dayIndex]} Week ${weekIndex + 1} - ${heatmapGrid[dayIndex][weekIndex]} sessions`}
                  />
                ))}
              </React.Fragment>
            ))}
          </div>
        </div>
      </div>
      {/* Legend */}
      <div className="flex items-center justify-end mt-4 gap-2">
        <span className="text-xs text-gray-600">Less</span>
        <div className="flex gap-1">
          <div className="w-3 h-3 bg-green-200 rounded-sm" />
          <div className="w-3 h-3 bg-green-400 rounded-sm" />
          <div className="w-3 h-3 bg-green-600 rounded-sm" />
          <div className="w-3 h-3 bg-green-800 rounded-sm" />
        </div>
        <span className="text-xs text-gray-600">More</span>
      </div>
    </>
  );
};
