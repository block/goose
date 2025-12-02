import { useState, useEffect } from 'react';

export type NavigationPosition = 'top' | 'bottom' | 'left' | 'right';

const STORAGE_KEY = 'navigation_position';

export const useNavigationPosition = () => {
  const [position, setPosition] = useState<NavigationPosition>(() => {
    const stored = localStorage.getItem(STORAGE_KEY);
    return (stored as NavigationPosition) || 'top';
  });

  const updatePosition = (newPosition: NavigationPosition) => {
    setPosition(newPosition);
    localStorage.setItem(STORAGE_KEY, newPosition);
    // Dispatch custom event for AppLayout to listen to
    window.dispatchEvent(new CustomEvent('navigation-position-changed', { 
      detail: { position: newPosition } 
    }));
  };

  return { position, updatePosition };
};

// Icon components for navigation positions
const TopNavIcon = () => (
  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" className="w-5 h-5">
    {/* Small tiles on top */}
    <rect x="3" y="3" width="4" height="4" rx="1.5" fill="currentColor" />
    <rect x="10" y="3" width="4" height="4" rx="1.5" fill="currentColor" />
    <rect x="17" y="3" width="4" height="4" rx="1.5" fill="currentColor" />
    {/* Large tile below */}
    <rect x="3" y="10" width="18" height="11" rx="2" fill="currentColor" opacity="0.3" />
  </svg>
);

const BottomNavIcon = () => (
  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" className="w-5 h-5">
    {/* Large tile on top */}
    <rect x="3" y="3" width="18" height="11" rx="2" fill="currentColor" opacity="0.3" />
    {/* Small tiles on bottom */}
    <rect x="3" y="17" width="4" height="4" rx="1.5" fill="currentColor" />
    <rect x="10" y="17" width="4" height="4" rx="1.5" fill="currentColor" />
    <rect x="17" y="17" width="4" height="4" rx="1.5" fill="currentColor" />
  </svg>
);

const LeftNavIcon = () => (
  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" className="w-5 h-5">
    {/* Small tiles on left */}
    <rect x="3" y="3" width="4" height="4" rx="1.5" fill="currentColor" />
    <rect x="3" y="10" width="4" height="4" rx="1.5" fill="currentColor" />
    <rect x="3" y="17" width="4" height="4" rx="1.5" fill="currentColor" />
    {/* Large tile on right */}
    <rect x="10" y="3" width="11" height="18" rx="2" fill="currentColor" opacity="0.3" />
  </svg>
);

const RightNavIcon = () => (
  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" className="w-5 h-5">
    {/* Large tile on left */}
    <rect x="3" y="3" width="11" height="18" rx="2" fill="currentColor" opacity="0.3" />
    {/* Small tiles on right */}
    <rect x="17" y="3" width="4" height="4" rx="1.5" fill="currentColor" />
    <rect x="17" y="10" width="4" height="4" rx="1.5" fill="currentColor" />
    <rect x="17" y="17" width="4" height="4" rx="1.5" fill="currentColor" />
  </svg>
);

export default function NavigationPositionSelector() {
  const { position, updatePosition } = useNavigationPosition();

  const positions: Array<{ value: NavigationPosition; label: string; icon: React.ReactNode }> = [
    { value: 'top', label: 'Top', icon: <TopNavIcon /> },
    { value: 'left', label: 'Left', icon: <LeftNavIcon /> },
    { value: 'bottom', label: 'Bottom', icon: <BottomNavIcon /> },
    { value: 'right', label: 'Right', icon: <RightNavIcon /> },
  ];

  return (
    <div className="grid grid-cols-4 gap-2">
      {positions.map((pos) => (
        <button
          key={pos.value}
          onClick={() => updatePosition(pos.value)}
          className={`
            flex flex-col items-center justify-center gap-2 p-4 rounded-lg border-2 transition-all
            ${position === pos.value
              ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400'
              : 'border-border-default hover:border-border-subtle hover:bg-background-subtle'
            }
          `}
        >
          {pos.icon}
          <span className="text-xs font-medium">{pos.label}</span>
        </button>
      ))}
    </div>
  );
}
