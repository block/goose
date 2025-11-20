import React, { useState } from 'react';
import { X, SquareSplitHorizontal, BetweenHorizontalStart, FileDiff, Globe, FileText, Edit } from 'lucide-react';
import { Button } from './ui/button';
import { Tooltip, TooltipTrigger, TooltipContent } from './ui/Tooltip';
import { TabSidecarState } from './TabBar';

interface TabSidecarProps {
  sidecarState: TabSidecarState;
  onHideView: (viewId: string) => void;
  className?: string;
}

// Component renderers for different sidecar types
const MonacoDiffViewer: React.FC<{ diffContent: string }> = ({ diffContent }) => (
  <div className="h-full p-4 bg-background-default overflow-auto">
    <pre className="text-sm text-textStandard whitespace-pre-wrap font-mono">{diffContent}</pre>
  </div>
);

const LocalhostViewer: React.FC<{ url: string; title: string }> = ({ url, title }) => (
  <div className="h-full">
    <iframe 
      src={url} 
      className="w-full h-full border-0" 
      title={title}
      sandbox="allow-same-origin allow-scripts allow-forms allow-popups"
    />
  </div>
);

const SimpleFileViewer: React.FC<{ path: string }> = ({ path }) => (
  <div className="h-full p-4 bg-background-default">
    <div className="text-sm text-textMuted mb-2">File: {path}</div>
    <div className="text-sm text-textStandard">
      File viewer for: {path}
      <br />
      (Content loading would be implemented here)
    </div>
  </div>
);

const SimpleDocumentEditor: React.FC<{ path?: string; content?: string }> = ({ path, content }) => (
  <div className="h-full p-4 bg-background-default flex flex-col">
    <div className="text-sm text-textMuted mb-2">
      {path ? `Editing: ${path}` : 'New Document'}
    </div>
    <textarea 
      className="flex-1 w-full border border-borderSubtle rounded p-2 text-sm resize-none"
      placeholder="Start writing your document..."
      defaultValue={content || ''}
    />
  </div>
);

// Icon renderer
const renderIcon = (iconType: string) => {
  switch (iconType) {
    case 'diff':
      return <FileDiff size={16} />;
    case 'localhost':
      return <Globe size={16} />;
    case 'file':
      return <FileText size={16} />;
    case 'editor':
      return <Edit size={16} />;
    default:
      return <FileText size={16} />;
  }
};

// Content renderer
const renderContent = (contentType: string, contentProps: Record<string, any>) => {
  switch (contentType) {
    case 'diff':
      return <MonacoDiffViewer diffContent={contentProps.diffContent || ''} />;
    case 'localhost':
      return <LocalhostViewer url={contentProps.url || 'http://localhost:3000'} title={contentProps.title || 'Localhost Viewer'} />;
    case 'file':
      return <SimpleFileViewer path={contentProps.path || ''} />;
    case 'editor':
      return <SimpleDocumentEditor path={contentProps.path} content={contentProps.content} />;
    default:
      return <div className="h-full p-4 bg-background-default">Unknown content type: {contentType}</div>;
  }
};

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
  const isDiffViewer = currentView.contentType === 'diff';

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
            {renderIcon(currentView.iconType)}
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
          {renderContent(currentView.contentType, currentView.contentProps)}
        </div>
      </div>
    </div>
  );
};
