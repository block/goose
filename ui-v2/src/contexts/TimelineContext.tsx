import React, { createContext, useContext, useState } from 'react';

interface TimelineContextType {
  currentDate: Date;
  setCurrentDate: (date: Date) => void;
}

const TimelineContext = createContext<TimelineContextType | undefined>(undefined);

export function TimelineProvider({ children }: { children: React.ReactNode }) {
  const [currentDate, setCurrentDate] = useState(new Date());

  return (
    <TimelineContext.Provider value={{ currentDate, setCurrentDate }}>
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
