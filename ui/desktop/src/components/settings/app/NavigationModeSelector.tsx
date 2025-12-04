import { useState, useEffect } from 'react';

export type NavigationMode = 'push' | 'overlay';

const STORAGE_KEY = 'navigation_mode';

export const useNavigationMode = () => {
  const [mode, setMode] = useState<NavigationMode>(() => {
    const stored = localStorage.getItem(STORAGE_KEY);
    return (stored as NavigationMode) || 'push';
  });

  const updateMode = (newMode: NavigationMode) => {
    setMode(newMode);
    localStorage.setItem(STORAGE_KEY, newMode);
    // Dispatch custom event for AppLayout to listen to
    window.dispatchEvent(new CustomEvent('navigation-mode-changed', { 
      detail: { mode: newMode } 
    }));
  };

  return { mode, updateMode };
};

// Icon components for navigation modes
const PushMenuIcon = () => (
  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" className="w-5 h-5">
    {/* Sidebar pushing content */}
    <rect x="3" y="3" width="6" height="18" rx="2" fill="currentColor" />
    <rect x="12" y="3" width="9" height="18" rx="2" fill="currentColor" opacity="0.3" />
    {/* Arrow indicating push */}
    <path 
      d="M10 8 L14 12 L10 16" 
      stroke="currentColor" 
      strokeWidth="1.5" 
      fill="none" 
      strokeLinecap="round" 
      strokeLinejoin="round"
    />
  </svg>
);

const OverlayIcon = () => (
  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" className="w-5 h-5">
    {/* Full background */}
    <rect x="3" y="3" width="18" height="18" rx="2" fill="currentColor" opacity="0.3" />
    {/* Overlay panel floating on top */}
    <rect x="6" y="6" width="12" height="12" rx="2" fill="currentColor" />
    {/* Small indicator dots */}
    <circle cx="8" cy="8" r="1" fill="currentColor" opacity="0.3" />
    <circle cx="12" cy="8" r="1" fill="currentColor" opacity="0.3" />
    <circle cx="16" cy="8" r="1" fill="currentColor" opacity="0.3" />
  </svg>
);

export default function NavigationModeSelector() {
  const { mode, updateMode } = useNavigationMode();

  const modes: Array<{ 
    value: NavigationMode; 
    label: string; 
    description: string; 
    icon: React.ReactNode 
  }> = [
    { 
      value: 'push', 
      label: 'Push Menu', 
      description: 'Navigation pushes content aside',
      icon: <PushMenuIcon /> 
    },
    { 
      value: 'overlay', 
      label: 'Overlay', 
      description: 'Floating launcher overlay',
      icon: <OverlayIcon /> 
    },
  ];

  return (
    <div className="grid grid-cols-2 gap-3">
      {modes.map((modeOption) => (
        <button
          key={modeOption.value}
          onClick={() => updateMode(modeOption.value)}
          className={`
            flex flex-col items-center justify-center gap-3 p-4 rounded-lg border-2 transition-all
            ${mode === modeOption.value
              ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400'
              : 'border-border-default hover:border-border-subtle hover:bg-background-subtle'
            }
          `}
        >
          {modeOption.icon}
          <div className="text-center">
            <span className="text-sm font-medium block">{modeOption.label}</span>
            <span className="text-xs text-text-muted">{modeOption.description}</span>
          </div>
        </button>
      ))}
    </div>
  );
}
