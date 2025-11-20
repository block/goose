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
  sidebarCollapsed?: boolean; // Add prop to know if sidebar is collapsed
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

export const TabBar: React.FC<TabBarProps> = ({
  tabs,
  activeTabId,
  onTabClick,
  onTabClose,
  onNewTab,
  className,
  sidebarCollapsed = false
}) => {
  return (
    <div className={cn(
      "flex items-center bg-background-default border-b border-border-subtle",
      "min-h-[44px] gap-1 overflow-x-auto tab-bar-container",
      "shadow-sm transition-all duration-200", // Add subtle shadow for depth and smooth transitions
      // Adjust padding based on sidebar state - extra left padding when sidebar is collapsed for macOS stoplight buttons
      sidebarCollapsed ? "pl-20 pr-3" : "px-3",
      className
    )}>
      {/* Tabs */}
      {tabs.map((tab) => (
        <div
          key={tab.id}
          className={cn(
            "flex items-center gap-2 px-4 py-2.5 cursor-pointer",
            "min-w-[140px] max-w-[220px] group relative tab-item",
            "transition-all duration-200 ease-out", // Smoother, longer transition
            tab.isActive
              ? "bg-background-muted text-text-prominent shadow-sm active"
              : "bg-transparent text-text-muted hover:bg-background-subtle hover:text-text-standard"
          )}
          onClick={() => onTabClick(tab.id)}
          title={getTabTitle(tab)}
        >
          {/* Tab Icon */}
          <div className="flex-shrink-0 opacity-80">
            {getTabIcon(tab.type)}
          </div>

          {/* Tab Title */}
          <span className="truncate text-sm font-medium flex-1">
            {getTabTitle(tab)}
          </span>

          {/* Unsaved Changes Indicator */}
          {tab.hasUnsavedChanges && (
            <div className="w-2 h-2 bg-accent-warning rounded-full flex-shrink-0 unsaved-indicator" />
          )}

          {/* Close Button - Only show for active tab */}
          {tab.isActive && (
            <button
              onClick={(e) => {
                e.stopPropagation();
                onTabClose(tab.id);
              }}
              className={cn(
                "flex-shrink-0 p-1 rounded-md tab-close-button",
                "text-text-muted hover:text-text-standard transition-all duration-200",
                "ml-1" // Add some margin for better spacing
              )}
              title="Close tab"
            >
              <X className="w-3.5 h-3.5" />
            </button>
          )}
        </div>
      ))}

      {/* New Tab Button */}
      <button
        onClick={onNewTab}
        className={cn(
          "flex items-center justify-center w-10 h-10 rounded-lg ml-1",
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
