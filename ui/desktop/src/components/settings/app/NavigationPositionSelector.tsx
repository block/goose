import { useState, useEffect } from 'react';
import { ArrowUp, ArrowDown, ArrowLeft, ArrowRight } from 'lucide-react';

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

export default function NavigationPositionSelector() {
  const { position, updatePosition } = useNavigationPosition();

  const positions: Array<{ value: NavigationPosition; label: string; icon: React.ReactNode }> = [
    { value: 'top', label: 'Top', icon: <ArrowUp className="w-5 h-5" /> },
    { value: 'left', label: 'Left', icon: <ArrowLeft className="w-5 h-5" /> },
    { value: 'bottom', label: 'Bottom', icon: <ArrowDown className="w-5 h-5" /> },
    { value: 'right', label: 'Right', icon: <ArrowRight className="w-5 h-5" /> },
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
