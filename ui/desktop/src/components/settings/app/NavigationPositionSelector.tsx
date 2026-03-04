import React from 'react';
import { ArrowUp, ArrowDown, ArrowLeft, ArrowRight } from 'lucide-react';
import { useNavigationContext, NavigationPosition } from '../../Layout/NavigationContext';
import { cn } from '../../../utils';

interface NavigationPositionSelectorProps {
  className?: string;
}

const positions: { value: NavigationPosition; label: string; icon: React.ReactNode }[] = [
  { value: 'top', label: 'Top', icon: <ArrowUp className="w-5 h-5" /> },
  { value: 'bottom', label: 'Bottom', icon: <ArrowDown className="w-5 h-5" /> },
  { value: 'left', label: 'Left', icon: <ArrowLeft className="w-5 h-5" /> },
  { value: 'right', label: 'Right', icon: <ArrowRight className="w-5 h-5" /> },
];

export const NavigationPositionSelector: React.FC<NavigationPositionSelectorProps> = ({
  className,
}) => {
  const { navigationPosition, setNavigationPosition } = useNavigationContext();

  return (
    <div className={className}>
      <div className="grid grid-cols-4 gap-3">
        {positions.map((position) => (
          <button
            key={position.value}
            onClick={() => setNavigationPosition(position.value)}
            className={cn(
              'flex flex-col items-center gap-2 p-3 rounded-lg border-2 transition-all',
              navigationPosition === position.value
                ? 'border-border-primary bg-background-tertiary'
                : 'border-border-secondary bg-background-primary hover:border-border-medium'
            )}
          >
            <div className="text-text-primary">{position.icon}</div>
            <div className="text-xs font-medium text-text-primary">{position.label}</div>
          </button>
        ))}
      </div>
    </div>
  );
};
