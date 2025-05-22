import React, { useState, useEffect } from 'react';
import { FilterOption } from './types';

interface FloatingFiltersProps {
  children: React.ReactNode;
}

const getBarColor = (filters: FilterOption[], isDarkMode: boolean): string => {
  const activeFilter = filters.find(f => f.isActive);
  switch (activeFilter?.id) {
    case 'tasks':
      return '#05C168';
    case 'projects':
      return '#0066FF';
    case 'automations':
      return '#B18CFF';
    case 'problems':
      return '#FF2E6C';
    default:
      return isDarkMode ? '#FFFFFF' : '#000000';
  }
};

export function FloatingFilters({ children }: FloatingFiltersProps) {
  const [isVisible, setIsVisible] = useState(false);
  const [isHovering, setIsHovering] = useState(false);
  const [activeFilters, setActiveFilters] = useState<FilterOption[]>([]);
  const [isDarkMode, setIsDarkMode] = useState(false);

  useEffect(() => {
    const timer = setTimeout(() => {
      setIsVisible(isHovering);
    }, isHovering ? 0 : 300); // Increased delay for smoother mouseout

    return () => clearTimeout(timer);
  }, [isHovering]);

  useEffect(() => {
    const filterPills = React.Children.toArray(children)[0] as React.ReactElement;
    if (filterPills?.props?.filters) {
      setActiveFilters(filterPills.props.filters);
    }
  }, [children]);

  useEffect(() => {
    const isDark = document.documentElement.classList.contains('dark');
    setIsDarkMode(isDark);

    const observer = new MutationObserver((mutations) => {
      mutations.forEach((mutation) => {
        if (mutation.attributeName === 'class') {
          setIsDarkMode(document.documentElement.classList.contains('dark'));
        }
      });
    });

    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['class']
    });

    return () => observer.disconnect();
  }, []);

  const barColor = getBarColor(activeFilters, isDarkMode);

  return (
    <div 
      className="fixed left-0 right-0 z-50 top-[56px]"
      onMouseEnter={() => setIsHovering(true)}
      onMouseLeave={() => setIsHovering(false)}
    >
      {/* Hover trigger area with colored bar indicator */}
      <div className="absolute top-0 left-0 right-0 h-16 bg-transparent flex justify-center">
        <div 
          className={`
            w-[200px] h-[6px]
            rounded-b-[24px]
            transition-all duration-500 ease-in-out
            absolute top-0
            ${isVisible ? 'opacity-0 transform -translate-y-1' : 'opacity-100 transform translate-y-0'}
          `}
          style={{ backgroundColor: barColor }}
        />
      </div>

      {/* Filters container with transition */}
      <div 
        className={`
          transform transition-all duration-500 ease-in-out w-full bg-transparent
          ${isVisible 
            ? 'translate-y-0 opacity-100 scale-y-100 origin-top' 
            : 'translate-y-[calc(-100%+6px)] opacity-0 scale-y-95 origin-top'
          }
        `}
      >
        <div className="filter-container px-4">
          {children}
        </div>
      </div>
    </div>
  );
}
