import React from 'react';
import { X, Plus, MessageCircle, Bot, Users } from 'lucide-react';
import { cn } from '../utils';
import '../styles/tabs.css';

// Sidecar view interface for tab-specific sidecars
export interface TabSidecarView {
  id: string;
  title: string;
  iconType: 'diff' | 'localhost' | 'web' | 'file' | 'editor';
  contentType: 'diff' | 'localhost' | 'web' | 'file' | 'editor';
  contentProps: Record<string, any>;
  fileName?: string;
  instanceId?: string;
}

// Sidecar state for each tab
export interface TabSidecarState {
  activeViews: string[]; // Array of active view IDs
  views: TabSidecarView[]; // All available views for this tab
}

export interface Tab {
  id: string;
  title: string;
  type: 'chat' | 'recipe' | 'matrix';
  sessionId: string;
  isActive: boolean;
  hasUnsavedChanges?: boolean;
  matrixRoomId?: string;
  matrixRecipientId?: string;
  recipeTitle?: string;
  // Add sidecar state to each tab
  sidecarState?: TabSidecarState;
}

interface TabBarProps {
  tabs: Tab[];
  activeTabId: string;
  onTabClick: (tabId: string) => void;
  onTabClose: (tabId: string) => void;
  onNewTab: () => void;
  className?: string;
  sidebarCollapsed?: boolean;
  workingDirectory?: string;
}

const getTabIcon = (type: Tab['type']) => {
  switch (type) {
    case 'recipe':
      return <Bot className="w-3 h-3" />;
    case 'matrix':
      return <Users className="w-3 h-3" />;
    default:
      return <MessageCircle className="w-3 h-3" />;
  }
};

const getTabTitle = (tab: Tab) => {
  if (tab.recipeTitle) return tab.recipeTitle;
  if (tab.matrixRoomId) return `Matrix: ${tab.title}`;
  return tab.title || 'New Chat';
};

const getTabTooltip = (tab: Tab, workingDirectory?: string) => {
  const title = getTabTitle(tab);
  if (workingDirectory) {
    return `${title}\nWorkspace: ${workingDirectory}`;
  }
  return title;
};

export const TabBar: React.FC<TabBarProps> = ({
  tabs,
  activeTabId,
  onTabClick,
  onTabClose,
  onNewTab,
  className,
  sidebarCollapsed = false,
  workingDirectory
}) => {
  return (
    <div className={cn(
      "flex bg-zinc-100 dark:bg-neutral-950/50",
      "min-h-[40px] gap-1 overflow-x-auto tab-bar-container",
      "transition-all duration-200", 
      // Adjust padding based on sidebar state - extra left padding when sidebar is collapsed for macOS stoplight buttons
      sidebarCollapsed ? "pl-20 pr-3" : "px-3",
      className
    )}>
      {/* Tabs */}
      {tabs.map((tab) => (
        <button
          key={tab.id}
          className={cn(
            "flex items-center gap-2 px-4 py-1.5 cursor-pointer no-drag border-0",
            "min-w-[140px] max-w-[220px] group relative tab-item rounded-t-lg",
            "transition-all duration-200 ease-out",
            tab.isActive
              ? "bg-white dark:bg-neutral-800 text-zinc-900 dark:text-zinc-100 shadow-sm ring-1 ring-black/5 dark:ring-white/5 active"
              : "bg-transparent text-zinc-500 dark:text-zinc-400 hover:bg-white/50 dark:hover:bg-white/5 hover:text-zinc-700 dark:hover:text-zinc-300"
          )}
          onClick={() => onTabClick(tab.id)}
          title={getTabTooltip(tab, workingDirectory)}
        >
          {/* Tab Icon */}
          <div className="flex-shrink-0 opacity-80 pointer-events-none">
            {getTabIcon(tab.type)}
          </div>

          {/* Tab Title */}
          <span className="truncate text-sm font-medium flex-1 pointer-events-none">
            {getTabTitle(tab)}
          </span>

          {/* Unsaved Changes Indicator */}
          {tab.hasUnsavedChanges && (
            <div className="w-2 h-2 bg-accent-warning rounded-full flex-shrink-0 unsaved-indicator pointer-events-none" />
          )}

          {/* Close Button - Only show for active tab */}
          {tab.isActive && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                onTabClose(tab.id);
              }}
              className={cn(
                "flex-shrink-0 w-6 h-6 flex items-center justify-center rounded-md tab-close-button",
                "text-text-muted hover:text-text-standard hover:bg-black/5 dark:hover:bg-white/10 transition-all duration-200",
                "ml-1.5 -mr-1" 
              )}
              title="Close tab"
            >
              <X className="w-3.5 h-3.5" />
            </button>
          )}
        </button>
      ))}

      {/* New Tab Button */}
      <button
        onClick={onNewTab}
        className={cn(
          "flex items-center justify-center w-8 h-8 rounded-lg ml-1 self-center",
          "text-text-muted hover:text-text-standard new-tab-button",
          "hover:bg-background-subtle transition-all duration-200",
          "border border-transparent hover:border-border-subtle",
          "shadow-sm hover:shadow-md" // Add subtle shadow effects
        )}
        title="New tab (Ctrl+T)"
      >
        <Plus className="w-4 h-4" />
      </button>

      {/* Spacer to push content left */}
      <div className="flex-1" />
    </div>
  );
};
