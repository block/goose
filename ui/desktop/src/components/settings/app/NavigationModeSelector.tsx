import React, { useState, useEffect } from 'react';
import { PanelLeft, Layers } from 'lucide-react';

export type NavigationMode = 'push' | 'overlay';

interface NavigationModeSelectorProps {
  className?: string;
}

export const NavigationModeSelector: React.FC<NavigationModeSelectorProps> = ({ className }) => {
  const [selectedMode, setSelectedMode] = useState<NavigationMode>(() => {
    const stored = localStorage.getItem('navigation_mode');
    return (stored as NavigationMode) || 'push';
  });

  useEffect(() => {
    localStorage.setItem('navigation_mode', selectedMode);
    
    // When overlay mode is selected, automatically set position to center and style to expanded
    if (selectedMode === 'overlay') {
      localStorage.setItem('navigation_position', 'top'); // Center position
      localStorage.setItem('navigation_style', 'expanded');
      
      // Dispatch events to update other components
      window.dispatchEvent(
        new CustomEvent('navigation-position-changed', {
          detail: { position: 'top' },
        })
      );
      window.dispatchEvent(
        new CustomEvent('navigation-style-changed', {
          detail: { style: 'expanded' },
        })
      );
    }
    
    window.dispatchEvent(
      new CustomEvent('navigation-mode-changed', {
        detail: { mode: selectedMode },
      })
    );
  }, [selectedMode]);

  const modes: { value: NavigationMode; label: string; icon: React.ReactNode; description: string }[] = [
    {
      value: 'push',
      label: 'Push',
      icon: <PanelLeft className="w-5 h-5" />,
      description: 'Navigation pushes content',
    },
    {
      value: 'overlay',
      label: 'Overlay',
      icon: <Layers className="w-5 h-5" />,
      description: 'Full-screen overlay',
    },
  ];

  return (
    <div className={className}>
      <div className="grid grid-cols-2 gap-3">
        {modes.map((mode) => (
          <button
            key={mode.value}
            onClick={() => setSelectedMode(mode.value)}
            className={`
              flex flex-col items-center gap-2 p-4 rounded-lg border-2 transition-all
              ${
                selectedMode === mode.value
                  ? 'border-border-strong bg-background-medium'
                  : 'border-border-subtle bg-background-default hover:border-border-medium'
              }
            `}
          >
            <div className="text-text-default">{mode.icon}</div>
            <div className="text-center">
              <div className="text-sm font-medium text-text-default">{mode.label}</div>
              <div className="text-xs text-text-muted mt-1">{mode.description}</div>
            </div>
          </button>
        ))}
      </div>
    </div>
  );
};
