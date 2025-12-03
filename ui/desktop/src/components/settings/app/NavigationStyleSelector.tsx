import { useState, useEffect } from 'react';

export type NavigationStyle = 'expanded' | 'condensed';

const STORAGE_KEY = 'navigation_style';

export const useNavigationStyle = () => {
  const [style, setStyle] = useState<NavigationStyle>(() => {
    const stored = localStorage.getItem(STORAGE_KEY);
    return (stored as NavigationStyle) || 'expanded';
  });

  const updateStyle = (newStyle: NavigationStyle) => {
    setStyle(newStyle);
    localStorage.setItem(STORAGE_KEY, newStyle);
    // Dispatch custom event for AppLayout to listen to
    window.dispatchEvent(new CustomEvent('navigation-style-changed', { 
      detail: { style: newStyle } 
    }));
  };

  return { style, updateStyle };
};

// Icon components for navigation styles
const ExpandedIcon = () => (
  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" className="w-5 h-5">
    {/* Grid of tiles */}
    <rect x="3" y="3" width="7" height="7" rx="1.5" fill="currentColor" />
    <rect x="13" y="3" width="7" height="7" rx="1.5" fill="currentColor" />
    <rect x="3" y="13" width="7" height="7" rx="1.5" fill="currentColor" />
    <rect x="13" y="13" width="7" height="7" rx="1.5" fill="currentColor" opacity="0.5" />
  </svg>
);

const CondensedIcon = () => (
  <svg width="24" height="24" viewBox="0 0 24 24" fill="none" className="w-5 h-5">
    {/* Horizontal rows */}
    <rect x="3" y="4" width="18" height="3" rx="1.5" fill="currentColor" />
    <rect x="3" y="10" width="18" height="3" rx="1.5" fill="currentColor" />
    <rect x="3" y="16" width="18" height="3" rx="1.5" fill="currentColor" opacity="0.5" />
  </svg>
);

export default function NavigationStyleSelector() {
  const { style, updateStyle } = useNavigationStyle();

  const styles: Array<{ value: NavigationStyle; label: string; description: string; icon: React.ReactNode }> = [
    { 
      value: 'expanded', 
      label: 'Expanded', 
      description: 'Large tiles with details',
      icon: <ExpandedIcon /> 
    },
    { 
      value: 'condensed', 
      label: 'Condensed', 
      description: 'Compact rows',
      icon: <CondensedIcon /> 
    },
  ];

  return (
    <div className="grid grid-cols-2 gap-3">
      {styles.map((styleOption) => (
        <button
          key={styleOption.value}
          onClick={() => updateStyle(styleOption.value)}
          className={`
            flex flex-col items-center justify-center gap-3 p-4 rounded-lg border-2 transition-all
            ${style === styleOption.value
              ? 'border-blue-500 bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400'
              : 'border-border-default hover:border-border-subtle hover:bg-background-subtle'
            }
          `}
        >
          {styleOption.icon}
          <div className="text-center">
            <span className="text-sm font-medium block">{styleOption.label}</span>
            <span className="text-xs text-text-muted">{styleOption.description}</span>
          </div>
        </button>
      ))}
    </div>
  );
}
