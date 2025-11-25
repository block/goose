import React, { useState, useEffect } from 'react';
import { X, SquareSplitHorizontal, BetweenHorizontalStart, FileDiff, Globe, FileText, Edit, Monitor } from 'lucide-react';
import { Button } from './ui/button';
import { Tooltip, TooltipTrigger, TooltipContent } from './ui/Tooltip';
import { TabSidecarState } from './TabBar';
import DocumentEditor from './DocumentEditor';
import WebBrowser from './WebBrowser';
import { useUnifiedSidecarContextOptional } from '../contexts/UnifiedSidecarContext';

interface TabSidecarProps {
  sidecarState: TabSidecarState;
  onHideView: (viewId: string) => void;
  tabId: string; // Add tabId to ensure proper component isolation
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

const RichDocumentEditor: React.FC<{ path?: string; content?: string }> = ({ path, content }) => (
  <div className="h-full">
    <DocumentEditor
      filePath={path}
      initialContent={content}
      placeholder="Start writing your document..."
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
    case 'web':
      return <Monitor size={16} />;
    case 'file':
      return <FileText size={16} />;
    case 'editor':
      return <Edit size={16} />;
    default:
      return <FileText size={16} />;
  }
};

// Content renderer
const renderContent = (contentType: string, contentProps: Record<string, any>, tabId: string, onClose?: () => void) => {
  switch (contentType) {
    case 'diff':
      return <MonacoDiffViewer key={`diff-${tabId}`} diffContent={contentProps.diffContent || ''} />;
    case 'localhost':
      return <LocalhostViewer key={`localhost-${tabId}`} initialUrl={contentProps.url || 'http://localhost:3000'} onClose={onClose} />;
    case 'web':
      return <WebViewer key={`web-${tabId}`} initialUrl={contentProps.url || 'https://google.com'} allowAllSites={true} onClose={onClose} />;
    case 'file':
      return <SimpleFileViewer key={`file-${tabId}`} path={contentProps.path || ''} />;
    case 'editor':
      return <RichDocumentEditor key={`editor-${tabId}`} path={contentProps.path} content={contentProps.content} />;
    default:
      return <div key={`unknown-${tabId}`} className="h-full p-4 bg-background-default">Unknown content type: {contentType}</div>;
  }
};

export const TabSidecar: React.FC<TabSidecarProps> = ({
  sidecarState,
  onHideView,
  tabId,
  className = ''
}) => {
  const [viewMode, setViewMode] = useState<'split' | 'unified'>('unified');
  const unifiedSidecarContext = useUnifiedSidecarContextOptional();

  // Get the first active view (for now, we'll show one at a time)
  const currentViewId = sidecarState.activeViews[0];
  const currentView = sidecarState.views.find(v => v.id === currentViewId);

  // Register the current view with UnifiedSidecarContext for AI awareness
  useEffect(() => {
    if (!unifiedSidecarContext || !currentView) {
      return;
    }

    console.log('🔧 TabSidecar: Registering view with unified context:', currentView.id, currentView.contentType);

    // Create sidecar info based on content type
    let sidecarInfo;
    const sidecarId = `tab-${tabId}-${currentView.id}`;

    switch (currentView.contentType) {
      case 'diff':
        const diffLines = (currentView.contentProps.diffContent || '').split('\n');
        const addedLines = diffLines.filter((line: string) => line.startsWith('+')).length;
        const removedLines = diffLines.filter((line: string) => line.startsWith('-')).length;
        
        sidecarInfo = {
          id: sidecarId,
          type: 'diff-viewer' as const,
          title: currentView.title || 'Diff Viewer',
          fileName: currentView.fileName || 'File',
          filePath: undefined,
          addedLines,
          removedLines,
          totalChanges: addedLines + removedLines,
          viewMode: viewMode,
          timestamp: Date.now(),
        };
        break;

      case 'localhost':
        const localhostUrl = currentView.contentProps.url || 'http://localhost:3000';
        const port = parseInt(new URL(localhostUrl).port) || 3000;
        
        sidecarInfo = {
          id: sidecarId,
          type: 'localhost-viewer' as const,
          title: currentView.title || 'Localhost Viewer',
          url: localhostUrl,
          port,
          protocol: new URL(localhostUrl).protocol.replace(':', '') as 'http' | 'https',
          isLocal: true,
          serviceType: 'development',
          timestamp: Date.now(),
        };
        break;

      case 'web':
        // Note: WebBrowser component handles its own registration
        // Skip registration here to avoid duplicates
        console.log('🔧 TabSidecar: Skipping web viewer registration (handled by WebBrowser component)');
        break;

      case 'file':
        const filePath = currentView.contentProps.path || '';
        const fileName = filePath.split('/').pop() || filePath;
        const fileExtension = fileName.split('.').pop() || '';
        
        sidecarInfo = {
          id: sidecarId,
          type: 'file-viewer' as const,
          title: currentView.title || 'File Viewer',
          filePath,
          fileName,
          fileSize: 0, // Would need to fetch from file system
          fileType: fileExtension,
          isReadable: true,
          lastModified: Date.now(),
          timestamp: Date.now(),
        };
        break;

      case 'editor':
        const editorPath = currentView.contentProps.path;
        const editorFileName = editorPath 
          ? editorPath.split('/').pop() || editorPath 
          : currentView.fileName || 'Untitled Document';
        
        sidecarInfo = {
          id: sidecarId,
          type: 'document-editor' as const,
          title: currentView.title || 'Document Editor',
          filePath: editorPath,
          fileName: editorFileName,
          contentLength: (currentView.contentProps.content || '').length,
          hasUnsavedChanges: false, // Would need to track this
          isNewDocument: !editorPath,
          language: editorPath ? editorPath.split('.').pop() : undefined,
          timestamp: Date.now(),
        };
        break;
    }

    if (sidecarInfo) {
      unifiedSidecarContext.registerSidecar(sidecarInfo);
      console.log('🔧 TabSidecar: Registered sidecar:', sidecarInfo.id, sidecarInfo.type);
    }

    // Cleanup: unregister when view changes or component unmounts
    return () => {
      if (sidecarInfo) {
        unifiedSidecarContext.unregisterSidecar(sidecarId);
        console.log('🔧 TabSidecar: Unregistered sidecar:', sidecarId);
      }
    };
  }, [unifiedSidecarContext, currentView, tabId, viewMode]);

  if (!currentView || !sidecarState.activeViews.includes(currentView.id)) {
    return null;
  }

  // Check if current view is diff viewer
  const isDiffViewer = currentView.contentType === 'diff';
  const isWebViewer = currentView.contentType === 'web' || currentView.contentType === 'localhost';

  // Update the diff viewer when view mode changes
  React.useEffect(() => {
    if (isDiffViewer && (window as any).diffViewerControls) {
      (window as any).diffViewerControls.setViewMode(viewMode);
    }
  }, [viewMode, isDiffViewer]);

  return (
    <div className="h-full w-full shadow-2xl drop-shadow-2xl overflow-hidden">
      <div
        className={`bg-background-default overflow-hidden flex flex-col h-full ${className}`}
      >
        {/* Sidecar Header - Hidden for web viewers */}
        {!isWebViewer && (
          <div className="flex items-center justify-between px-4 py-2 border-b border-borderSubtle flex-shrink-0 flex-grow-0">
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
        )}

        {/* Sidecar Content */}
        <div className="flex-1 overflow-hidden">
          {renderContent(currentView.contentType, currentView.contentProps, tabId, () => onHideView(currentView.id))}
        </div>
      </div>
    </div>
  );
};
