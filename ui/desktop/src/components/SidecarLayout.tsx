import React, { useState, createContext, useContext } from 'react';
import { X, GitBranch } from 'lucide-react';
import { Button } from './ui/button';

interface SidecarView {
  id: string;
  title: string;
  icon: React.ReactNode;
  content: React.ReactNode;
}

interface SidecarContextType {
  activeView: string | null;
  views: SidecarView[];
  showView: (view: SidecarView) => void;
  hideView: () => void;
  showDiffViewer: (diffContent: string, fileName?: string) => void;
  hideDiffViewer: () => void;
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
function MonacoDiffViewer({ diffContent, fileName }: { diffContent: string; fileName: string }) {
  const [viewMode, setViewMode] = useState<'split' | 'unified'>('split');
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
  }, [diffContent]);

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
          return 'text-red-400';
        case 'added':
          return 'text-green-400';
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
        className={`flex font-mono text-sm ${getLineStyle()}`}
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
          return 'text-red-400';
        case 'added':
          return 'text-green-400';
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
      <div key={`unified-${index}`} className={`flex font-mono text-sm ${getLineStyle()}`}>
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

  return (
    <div className="h-full flex flex-col bg-background-default">
      {/* Header */}
      <div className="flex items-center justify-between p-4 border-b border-borderSubtle">
        <div className="flex items-center space-x-2">
          <GitBranch size={16} className="text-primary" />
          <span className="text-textStandard font-medium">{fileName}</span>
        </div>

        {/* View Mode Toggle */}
        <div className="flex items-center space-x-1 bg-background-muted rounded-md p-1">
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setViewMode('split')}
            className={`px-3 py-1 text-xs ${
              viewMode === 'split'
                ? 'bg-background-subtle text-textStandard'
                : 'text-textSubtle hover:text-textStandard hover:bg-background-subtle'
            }`}
          >
            Split
          </Button>
          <Button
            variant="ghost"
            size="sm"
            onClick={() => setViewMode('unified')}
            className={`px-3 py-1 text-xs ${
              viewMode === 'unified'
                ? 'bg-background-subtle text-textStandard'
                : 'text-textSubtle hover:text-textStandard hover:bg-background-subtle'
            }`}
          >
            Unified
          </Button>
        </div>
      </div>

      {/* Diff Content */}
      {viewMode === 'split' ? (
        /* Split Diff Content */
        <div className="flex-1 overflow-hidden flex">
          {/* Before (Left Side) */}
          <div className="flex-1 border-r border-borderSubtle">
            <div className="bg-background-muted text-textStandard px-4 py-2 text-sm font-medium border-b border-borderSubtle">
              Before
            </div>
            <div className="h-[calc(100%-40px)] overflow-auto">
              {parsedDiff.beforeLines.map((line) => renderDiffLine(line, 'before'))}
            </div>
          </div>

          {/* After (Right Side) */}
          <div className="flex-1">
            <div className="bg-background-muted text-textStandard px-4 py-2 text-sm font-medium border-b border-borderSubtle">
              After
            </div>
            <div className="h-[calc(100%-40px)] overflow-auto">
              {parsedDiff.afterLines.map((line) => renderDiffLine(line, 'after'))}
            </div>
          </div>
        </div>
      ) : (
        /* Unified Diff Content */
        <div className="flex-1 overflow-hidden">
          <div className="h-full overflow-auto">
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

  const showView = (view: SidecarView) => {
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
      icon: <GitBranch size={16} />,
      content: <MonacoDiffViewer diffContent={content} fileName={fileName} />,
    };
    showView(diffView);
  };

  const hideDiffViewer = () => {
    setViews((prev) => prev.filter((v) => v.id !== 'diff'));
    if (activeView === 'diff') {
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
  };

  const currentView = views.find((v) => v.id === activeView);

  // Don't render sidecar if showSidecar is false
  if (!showSidecar) {
    return <SidecarContext.Provider value={contextValue}>{children}</SidecarContext.Provider>;
  }

  return (
    <SidecarContext.Provider value={contextValue}>
      <div className="flex h-full relative">
        {/* Main Content */}
        <div className={`flex-1 transition-all duration-300 ${activeView ? 'mr-[700px]' : ''}`}>
          {children}
        </div>

        {/* Collapsed Sidecar Panel - Only visible when not expanded */}
        {!activeView && (
          <div className="fixed top-0 right-0 h-full w-16 bg-background-default border-l border-borderSubtle opacity-0 pointer-events-none" />
        )}

        {/* Expanded Sidecar Panel - Only visible when there's an active view */}
        {activeView && currentView && (
          <div className="fixed right-0 top-0 h-full w-[700px] bg-background-default border-l border-borderSubtle transition-transform duration-300">
            {/* Sidecar Header */}
            <div className="flex items-center justify-between p-4 border-b border-borderSubtle">
              <div className="flex items-center space-x-2">
                {currentView.icon}
                <span className="text-textStandard font-medium">{currentView.title}</span>
              </div>
              <Button
                variant="ghost"
                size="sm"
                onClick={hideView}
                className="text-textSubtle hover:text-textStandard"
              >
                <X size={16} />
              </Button>
            </div>

            {/* Sidecar Content */}
            <div className="h-[calc(100%-60px)] overflow-hidden">{currentView.content}</div>
          </div>
        )}
      </div>
    </SidecarContext.Provider>
  );
}
