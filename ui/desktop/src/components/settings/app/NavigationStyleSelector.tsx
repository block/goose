import React from 'react';
import { LayoutGrid, List } from 'lucide-react';
import { useNavigationContext, NavigationStyle } from '../../Layout/NavigationContext';
import { cn } from '../../../utils';

interface NavigationStyleSelectorProps {
  className?: string;
}

const styles: { value: NavigationStyle; label: string; icon: React.ReactNode; description: string }[] = [
  {
    value: 'expanded',
    label: 'Expanded',
    icon: <LayoutGrid className="w-5 h-5" />,
    description: 'Full navigation with labels',
  },
  {
    value: 'condensed',
    label: 'Condensed',
    icon: <List className="w-5 h-5" />,
    description: 'Compact icon-only view',
  },
];

export const NavigationStyleSelector: React.FC<NavigationStyleSelectorProps> = ({ className }) => {
  const { navigationStyle, setNavigationStyle } = useNavigationContext();

  return (
    <div className={className}>
      <div className="grid grid-cols-2 gap-3">
        {styles.map((style) => (
          <button
            key={style.value}
            onClick={() => setNavigationStyle(style.value)}
            className={cn(
              'flex flex-col items-center gap-2 p-4 rounded-lg border-2 transition-all',
              navigationStyle === style.value
                ? 'border-border-strong bg-background-medium'
                : 'border-border-subtle bg-background-default hover:border-border-medium'
            )}
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
