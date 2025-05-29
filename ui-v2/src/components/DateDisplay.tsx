import { useEffect, useState, ReactElement } from 'react';

import { useTimeline } from '../contexts/TimelineContext';

export function DateDisplay(): ReactElement {
  const { currentDate } = useTimeline();
  const [displayDate, setDisplayDate] = useState(currentDate);
  const [isFlipping, setIsFlipping] = useState(false);

  useEffect(() => {
    setIsFlipping(true);
    const timer = setTimeout(() => {
      setDisplayDate(currentDate);
      setIsFlipping(false);
    }, 50); // Reduced from 100ms to 50ms for faster flip

    return () => clearTimeout(timer);
  }, [currentDate]);

  const formatDate = (date: Date) => {
    const monthNames = [
      'January',
      'February',
      'March',
      'April',
      'May',
      'June',
      'July',
      'August',
      'September',
      'October',
      'November',
      'December',
    ];
    const dayNames = ['Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday'];

    return {
      month: monthNames[date.getMonth()],
      day: date.getDate(),
      weekday: dayNames[date.getDay()],
    };
  };

  const formattedDate = formatDate(displayDate);

  return (
    <div className="fixed top-[2px] left-1/2 -translate-x-1/2 z-40">
      <div
        className={`
          flex items-center gap-2 px-4 py-2
          text-text-default dark:text-white/70
          transition-all duration-150
          ${isFlipping ? 'transform -translate-y-1 opacity-0' : 'transform translate-y-0 opacity-100'}
        `}
      >
        {formattedDate.weekday} {formattedDate.month} {formattedDate.day}
      </div>
    </div>
  );
}
