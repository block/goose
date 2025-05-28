import { useRef, useMemo, useEffect, ReactElement } from 'react';

import {
  ChartLineIcon,
  ChartBarIcon,
  PieChartIcon,
  ListIcon,
  StarIcon,
  TrendingUpIcon,
} from './icons';
import { useTimeline } from '../contexts/TimelineContext';
import ChartTile from './tiles/ChartTile.tsx';
import ClockTile from './tiles/ClockTile.tsx';
import HighlightTile from './tiles/HighlightTile.tsx';
import ListTile from './tiles/ListTile.tsx';
import PieChartTile from './tiles/PieChartTile.tsx';
import TimelineDots from './TimelineDots';

const generateRandomData = (length: number) =>
  Array.from({ length }, () => Math.floor(Math.random() * 100));

const generateTileData = (date: Date) => {
  const isToday = new Date().toDateString() === date.toDateString();

  return {
    left: [
      // Performance metrics
      {
        type: 'chart' as const,
        props: {
          title: 'Daily Activity',
          value: '487',
          trend: '↑ 12%',
          data: generateRandomData(7),
          icon: <ChartLineIcon />,
          variant: 'line' as const,
          date,
        },
      },
      {
        type: 'highlight' as const,
        props: {
          title: 'Achievement',
          value: isToday ? 'New Record!' : 'Great Work',
          icon: <StarIcon />,
          subtitle: isToday ? 'Personal best today' : 'Keep it up',
          date,
          accentColor: '#FFB800',
        },
      },
      {
        type: 'pie' as const,
        props: {
          title: 'Task Distribution',
          icon: <PieChartIcon />,
          segments: [
            { value: 45, color: '#00CAF7', label: 'Completed' },
            { value: 35, color: '#FFB800', label: 'In Progress' },
            { value: 20, color: '#FF4444', label: 'Pending' },
          ],
          date,
        },
      },
      // Additional metrics
      {
        type: 'chart' as const,
        props: {
          title: 'Response Time',
          value: '245ms',
          trend: '↓ 18%',
          data: generateRandomData(7),
          icon: <ChartBarIcon />,
          variant: 'bar' as const,
          date,
        },
      },
      {
        type: 'highlight' as const,
        props: {
          title: 'User Satisfaction',
          value: '98%',
          icon: <StarIcon />,
          subtitle: 'Based on feedback',
          date,
          accentColor: '#4CAF50',
        },
      },
      {
        type: 'list' as const,
        props: {
          title: 'Top Priorities',
          icon: <ListIcon />,
          items: [
            { text: 'Project Alpha', value: '87%', color: '#00CAF7' },
            { text: 'Team Meeting', value: '2:30 PM' },
            { text: 'Review Code', value: '13', color: '#FFB800' },
            { text: 'Deploy Update', value: 'Done', color: '#4CAF50' },
          ],
          date,
        },
      },
      // System metrics
      {
        type: 'chart' as const,
        props: {
          title: 'System Load',
          value: '42%',
          trend: '↑ 5%',
          data: generateRandomData(7),
          icon: <ChartLineIcon />,
          variant: 'line' as const,
          date,
        },
      },
      {
        type: 'pie' as const,
        props: {
          title: 'Storage Usage',
          icon: <PieChartIcon />,
          segments: [
            { value: 60, color: '#4CAF50', label: 'Free' },
            { value: 25, color: '#FFB800', label: 'Used' },
            { value: 15, color: '#FF4444', label: 'System' },
          ],
          date,
        },
      },
    ],
    right: [
      // Performance metrics
      {
        type: 'chart' as const,
        props: {
          title: 'Performance',
          value: '92%',
          trend: '↑ 8%',
          data: generateRandomData(7),
          icon: <ChartBarIcon />,
          variant: 'bar' as const,
          date,
        },
      },
      // Clock tile
      {
        type: 'clock' as const,
        props: {
          title: 'Current Time',
          date,
        },
      },
      {
        type: 'highlight' as const,
        props: {
          title: 'Efficiency',
          value: '+28%',
          icon: <TrendingUpIcon />,
          subtitle: 'Above target',
          date,
          accentColor: '#4CAF50',
        },
      },
      {
        type: 'pie' as const,
        props: {
          title: 'Resource Usage',
          icon: <PieChartIcon />,
          segments: [
            { value: 55, color: '#4CAF50', label: 'Available' },
            { value: 30, color: '#FFB800', label: 'In Use' },
            { value: 15, color: '#FF4444', label: 'Reserved' },
          ],
          date,
        },
      },
      // Updates and notifications
      {
        type: 'list' as const,
        props: {
          title: 'Recent Updates',
          icon: <ListIcon />,
          items: [
            { text: 'System Update', value: 'Complete', color: '#4CAF50' },
            { text: 'New Features', value: '3', color: '#00CAF7' },
            { text: 'Bug Fixes', value: '7', color: '#FFB800' },
            { text: 'Performance', value: '+15%', color: '#4CAF50' },
          ],
          date,
        },
      },
      // Additional metrics
      {
        type: 'chart' as const,
        props: {
          title: 'User Activity',
          value: '1,247',
          trend: '↑ 23%',
          data: generateRandomData(7),
          icon: <ChartLineIcon />,
          variant: 'line' as const,
          date,
        },
      },
      {
        type: 'highlight' as const,
        props: {
          title: 'New Users',
          value: '+156',
          icon: <TrendingUpIcon />,
          subtitle: 'Last 24 hours',
          date,
          accentColor: '#00CAF7',
        },
      },
      // System health
      {
        type: 'pie' as const,
        props: {
          title: 'API Health',
          icon: <PieChartIcon />,
          segments: [
            { value: 75, color: '#4CAF50', label: 'Healthy' },
            { value: 20, color: '#FFB800', label: 'Warning' },
            { value: 5, color: '#FF4444', label: 'Critical' },
          ],
          date,
        },
      },
      {
        type: 'list' as const,
        props: {
          title: 'System Status',
          icon: <ListIcon />,
          items: [
            { text: 'Main API', value: 'Online', color: '#4CAF50' },
            { text: 'Database', value: '98%', color: '#00CAF7' },
            { text: 'Cache', value: 'Synced', color: '#4CAF50' },
            { text: 'CDN', value: 'Active', color: '#4CAF50' },
          ],
          date,
        },
      },
    ],
  };
};

export default function Timeline(): ReactElement {
  const containerRef = useRef<HTMLDivElement>(null);
  const sectionRefs = useRef<(HTMLDivElement | null)[]>([]);
  const { setCurrentDate } = useTimeline();

  const sections = useMemo(() => {
    const result = [];
    const today = new Date();

    for (let i = 0; i <= 29; i++) {
      const date = new Date(today);
      date.setDate(today.getDate() - i);

      const tileData = generateTileData(date);

      result.push({
        date,
        isToday: i === 0,
        leftTiles: tileData.left,
        rightTiles: tileData.right,
      });
    }

    return result;
  }, []);

  // Function to center the timeline in a section
  const centerTimeline = (
    sectionElement: HTMLDivElement | null,
    animate: boolean = true
  ): HTMLDivElement | null => {
    if (!sectionElement) return sectionElement;

    requestAnimationFrame(() => {
      const totalWidth = sectionElement.scrollWidth;
      const viewportWidth = sectionElement.clientWidth;
      const scrollToX = Math.max(0, (totalWidth - viewportWidth) / 2);

      if (animate) {
        sectionElement.scrollTo({
          left: scrollToX,
          behavior: 'smooth',
        });
      } else {
        sectionElement.scrollLeft = scrollToX;
      }
    });

    return sectionElement;
  };

  useEffect(() => {
    // Capture ref values at the start of the effect
    const currentContainer = containerRef.current;
    const currentSections = [...sectionRefs.current];

    // Create the intersection observer
    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          const section = entry.target as HTMLDivElement;

          // When section comes into view
          if (entry.isIntersecting) {
            // Update current date
            const sectionIndex = sectionRefs.current.indexOf(section);
            if (sectionIndex !== -1 && sections[sectionIndex]) {
              const date = sections[sectionIndex].date;
              setCurrentDate(date);
            }
          }

          // When section is fully visible and centered
          if (entry.intersectionRatio > 0.8) {
            centerTimeline(section, true);
          }
        });
      },
      {
        threshold: [0, 0.8, 1], // Track when section is hidden, mostly visible, and fully visible
        rootMargin: '-10% 0px', // Slightly reduced margin for more natural triggering
      }
    );

    // Add scroll handler for even faster updates
    const handleScroll = () => {
      if (!currentContainer) return;

      // Find the section closest to the middle of the viewport
      const viewportMiddle = window.innerHeight / 2;
      let closestSection: HTMLDivElement | null = null;
      let closestDistance = Infinity;

      sectionRefs.current.forEach((section) => {
        if (!section) return;
        const rect = section.getBoundingClientRect();
        const sectionMiddle = rect.top + rect.height / 2;
        const distance = Math.abs(sectionMiddle - viewportMiddle);

        if (distance < closestDistance) {
          closestDistance = distance;
          closestSection = section;
        }
      });

      if (closestSection) {
        const sectionIndex = sectionRefs.current.indexOf(closestSection);
        if (sectionIndex !== -1 && sections[sectionIndex]) {
          const date = sections[sectionIndex].date;
          setCurrentDate(date);
        }
      }
    };

    // Add scroll event listener with throttling
    let lastScrollTime = 0;
    const throttledScrollHandler = () => {
      const now = Date.now();
      if (now - lastScrollTime >= 150) {
        // Throttle to ~6-7 times per second
        handleScroll();
        lastScrollTime = now;
      }
    };

    currentContainer?.addEventListener('scroll', throttledScrollHandler, { passive: true });

    // Add resize handler
    const handleResize = () => {
      // Find the currently visible section
      const visibleSection = sectionRefs.current.find((section) => {
        if (!section) return false;
        const rect = section.getBoundingClientRect();
        const viewportHeight = window.innerHeight;
        return rect.top >= -viewportHeight / 2 && rect.bottom <= viewportHeight * 1.5;
      });

      if (visibleSection) {
        centerTimeline(visibleSection, true); // Animate on resize
      }
    };

    // Add resize event listener
    window.addEventListener('resize', handleResize);

    // Observe all sections
    sectionRefs.current.forEach((section) => {
      if (section) {
        observer.observe(section);
        centerTimeline(section, false); // No animation on initial load
      }
    });

    // Cleanup function using captured values
    return () => {
      window.removeEventListener('resize', handleResize);
      currentContainer?.removeEventListener('scroll', throttledScrollHandler);
      currentSections.forEach((section) => {
        if (section) {
          observer.unobserve(section);
        }
      });
    };
  }, [sections, setCurrentDate]);

  interface TileProps {
    [key: string]: unknown;
  }

  interface Tile {
    type: string;
    props: TileProps;
  }

  const renderTile = (tile: Tile, index: number): ReactElement | null => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const props = tile.props as any; // Use any for flexibility with different tile prop types
    switch (tile.type) {
      case 'chart':
        return <ChartTile key={index} {...props} />;
      case 'highlight':
        return <HighlightTile key={index} {...props} />;
      case 'pie':
        return <PieChartTile key={index} {...props} />;
      case 'list':
        return <ListTile key={index} {...props} />;
      case 'clock':
        return <ClockTile key={index} {...props} />;
      default:
        return null;
    }
  };

  return (
    <div
      ref={containerRef}
      className="h-screen overflow-y-scroll overflow-x-hidden snap-y snap-mandatory relative scrollbar-hide"
    >
      {sections.map((section, index) => (
        <div
          key={index}
          ref={(el) => {
            sectionRefs.current[index] = el;
          }}
          className="h-screen relative snap-center snap-always overflow-y-hidden overflow-x-scroll snap-x snap-mandatory scrollbar-hide animate-[fadein_300ms_ease-in-out]"
        >
          <div className="relative min-w-[calc(200vw+100px)] h-full flex items-center">
            {/* Main flex container */}
            <div className="w-full h-full flex">
              {/* Left Grid */}
              <div className="w-screen p-4 mt-6 overflow-hidden">
                <div
                  className="ml-auto mr-0 flex flex-wrap gap-4 content-start justify-end"
                  style={{ width: 'min(720px, 90%)' }}
                >
                  {section.leftTiles.map((tile, i) => (
                    <div key={i} className="w-[calc(50%-8px)]">
                      {renderTile(tile, i)}
                    </div>
                  ))}
                </div>
              </div>

              {/* Center Timeline */}
              <div className="w-100px relative flex flex-col items-center h-screen">
                {/* Upper Timeline Dots */}
                <TimelineDots
                  height="calc(50vh - 96px)"
                  isUpper={true}
                  isCurrentDay={section.isToday}
                />

                {/* Date Display */}
                <div className="bg-white dark:bg-black shadow-[0_0_13.7px_rgba(0,0,0,0.04)] dark:shadow-[0_0_24px_rgba(255,255,255,0.08)] p-4 rounded-xl z-[3] flex flex-col items-center transition-all">
                  <div
                    className={`font-['Cash_Sans'] text-3xl font-light transition-colors ${
                      section.isToday
                        ? 'text-black dark:text-white'
                        : 'text-black/40 dark:text-white/40'
                    }`}
                  >
                    {section.date.toLocaleString('default', { month: 'short' })}
                  </div>
                  <div
                    className={`font-['Cash_Sans'] text-[64px] font-light leading-none transition-colors ${
                      section.isToday
                        ? 'text-black dark:text-white'
                        : 'text-black/40 dark:text-white/40'
                    }`}
                  >
                    {section.date.getDate()}
                  </div>
                  <div
                    className={`font-['Cash_Sans'] text-sm font-light mt-1 transition-colors ${
                      section.isToday
                        ? 'text-black dark:text-white'
                        : 'text-black/40 dark:text-white/40'
                    }`}
                  >
                    {section.date.toLocaleString('default', { weekday: 'long' })}
                  </div>
                </div>

                {/* Lower Timeline Dots */}
                <TimelineDots
                  height="calc(50vh - 96px)"
                  isUpper={false}
                  isCurrentDay={section.isToday}
                />
              </div>

              {/* Right Grid */}
              <div className="w-screen p-4 mt-6 overflow-hidden">
                <div
                  className="flex flex-wrap gap-4 content-start"
                  style={{ width: 'min(720px, 90%)' }}
                >
                  {section.rightTiles.map((tile, i) => (
                    <div key={i} className="w-[calc(50%-8px)]">
                      {renderTile(tile, i)}
                    </div>
                  ))}
                </div>
              </div>
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}
