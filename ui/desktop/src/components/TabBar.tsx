import React, { useState, useEffect } from 'react';
import { X, Plus, MessageCircle, Bot, Users, Calendar, Target, Folder } from 'lucide-react';
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

// Tab tooltip component with session details and workspace directory
const TabTooltip: React.FC<{ tab: Tab; children: React.ReactNode; workingDirectory?: string }> = ({ tab, children, workingDirectory }) => {
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
          
          {/* Workspace directory */}
          {workingDirectory && (
            <div className="flex items-center gap-1.5 text-xs opacity-90">
              <Folder className="w-3 h-3" />
              <span className="truncate">{workingDirectory}</span>
            </div>
          )}
          
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
  sidebarCollapsed = false,
  workingDirectory
}) => {
  return (
    <div className={cn(
      "flex items-center",
      "min-h-[48px] gap-2 overflow-x-auto tab-bar-container",
      "transition-all duration-200", 
      // Adjust padding based on sidebar state - extra left padding when sidebar is collapsed for macOS stoplight buttons
      sidebarCollapsed ? "pl-20 pr-3" : "px-3",
      className
    )}>
      {/* Tabs */}
      {tabs.map((tab) => (
        <TabTooltip key={tab.id} tab={tab} workingDirectory={workingDirectory}>
          <button
            className={cn(
              "h-8 cursor-pointer no-drag border-0 rounded-2xl flex items-center",
              "w-[160px] group relative tab-item",
              "transition-all duration-200 ease-out",
              tab.isActive
                ? "bg-[#33333A]/90 dark:bg-[#33333A]/90"
                : "bg-[#0E0E13]/90 dark:bg-[#0E0E13]/90 hover:bg-[#0E0E13] dark:hover:bg-[#0E0E13]"
            )}
            onClick={() => onTabClick(tab.id)}
          >
            {/* Tab Title - with explicit padding */}
            <div className="flex-1 min-w-0 pl-3">
              <span className={cn(
                "truncate text-xs font-normal block pointer-events-none text-left",
                tab.isActive
                  ? "text-slate-200"
                  : "text-neutral-700"
              )}>
                {getTabTitle(tab)}
              </span>
            </div>

            {/* Close Button - Always reserve space, only show icon when active */}
            <div className="flex-shrink-0 w-3 pr-3 flex items-center justify-center">
              {tab.isActive && (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    onTabClose(tab.id);
                  }}
                  className="flex items-center justify-center tab-close-button pointer-events-auto"
                  title="Close tab"
                >
                  <X className="w-3 h-3 text-white" />
                </button>
              )}
            </div>
          </button>
        </TabTooltip>
      ))}

      {/* New Tab Button */}
      <button
        onClick={onNewTab}
        className={cn(
          "flex items-center justify-center w-8 h-8 rounded-2xl",
          "bg-[#0E0E13]/90 hover:bg-[#0E0E13] transition-all duration-200",
          "text-neutral-700 hover:text-neutral-600"
        )}
        title="New tab (Ctrl+T)"
      >
        <Plus className="w-4 h-4" />
      </button>

      {/* Spacer to push content left */}
      <div className="flex-1" />
      
      {/* Optional future controls can go here */}
    </div>
  );
};

export default TabBar;
