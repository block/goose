import React from 'react';
import {
  Code,
  BarChart,
  Users,
  Package,
  MapPin,
  ShieldCheck,
  TrendingUp,
  Clock,
  AlertTriangle,
  FolderTree,
  MessageSquare,
  BookOpen,
  Terminal,
  Zap,
  Search,
  FileText,
  LayoutDashboard,
  CalendarCheck,
  type LucideIcon,
} from 'lucide-react';
import type { WhiteLabelStarterPrompt } from '../../whitelabel/types';

const ICON_MAP: Record<string, LucideIcon> = {
  'folder-tree': FolderTree,
  'message-square': MessageSquare,
  code: Code,
  'book-open': BookOpen,
  terminal: Terminal,
  zap: Zap,
  search: Search,
  'file-text': FileText,
  'layout-dashboard': LayoutDashboard,
  'calendar-check': CalendarCheck,
  'bar-chart': BarChart,
  users: Users,
  package: Package,
  'map-pin': MapPin,
  'shield-check': ShieldCheck,
  'trending-up': TrendingUp,
  clock: Clock,
  'alert-triangle': AlertTriangle,
};

function getIcon(name: string, className = 'w-4 h-4'): React.ReactNode {
  const Icon = ICON_MAP[name] ?? Code;
  return <Icon className={className} />;
}

interface InsightCardsProps {
  prompts: WhiteLabelStarterPrompt[];
  onSelect: (prompt: string) => void;
  heading?: string;
}

export function InsightCards({ prompts, onSelect, heading = 'Insights' }: InsightCardsProps) {
  return (
    <div className="bg-background-primary rounded-2xl p-6">
      <h3 className="text-sm text-text-secondary mb-3">{heading}</h3>
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-2">
        {prompts.map((sp, i) => (
          <button
            key={`insight-${i}`}
            className="flex flex-col gap-2 p-4 rounded-xl bg-background-secondary hover:bg-background-tertiary border border-border-primary transition-colors text-left cursor-pointer"
            onClick={() => onSelect(sp.prompt)}
          >
            <div className="flex items-center justify-between w-full">
              <span className="text-sm font-medium text-text-primary">{sp.label}</span>
              <span className="text-text-secondary flex-shrink-0 ml-2">
                {getIcon(sp.icon, 'w-4 h-4')}
              </span>
            </div>
            {sp.description && (
              <p className="text-xs text-text-secondary leading-relaxed line-clamp-2">
                {sp.description}
              </p>
            )}
            <span className="text-xs text-text-info hover:underline">
              {sp.action ?? 'Review →'}
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}
