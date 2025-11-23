import React, { useState, useEffect } from 'react';
import { X, Plus, MessageCircle, Bot, Users, Calendar, Target } from 'lucide-react';
import { cn } from '../utils';
import { Tooltip, TooltipContent, TooltipTrigger } from './ui/Tooltip';
import { getSession } from '../api';
import { formatMessageTimestamp } from '../utils/timeUtils';
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

// Tab tooltip component with session details
const TabTooltip: React.FC<{ tab: Tab; children: React.ReactNode }> = ({ tab, children }) => {
  const [sessionData, setSessionData] = useState<{
    messageCount: number;
    totalTokens: number;
    createdAt: string | null;
  } | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  useEffect(() => {
    // Only fetch session data for non-temporary sessions
    if (!tab.sessionId || tab.sessionId.startsWith('temp_') || tab.sessionId.startsWith('new_')) {
      return;
    }

    const fetchSessionData = async () => {
      setIsLoading(true);
      try {
        const response = await getSession({ path: { session_id: tab.sessionId } });
        if (response.data) {
          setSessionData({
            messageCount: response.data.message_count || 0,
            totalTokens: response.data.total_tokens || 0,
            createdAt: response.data.conversation?.[0]?.created || null,
          });
        }
      } catch (error) {
        console.error('Failed to fetch session data for tooltip:', error);
      } finally {
        setIsLoading(false);
      }
    };

    fetchSessionData();
  }, [tab.sessionId]);

  return (
    <Tooltip delayDuration={500}>
      <TooltipTrigger asChild>
        {children}
      </TooltipTrigger>
      <TooltipContent className="max-w-sm">
        <div className="space-y-2">
          {/* Full tab title */}
          <div className="font-medium text-sm">{getTabTitle(tab)}</div>
          
          {/* Session metadata */}
          {sessionData && !isLoading && (
            <div className="flex flex-col gap-1 text-xs opacity-90">
              {sessionData.createdAt && (
                <div className="flex items-center gap-1.5">
                  <Calendar className="w-3 h-3" />
                  <span>{formatMessageTimestamp(sessionData.createdAt)}</span>
                </div>
              )}
              <div className="flex items-center gap-1.5">
                <MessageCircle className="w-3 h-3" />
                <span>{sessionData.messageCount} messages</span>
              </div>
              {sessionData.totalTokens > 0 && (
                <div className="flex items-center gap-1.5">
                  <Target className="w-3 h-3" />
                  <span>{sessionData.totalTokens.toLocaleString()} tokens</span>
                </div>
              )}
            </div>
          )}
          
          {/* Loading state */}
          {isLoading && (
            <div className="text-xs opacity-70">Loading session details...</div>
          )}
          
          {/* New session indicator */}
          {(!sessionData || tab.sessionId.startsWith('temp_') || tab.sessionId.startsWith('new_')) && !isLoading && (
            <div className="text-xs opacity-70">New session</div>
          )}
        </div>
      </TooltipContent>
    </Tooltip>
  );
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
        <TabTooltip key={tab.id} tab={tab}>
          <div
            className={cn(
              "flex items-center gap-2 px-4 py-2.5 cursor-pointer",
              "min-w-[120px] max-w-[180px] group relative tab-item",
              "transition-all duration-200 ease-out", // Smoother, longer transition
              tab.isActive
                ? "bg-background-muted text-text-prominent shadow-sm active"
                : "bg-transparent text-text-muted hover:bg-background-subtle hover:text-text-standard"
            )}
            onClick={() => onTabClick(tab.id)}
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
                  "flex-shrink-0 p-1.5 rounded-md tab-close-button",
                  "text-text-muted hover:text-text-standard transition-all duration-200",
                  "ml-1 -mr-1" // Add some margin for better spacing and extend clickable area
                )}
                title="Close tab"
              >
                <X className="w-4 h-4" />
              </button>
            )}
          </div>
        </TabTooltip>
      ))}

      {/* New Tab Button */}
      <button
        onClick={onNewTab}
        className={cn(
          "flex items-center justify-center w-10 h-10 rounded-full ml-1 flex-shrink-0",
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
      
      {/* Right spacer to ensure plus button stays accessible - only show when sidebar is collapsed */}
      {sidebarCollapsed && <div className="flex-shrink-0 w-[200px]" />}
    </div>
  );
};
