import type { LucideIcon } from 'lucide-react';
import { cn } from '@/utils';

interface Tab {
  id: string;
  label: string;
  icon?: LucideIcon;
  badge?: string | number;
}

interface TabGroup {
  label?: string;
  tabs: Tab[];
}

interface TabBarProps {
  groups: TabGroup[];
  activeTab: string;
  onTabChange: (tabId: string) => void;
  variant?: 'default' | 'pill' | 'underline';
  className?: string;
}

export function TabBar({
  groups,
  activeTab,
  onTabChange,
  variant = 'default',
  className,
}: TabBarProps) {
  return (
    <div
      className={cn(
        'flex flex-wrap items-center gap-1',
        variant === 'underline' && 'border-b border-border-default gap-0',
        className
      )}
    >
      {groups.map((group, gi) => (
        <div key={gi} className="flex items-center gap-1">
          {group.label && (
            <span className="text-[10px] font-semibold uppercase tracking-wider text-text-muted mr-1 ml-2 first:ml-0">
              {group.label}
            </span>
          )}
          {group.tabs.map((tab) => {
            const isActive = tab.id === activeTab;
            const Icon = tab.icon;
            return (
              <button type="button"
                key={tab.id}
                onClick={() => onTabChange(tab.id)}
                className={cn(
                  'flex items-center gap-1.5 px-3 py-1.5 text-xs font-medium rounded-md transition-colors',
                  variant === 'default' && [
                    isActive
                      ? 'bg-background-muted text-text-default'
                      : 'text-text-muted hover:text-text-default hover:bg-background-muted/50',
                  ],
                  variant === 'pill' && [
                    isActive
                      ? 'bg-background-accent text-text-on-accent'
                      : 'text-text-muted hover:text-text-default hover:bg-background-muted',
                  ],
                  variant === 'underline' && [
                    'rounded-none border-b-2 -mb-px',
                    isActive
                      ? 'border-border-accent text-text-default'
                      : 'border-transparent text-text-muted hover:text-text-default hover:border-border-default',
                  ]
                )}
              >
                {Icon && <Icon className="h-3.5 w-3.5" />}
                {tab.label}
                {tab.badge !== undefined && (
                  <span
                    className={cn(
                      'text-[10px] rounded-full px-1.5 py-0.5 min-w-[1.25rem] text-center',
                      isActive
                        ? 'bg-background-default/20 text-inherit'
                        : 'bg-background-muted text-text-muted'
                    )}
                  >
                    {tab.badge}
                  </span>
                )}
              </button>
            );
          })}
        </div>
      ))}
    </div>
  );
}

export type { Tab, TabGroup, TabBarProps };
