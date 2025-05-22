import React, { Suspense } from 'react';
import { Outlet } from '@tanstack/react-router';
import { FloatingFilters } from '../components/filters/FloatingFilters';
import SuspenseLoader from '../components/SuspenseLoader';
import { FilterOption } from '../components/filters/types';
import { DarkModeToggle } from '../components/DarkModeToggle';
import { DateDisplay } from '../components/DateDisplay';
import { TimelineProvider } from '../contexts/TimelineContext';

const defaultFilters: FilterOption[] = [
  { id: 'all', label: 'All', isActive: true },
  { id: 'metrics', label: 'Metrics', isActive: false },
  { id: 'tasks', label: 'Tasks', isActive: false },
  { id: 'projects', label: 'Projects', isActive: false },
  { id: 'automations', label: 'Automations', isActive: false },
  { id: 'problems', label: 'Problems', isActive: false },
];

const getFilterColor = (filterId: string): string => {
  switch (filterId) {
    case 'tasks':
      return '#05C168';
    case 'projects':
      return '#0066FF';
    case 'automations':
      return '#B18CFF';
    case 'problems':
      return '#FF2E6C';
    default:
      return 'transparent';
  }
};

export const MainLayout: React.FC = () => {
  return (
    <TimelineProvider>
      <div className="min-h-screen bg-background-default dark:bg-zinc-800 transition-colors duration-200">
        <div className="titlebar-drag-region" />
        <DateDisplay />
        <div className="h-10 w-full" />
        
        <FloatingFilters>
          <div filters={defaultFilters}>
            <div className="flex justify-center w-full px-4 pt-4">
              <div className="inline-flex gap-3 justify-center">
                {defaultFilters.map((filter) => (
                  <button
                    key={filter.id}
                    className={`
                      cursor-pointer
                      px-4 py-2 rounded-full text-sm font-light transition-all
                      shadow-[0_0_13.7px_rgba(0,0,0,0.04)]
                      dark:shadow-[0_0_24px_rgba(255,255,255,0.08)]
                      flex items-center gap-2
                      ${filter.isActive
                        ? 'bg-background-inverse text-text-inverse'
                        : 'bg-background-default text-text-muted hover:text-text-default dark:text-white/60 dark:hover:text-white'
                      }
                    `}
                  >
                    {filter.id !== 'all' && filter.id !== 'metrics' && (
                      <div 
                        className="w-2 h-2 rounded-full"
                        style={{ backgroundColor: getFilterColor(filter.id) }}
                      />
                    )}
                    {filter.label}
                  </button>
                ))}
              </div>
            </div>
          </div>
        </FloatingFilters>

        <main className="w-full">
          <Suspense fallback={<SuspenseLoader />}>
            <Outlet />
          </Suspense>
        </main>

        <DarkModeToggle />
      </div>
    </TimelineProvider>
  );
};
