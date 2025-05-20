import React, { createContext, useContext, useState, useCallback } from 'react';

interface TimelineContextType {
  currentDate: Date;
  setCurrentDate: (date: Date) => void;
  isCurrentDate: (date: Date) => boolean;
}

const TimelineContext = createContext<TimelineContextType | undefined>(undefined);

export function TimelineProvider({ children }: { children: React.ReactNode }) {
  const [currentDate, setCurrentDate] = useState(new Date());

  const isCurrentDate = useCallback((date: Date) => {
    return date.toDateString() === new Date().toDateString();
  }, []);

  return (
    <TimelineContext.Provider value={{ currentDate, setCurrentDate, isCurrentDate }}>
      {children}
    </TimelineContext.Provider>
  );
}

export function useTimeline() {
  const context = useContext(TimelineContext);
  if (context === undefined) {
    throw new Error('useTimeline must be used within a TimelineProvider');
  }
  return context;
}
