import { useTimeline } from '../components/TimelineContext';

interface TimelineStyles {
  isPastDate: boolean;
  greetingCardStyle: {
    background: string;
    text: string;
  };
  contentCardStyle: string;
}

export function useTimelineStyles(date?: Date): TimelineStyles {
  const { isCurrentDate } = useTimeline();
  const isPastDate = date ? date < new Date() && !isCurrentDate(date) : false;

  // Content cards match the Tasks Completed tile styling
  const contentCardStyle =
    'bg-white dark:bg-[#121212] shadow-[0_0_13.7px_rgba(0,0,0,0.04)] dark:shadow-[0_0_24px_rgba(255,255,255,0.02)]';

  // Greeting card styles based on date
  const greetingCardStyle = !isPastDate
    ? {
        background: 'bg-textStandard', // Black background
        text: 'text-white', // White text
      }
    : {
        background: 'bg-gray-100', // Light grey background
        text: 'text-gray-600', // Darker grey text
      };

  return {
    isPastDate,
    greetingCardStyle,
    contentCardStyle,
  };
}
