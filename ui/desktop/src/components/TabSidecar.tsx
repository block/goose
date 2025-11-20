import React, { useState } from 'react';
import { X, SquareSplitHorizontal, BetweenHorizontalStart } from 'lucide-react';
import { Button } from './ui/button';
import { Tooltip, TooltipTrigger, TooltipContent } from './ui/Tooltip';
import { TabSidecarState } from './TabBar';

interface TabSidecarProps {
  sidecarState: TabSidecarState;
  onHideView: (viewId: string) => void;
  className?: string;
}

export const TabSidecar: React.FC<TabSidecarProps> = ({
  sidecarState,
  onHideView,
  className = ''
}) => {
  const [viewMode, setViewMode] = useState<'split' | 'unified'>('unified');

  // Get the first active view (for now, we'll show one at a time)
  const currentViewId = sidecarState.activeViews[0];
  const currentView = sidecarState.views.find(v => v.id === currentViewId);

  if (!currentView || !sidecarState.activeViews.includes(currentView.id)) {
    return null;
  }

  // Check if current view is diff viewer
  const isDiffViewer = currentView.id.startsWith('diff');

  // Update the diff viewer when view mode changes
  React.useEffect(() => {
    if (isDiffViewer && (window as any).diffViewerControls) {
      (window as any).diffViewerControls.setViewMode(viewMode);
    }
  }, [viewMode, isDiffViewer]);

  return (
    <div className="h-full w-full rounded-2xl shadow-2xl drop-shadow-2xl border border-border-subtle overflow-hidden">
      <div
        className={`bg-background-default overflow-hidden flex flex-col h-full ${className}`}
      >
        {/* Sidecar Header */}
        <div className="flex items-center justify-between p-4 border-b border-borderSubtle flex-shrink-0 flex-grow-0">
          <div className="flex items-center space-x-2">
            {currentView.icon}
            <div className="flex flex-col">
              <span className="text-textStandard font-medium">{currentView.title}</span>
              {currentView.fileName && (
                <span className="text-xs font-mono text-text-muted">{currentView.fileName}</span>
              )}
            </div>
          </div>

          <div className="flex items-center space-x-2">
            {/* View Mode Toggle - Only show for diff viewer */}
            {isDiffViewer && (
              <div className="flex items-center space-x-1 bg-background-muted rounded-lg p-1">
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => setViewMode('unified')}
                      className={`px-2 py-1 cursor-pointer focus:outline-none focus:ring-2 focus:ring-borderProminent focus:ring-offset-1 ${
                        viewMode === 'unified'
                          ? 'bg-background-default text-textStandard hover:bg-background-default dark:hover:bg-background-default'
                          : 'text-textSubtle'
                      }`}
                    >
                      <BetweenHorizontalStart size={14} />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent side="bottom" sideOffset={8}>
                    Unified View
                  </TooltipContent>
                </Tooltip>

                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => setViewMode('split')}
                      className={`px-2 py-1 cursor-pointer focus:outline-none focus:ring-2 focus:ring-borderProminent focus:ring-offset-1  ${
                        viewMode === 'split'
                          ? 'bg-background-default text-textStandard hover:bg-background-default dark:hover:bg-background-default'
                          : 'text-textSubtle'
                      }`}
                    >
                      <SquareSplitHorizontal size={14} />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent side="bottom" sideOffset={8}>
                    Split View
                  </TooltipContent>
                </Tooltip>
              </div>
            )}

            {/* Close Button */}
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => onHideView(currentView.id)}
                  className="text-textSubtle hover:text-textStandard cursor-pointer focus:outline-none focus:ring-2 focus:ring-borderProminent focus:ring-offset-1"
                >
                  <X size={16} />
                </Button>
              </TooltipTrigger>
              <TooltipContent side="bottom">Close</TooltipContent>
            </Tooltip>
          </div>
        </div>

        {/* Sidecar Content */}
        <div className="flex-1 overflow-hidden">
          {currentView.content}
        </div>
      </div>
    </div>
  );
};
