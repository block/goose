import React, { useState, useEffect } from 'react';
import { LayoutGrid, List } from 'lucide-react';

export type NavigationStyle = 'expanded' | 'condensed';

interface NavigationStyleSelectorProps {
  className?: string;
}

export const NavigationStyleSelector: React.FC<NavigationStyleSelectorProps> = ({ className }) => {
  const [selectedStyle, setSelectedStyle] = useState<NavigationStyle>(() => {
    const stored = localStorage.getItem('navigation_style');
    return (stored as NavigationStyle) || 'expanded';
  });

  useEffect(() => {
    localStorage.setItem('navigation_style', selectedStyle);
    window.dispatchEvent(
      new CustomEvent('navigation-style-changed', {
        detail: { style: selectedStyle },
      })
    );
  }, [selectedStyle]);

  const styles: { value: NavigationStyle; label: string; icon: React.ReactNode; description: string }[] = [
    {
      value: 'expanded',
      label: 'Expanded',
      icon: <LayoutGrid className="w-5 h-5" />,
      description: 'Large tiles with widgets',
    },
    {
      value: 'condensed',
      label: 'Condensed',
      icon: <List className="w-5 h-5" />,
      description: 'Compact rows',
    },
  ];

  return (
    <div className={className}>
      <div className="grid grid-cols-2 gap-3">
        {styles.map((style) => (
          <button
            key={style.value}
            onClick={() => setSelectedStyle(style.value)}
            className={`
              flex flex-col items-center gap-2 p-4 rounded-lg border-2 transition-all
              ${
                selectedStyle === style.value
                  ? 'border-border-strong bg-background-medium'
                  : 'border-border-subtle bg-background-default hover:border-border-medium'
              }
            `}
          >
            <div className="text-text-default">{style.icon}</div>
            <div className="text-center">
              <div className="text-sm font-medium text-text-default">{style.label}</div>
              <div className="text-xs text-text-muted mt-1">{style.description}</div>
            </div>
          </button>
        ))}
      </div>
    </div>
  );
};
