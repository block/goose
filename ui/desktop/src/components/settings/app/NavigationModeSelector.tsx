import React from 'react';
import { Columns2, Layers } from 'lucide-react';
import { useNavigationContext, NavigationMode } from '../../Layout/NavigationContext';
import { cn } from '../../../utils';

interface NavigationModeSelectorProps {
  className?: string;
}

const modes: {
  value: NavigationMode;
  label: string;
  icon: React.ReactNode;
  description: string;
}[] = [
  {
    value: 'push',
    label: 'Push',
    icon: <Columns2 className="w-5 h-5" />,
    description: 'Navigation pushes content',
  },
  {
    value: 'overlay',
    label: 'Overlay',
    icon: <Layers className="w-5 h-5" />,
    description: 'Full-screen overlay',
  },
];

export const NavigationModeSelector: React.FC<NavigationModeSelectorProps> = ({ className }) => {
  const { navigationMode, setNavigationMode } = useNavigationContext();

  return (
    <div className={className}>
      <div className="grid grid-cols-2 gap-3">
        {modes.map((mode) => (
          <button
            key={mode.value}
            onClick={() => setNavigationMode(mode.value)}
            className={cn(
              'flex flex-col items-center gap-2 p-4 rounded-lg border-2 transition-all',
              navigationMode === mode.value
                ? 'border-border-primary bg-background-tertiary'
                : 'border-border-secondary bg-background-primary hover:border-border-medium'
            )}
          >
            <div className="text-text-primary">{mode.icon}</div>
            <div className="text-center">
              <div className="text-sm font-medium text-text-primary">{mode.label}</div>
              <div className="text-xs text-text-secondary mt-1">{mode.description}</div>
            </div>
          </button>
        ))}
      </div>
    </div>
  );
};
