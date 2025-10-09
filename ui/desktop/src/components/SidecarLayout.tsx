import React, { useState, createContext, useContext, useEffect } from 'react';
import { X, FileDiff, SquareSplitHorizontal, BetweenHorizontalStart, Globe, FileText } from 'lucide-react';
import { Button } from './ui/button';
import { Tooltip, TooltipTrigger, TooltipContent } from './ui/Tooltip';
import SidecarTabs from './SidecarTabs';
import { FileViewer } from './FileViewer';

interface SidecarView {
  id: string;
  title: string;
  icon: React.ReactNode;
  content: React.ReactNode;
  fileName?: string; // Optional fileName for diff viewer
}

interface SidecarContextType {
  activeView: string | null;
  views: SidecarView[];
  showView: (view: SidecarView) => void;
  hideView: () => void;
  showDiffViewer: (diffContent: string, fileName?: string) => void;
  hideDiffViewer: () => void;
  showLocalhostViewer: (url?: string, title?: string) => void;
  hideLocalhostViewer: () => void;
  showFileViewer: (filePath: string) => void;
  hideFileViewer: () => void;
}

const SidecarContext = createContext<SidecarContextType | null>(null);

export const useSidecar = () => {
  const context = useContext(SidecarContext);
  // Return null if no context (allows optional usage)
  return context;
};

interface SidecarProviderProps {
  children: React.ReactNode;
  showSidecar?: boolean; // Control whether sidecar should be visible
}

// Monaco Editor Diff Component
function MonacoDiffViewer({ diffContent, _fileName }: { diffContent: string; _fileName: string }) {
  const [viewMode, setViewMode] = useState<'split' | 'unified'>('unified');
  const [parsedDiff, setParsedDiff] = useState<{
    beforeLines: Array<{
      content: string;
      lineNumber: number;
      type: 'context' | 'removed' | 'added';
    }>;
    afterLines: Array<{
      content: string;
      lineNumber: number;
      type: 'context' | 'removed' | 'added';
    }>;
    unifiedLines: Array<{
      content: string;
      beforeLineNumber: number | null;
      afterLineNumber: number | null;
      type: 'context' | 'removed' | 'added';
    }>;
  }>({ beforeLines: [], afterLines: [], unifiedLines: [] });

  React.useEffect(() => {
    // Parse unified diff format into before/after with line numbers
    const lines = diffContent.split('\n');
    const beforeLines: Array<{
      content: string;
      lineNumber: number;
      type: 'context' | 'removed' | 'added';
    }> = [];
    const afterLines: Array<{
      content: string;
      lineNumber: number;
      type: 'context' | 'removed' | 'added';
    }> = [];
    const unifiedLines: Array<{
      content: string;
      beforeLineNumber: number | null;
      afterLineNumber: number | null;
      type: 'context' | 'removed' | 'added';
    }> = [];

    let beforeLineNum = 1;
    let afterLineNum = 1;
    let inHunk = false;

    for (const line of lines) {
      if (line.startsWith('@@')) {
        inHunk = true;
        const match = line.match(/@@ -(\d+),?\d* \+(\d+),?\d* @@/);
        if (match) {
          beforeLineNum = parseInt(match[1]);
          afterLineNum = parseInt(match[2]);
        }
        continue;
      }

      if (!inHunk) continue;

      if (line.startsWith('-')) {
        // Removed line - only in before
        const content = line.substring(1);
        beforeLines.push({ content, lineNumber: beforeLineNum, type: 'removed' });
        unifiedLines.push({
          content,
          beforeLineNumber: beforeLineNum,
          afterLineNumber: null,
          type: 'removed',
        });
        beforeLineNum++;
      } else if (line.startsWith('+')) {
        // Added line - only in after
        const content = line.substring(1);
        afterLines.push({ content, lineNumber: afterLineNum, type: 'added' });
        unifiedLines.push({
          content,
          beforeLineNumber: null,
          afterLineNumber: afterLineNum,
          type: 'added',
        });
        afterLineNum++;
      } else if (line.startsWith(' ')) {
        // Context line - in both
        const content = line.substring(1);
        beforeLines.push({ content, lineNumber: beforeLineNum, type: 'context' });
        afterLines.push({ content, lineNumber: afterLineNum, type: 'context' });
        unifiedLines.push({
          content,
          beforeLineNumber: beforeLineNum,
          afterLineNumber: afterLineNum,
          type: 'context',
        });
        beforeLineNum++;
        afterLineNum++;
      }
    }

    setParsedDiff({ beforeLines, afterLines, unifiedLines });
  }, [diffContent, _fileName]); // Include _fileName in dependencies to satisfy TypeScript

  const renderDiffLine = (
    line: { content: string; lineNumber: number; type: 'context' | 'removed' | 'added' },
    side: 'before' | 'after'
  ) => {
    const getLineStyle = () => {
      switch (line.type) {
        case 'removed':
          return 'bg-red-500/10 border-l-2 border-red-500';
        case 'added':
          return 'bg-green-500/10 border-l-2 border-green-500';
        case 'context':
        default:
          return 'bg-transparent';
      }
    };

    const getTextColor = () => {
      switch (line.type) {
        case 'removed':
          return 'text-red-500';
        case 'added':
          return 'text-green-500';
        case 'context':
        default:
          return 'text-textStandard';
      }
    };

    const getLinePrefix = () => {
      switch (line.type) {
        case 'removed':
          return '-';
        case 'added':
          return '+';
        case 'context':
        default:
          return ' ';
      }
    };

    return (
      <div
        key={`${side}-${line.lineNumber}`}
        className={`flex font-mono text-xs ${getLineStyle()}`}
      >
        <div className="w-12 text-textSubtle text-right pr-2 py-1 select-none flex-shrink-0">
          {line.lineNumber}
        </div>
        <div className="w-4 text-textSubtle text-center py-1 select-none flex-shrink-0">
          {getLinePrefix()}
        </div>
        <div className={`flex-1 py-1 pr-4 ${getTextColor()}`}>
          <code>{line.content || ' '}</code>
        </div>
      </div>
    );
  };

  const renderUnifiedLine = (
    line: {
      content: string;
      beforeLineNumber: number | null;
      afterLineNumber: number | null;
      type: 'context' | 'removed' | 'added';
    },
    index: number
  ) => {
    const getLineStyle = () => {
      switch (line.type) {
        case 'removed':
          return 'bg-red-500/10 border-l-2 border-red-500';
        case 'added':
          return 'bg-green-500/10 border-l-2 border-green-500';
        case 'context':
        default:
          return 'bg-transparent';
      }
    };

    const getTextColor = () => {
      switch (line.type) {
        case 'removed':
          return 'text-red-500';
        case 'added':
          return 'text-green-500';
        case 'context':
        default:
          return 'text-textStandard';
      }
    };

    const getLinePrefix = () => {
      switch (line.type) {
        case 'removed':
          return '-';
        case 'added':
          return '+';
        case 'context':
        default:
          return ' ';
      }
    };

    return (
      <div key={`unified-${index}`} className={`flex font-mono text-xs ${getLineStyle()}`}>
        <div className="w-12 text-textSubtle text-right pr-1 py-1 select-none flex-shrink-0">
          {line.beforeLineNumber || ''}
        </div>
        <div className="w-12 text-textSubtle text-right pr-2 py-1 select-none flex-shrink-0">
          {line.afterLineNumber || ''}
        </div>
        <div className="w-4 text-textSubtle text-center py-1 select-none flex-shrink-0">
          {getLinePrefix()}
        </div>
        <div className={`flex-1 py-1 pr-4 ${getTextColor()}`}>
          <code>{line.content || ' '}</code>
        </div>
      </div>
    );
  };

  // Expose the view mode controls to parent
  useEffect(() => {
    // Store the setViewMode function in a way the parent can access it
    (
      window as unknown as {
        diffViewerControls?: { viewMode: string; setViewMode: (mode: 'split' | 'unified') => void };
      }
    ).diffViewerControls = { viewMode, setViewMode };
  }, [viewMode, setViewMode]);

  return (
    <div className="h-full flex flex-col bg-background-default ">
      {viewMode === 'split' ? (
        /* Split Diff Content */
        <div className="flex-1 overflow-auto flex">
          {/* Before (Left Side) */}
          <div className="flex-1 border-r border-borderSubtle">
            <div className="py-2  text-textStandard text-xs font-mono text-center border-b-1 border-borderSubtle">
              Before
            </div>
            <div>{parsedDiff.beforeLines.map((line) => renderDiffLine(line, 'before'))}</div>
          </div>

          {/* After (Right Side) */}
          <div className="flex-1">
            <div className="py-2  text-textStandard text-xs font-mono text-center border-b-1 border-borderSubtle">
              After
            </div>
            <div>{parsedDiff.afterLines.map((line) => renderDiffLine(line, 'after'))}</div>
          </div>
        </div>
      ) : (
        /* Unified Diff Content */
        <div className="flex-1 overflow-hidden">
          <div className="h-full overflow-auto pb-(--radius-2xl)">
            {parsedDiff.unifiedLines.map((line, index) => renderUnifiedLine(line, index))}
          </div>
        </div>
      )}
    </div>
  );
}

export function SidecarProvider({ children, showSidecar = true }: SidecarProviderProps) {
  const [activeView, setActiveView] = useState<string | null>(null);
  const [views, setViews] = useState<SidecarView[]>([]);

  const showView = async (view: SidecarView) => {
    setViews((prev) => {
      const existing = prev.find((v) => v.id === view.id);
      if (existing) {
        return prev.map((v) => (v.id === view.id ? view : v));
      }
      return [...prev, view];
    });

    setActiveView(view.id);
  };

  const hideView = () => {
    setActiveView(null);
  };

  const showDiffViewer = (content: string, fileName = 'File') => {
    const diffView: SidecarView = {
      id: 'diff',
      title: 'Diff Viewer',
      icon: <FileDiff size={16} />,
      content: <MonacoDiffViewer diffContent={content} _fileName={fileName} />,
      fileName: fileName, // Store fileName for header display
    };
    showView(diffView);
  };

  const hideDiffViewer = () => {
    setViews((prev) => prev.filter((v) => v.id !== 'diff'));
    if (activeView === 'diff') {
      setActiveView(null);
    }
  };

  const showLocalhostViewer = (url = 'http://localhost:3000', title = 'Localhost Viewer') => {
    const localhostView: SidecarView = {
      id: 'localhost',
      title: title,
      icon: <Globe size={16} />,
      content: <SidecarTabs initialUrl={url} />,
      fileName: url,
    };
    showView(localhostView);
  };

  const hideLocalhostViewer = () => {
    setViews((prev) => prev.filter((v) => v.id !== 'localhost'));
    if (activeView === 'localhost') {
      setActiveView(null);
    }
  };

  const showFileViewer = (filePath: string) => {
    const fileName = filePath.split('/').pop() || filePath;
    const fileView: SidecarView = {
      id: 'file',
      title: 'File Viewer',
      icon: <FileText size={16} />,
      content: <FileViewer filePath={filePath} />,
      fileName: fileName,
    };
    showView(fileView);
  };

  const hideFileViewer = () => {
    setViews((prev) => prev.filter((v) => v.id !== 'file'));
    if (activeView === 'file') {
      setActiveView(null);
    }
  };

  const contextValue: SidecarContextType = {
    activeView,
    views,
    showView,
    hideView,
    showDiffViewer,
    hideDiffViewer,
    showLocalhostViewer,
    hideLocalhostViewer,
    showFileViewer,
    hideFileViewer,
  };

  // Don't render sidecar if showSidecar is false
  if (!showSidecar) {
    return <SidecarContext.Provider value={contextValue}>{children}</SidecarContext.Provider>;
  }

  // Just provide context, layout will be handled by MainPanelLayout
  return <SidecarContext.Provider value={contextValue}>{children}</SidecarContext.Provider>;
}

// Separate Sidecar component that can be used as a sibling
export function Sidecar({ className = '' }: { className?: string }) {
  const sidecar = useSidecar();
  const [viewMode, setViewMode] = useState<'split' | 'unified'>('unified');

  // Update the diff viewer when view mode changes
  useEffect(() => {
    if (sidecar) {
      const { activeView, views } = sidecar;
      const currentView = views.find((v) => v.id === activeView);
      const isDiffViewer = currentView?.id === 'diff';

      if (
        isDiffViewer &&
        (
          window as unknown as {
            diffViewerControls?: {
              viewMode: string;
              setViewMode: (mode: 'split' | 'unified') => void;
            };
          }
        ).diffViewerControls
      ) {
        (
          window as unknown as {
            diffViewerControls?: {
              viewMode: string;
              setViewMode: (mode: 'split' | 'unified') => void;
            };
          }
        ).diffViewerControls!.setViewMode(viewMode);
      }
    }
  }, [viewMode, sidecar]);

  if (!sidecar) return null;

  const { activeView, views, hideView } = sidecar;
  const currentView = views.find((v) => v.id === activeView);
  const isVisible = activeView && currentView;

  if (!isVisible) return null;

  // Check if current view is diff viewer
  const isDiffViewer = currentView.id === 'diff';

  return (
    <div
      className={`bg-background-default overflow-hidden rounded-xl flex flex-col h-full ${className}`}
      style={{
        marginLeft: '0px',
        height: '100%',
      }}
    >
      {currentView && (
        <>
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
                    onClick={hideView}
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
          <div className="flex-1  overflow-hidden rounded-xl">
            {currentView.content}
          </div>
        </>
      )}
    </div>
  );
}
