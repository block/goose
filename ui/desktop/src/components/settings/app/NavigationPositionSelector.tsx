import React, { useState, useEffect } from 'react';
import { ArrowUp, ArrowDown, ArrowLeft, ArrowRight } from 'lucide-react';

export type NavigationPosition = 'top' | 'bottom' | 'left' | 'right';

interface NavigationPositionSelectorProps {
  className?: string;
}

export const NavigationPositionSelector: React.FC<NavigationPositionSelectorProps> = ({ className }) => {
  const [selectedPosition, setSelectedPosition] = useState<NavigationPosition>(() => {
    const stored = localStorage.getItem('navigation_position');
    return (stored as NavigationPosition) || 'top';
  });

  useEffect(() => {
    localStorage.setItem('navigation_position', selectedPosition);
    window.dispatchEvent(
      new CustomEvent('navigation-position-changed', {
        detail: { position: selectedPosition },
      })
    );
  }, [selectedPosition]);

  const positions: { value: NavigationPosition; label: string; icon: React.ReactNode }[] = [
    {
      value: 'top',
      label: 'Top',
      icon: <ArrowUp className="w-5 h-5" />,
    },
    {
      value: 'bottom',
      label: 'Bottom',
      icon: <ArrowDown className="w-5 h-5" />,
    },
    {
      value: 'left',
      label: 'Left',
      icon: <ArrowLeft className="w-5 h-5" />,
    },
    {
      value: 'right',
      label: 'Right',
      icon: <ArrowRight className="w-5 h-5" />,
    },
  ];

  return (
    <div className={className}>
      <div className="grid grid-cols-4 gap-3">
        {positions.map((position) => (
          <button
            key={position.value}
            onClick={() => setSelectedPosition(position.value)}
            className={`
              flex flex-col items-center gap-2 p-4 rounded-lg border-2 transition-all
              ${
                selectedPosition === position.value
                  ? 'border-border-strong bg-background-medium'
                  : 'border-border-subtle bg-background-default hover:border-border-medium'
              }
            `}
          >
            <div className="text-text-default">{position.icon}</div>
            <div className="text-xs font-medium text-text-default">{position.label}</div>
          </button>
        ))}
      </div>
    </div>
  );
};
