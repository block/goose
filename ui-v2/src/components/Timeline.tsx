import React, { useRef, useMemo, useEffect } from 'react';
import ChartTile from './tiles/ChartTile.tsx';
import HighlightTile from './tiles/HighlightTile.tsx';
import PieChartTile from './tiles/PieChartTile.tsx';
import ListTile from './tiles/ListTile.tsx';
import ClockTile from './tiles/ClockTile.tsx';
import TimelineDots from './TimelineDots';
import {
  ChartLineIcon,
  ChartBarIcon,
  PieChartIcon,
  ListIcon,
  StarIcon,
  TrendingUpIcon,
} from './icons';

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

export default function Timeline() {
  const containerRef = useRef<HTMLDivElement>(null);
  const sectionRefs = useRef<(HTMLDivElement | null)[]>([]);

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
  const centerTimeline = (sectionElement: HTMLDivElement) => {
    if (!sectionElement) return;

    requestAnimationFrame(() => {
      const totalWidth = sectionElement.scrollWidth;
      const viewportWidth = sectionElement.clientWidth;
      const scrollToX = Math.max(0, (totalWidth - viewportWidth) / 2);

      sectionElement.scrollTo({
        left: scrollToX,
        behavior: 'smooth',
      });
    });
  };

  useEffect(() => {
    // Create the intersection observer
    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            const section = entry.target as HTMLDivElement;
            centerTimeline(section);
          }
        });
      },
      {
        threshold: 0.5,
        rootMargin: '0px',
      }
    );

    // Add resize handler
    const handleResize = () => {
      // Find the currently visible section
      const visibleSection = sectionRefs.current.find((section) => {
        if (!section) return false;
        const rect = section.getBoundingClientRect();
        const viewportHeight = window.innerHeight;
        // Check if the section is mostly visible in the viewport
        return rect.top >= -viewportHeight / 2 && rect.bottom <= viewportHeight * 1.5;
      });

      if (visibleSection) {
        centerTimeline(visibleSection);
      }
    };

    // Add resize event listener
    window.addEventListener('resize', handleResize);

    // Observe all sections
    sectionRefs.current.forEach((section) => {
      if (section) {
        observer.observe(section);
        centerTimeline(section);
      }
    });

    // Cleanup function
    return () => {
      window.removeEventListener('resize', handleResize);
      sectionRefs.current.forEach((section) => {
        if (section) {
          observer.unobserve(section);
        }
      });
    };
  }, []);

  const renderTile = (tile: any, index: number) => {
    switch (tile.type) {
      case 'chart':
        return <ChartTile key={index} {...tile.props} />;
      case 'highlight':
        return <HighlightTile key={index} {...tile.props} />;
      case 'pie':
        return <PieChartTile key={index} {...tile.props} />;
      case 'list':
        return <ListTile key={index} {...tile.props} />;
      case 'clock':
        return <ClockTile key={index} {...tile.props} />;
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
          ref={(el) => (sectionRefs.current[index] = el)}
          className="h-screen relative snap-center snap-always overflow-y-hidden overflow-x-scroll snap-x snap-mandatory scrollbar-hide"
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
                <div className="bg-white p-4 rounded z-[3] flex flex-col items-center transition-opacity">
                  <div
                    className={`font-['Cash_Sans'] text-3xl font-light ${section.isToday ? 'opacity-100' : 'opacity-20'}`}
                  >
                    {section.date.toLocaleString('default', { month: 'short' })}
                  </div>
                  <div
                    className={`font-['Cash_Sans'] text-[64px] font-light leading-none ${section.isToday ? 'opacity-100' : 'opacity-20'}`}
                  >
                    {section.date.getDate()}
                  </div>
                  <div
                    className={`font-['Cash_Sans'] text-sm font-light mt-1 ${section.isToday ? 'opacity-100' : 'opacity-20'}`}
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
