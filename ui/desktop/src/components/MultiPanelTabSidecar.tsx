import React, { useState, useCallback, useRef, useEffect } from 'react';
import { motion, AnimatePresence, useDragControls } from 'framer-motion';
import { 
  X, 
  SquareSplitHorizontal, 
  BetweenHorizontalStart, 
  FileDiff, 
  Globe, 
  FileText, 
  Edit, 
  Monitor,
  MoreVertical,
  Plus,
  GripVertical,
  Move
} from 'lucide-react';
import { Button } from './ui/button';
import { Tooltip, TooltipTrigger, TooltipContent } from './ui/Tooltip';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from './ui/dropdown-menu';
import { TabSidecarState, TabSidecarView } from './TabBar';
import DocumentEditor from './DocumentEditor';
import WebBrowser from './WebBrowser';
import { useUnifiedSidecarContextOptional } from '../contexts/UnifiedSidecarContext';

export type SidecarLayoutMode = 'single' | 'columns' | 'rows' | 'grid' | 'custom';

interface MultiPanelTabSidecarProps {
  sidecarState: TabSidecarState;
  onHideView: (viewId: string) => void;
  onShowView?: (view: TabSidecarView) => void;
  tabId: string;
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
const renderContent = (contentType: string, contentProps: Record<string, any>, tabId: string, viewId: string) => {
  const key = `${contentType}-${tabId}-${viewId}`;
  
  switch (contentType) {
    case 'diff':
      return <MonacoDiffViewer key={key} diffContent={contentProps.diffContent || ''} />;
    case 'localhost':
      return <LocalhostViewer key={key} url={contentProps.url || 'http://localhost:3000'} title={contentProps.title || 'Localhost Viewer'} />;
    case 'web':
      return <WebBrowser key={key} initialUrl={contentProps.url || 'https://google.com'} title={contentProps.title || 'Web Browser'} />;
    case 'file':
      return <SimpleFileViewer key={key} path={contentProps.path || ''} />;
    case 'editor':
      return <RichDocumentEditor key={key} path={contentProps.path} content={contentProps.content} />;
    default:
      return <div key={key} className="h-full p-4 bg-background-default">Unknown content type: {contentType}</div>;
  }
};

// Drop zone types
type DropZoneType = 'left' | 'right' | 'top' | 'bottom' | 'center' | 'grid-1' | 'grid-2' | 'grid-3' | 'grid-4';

interface DropZone {
  type: DropZoneType;
  bounds: { x: number; y: number; width: number; height: number };
  layoutMode: SidecarLayoutMode;
  position?: number;
}

// Individual Panel Component with drag functionality
interface PanelProps {
  view: TabSidecarView;
  tabId: string;
  onHideView: (viewId: string) => void;
  onPanelDrop?: (draggedViewId: string, dropZone: DropZone) => void;
  layoutMode: SidecarLayoutMode;
  style?: React.CSSProperties;
  className?: string;
  isDragging?: boolean;
  position?: number;
}

const Panel: React.FC<PanelProps> = ({ 
  view, 
  tabId, 
  onHideView, 
  onPanelDrop,
  layoutMode, 
  style, 
  className,
  isDragging = false,
  position = 0
}) => {
  const [viewMode, setViewMode] = useState<'split' | 'unified'>('unified');
  const [isLocalDragging, setIsLocalDragging] = useState(false);
  const dragControls = useDragControls();
  const panelRef = useRef<HTMLDivElement>(null);
  
  const isDiffViewer = view.contentType === 'diff';

  // Update the diff viewer when view mode changes
  React.useEffect(() => {
    if (isDiffViewer && (window as any).diffViewerControls) {
      (window as any).diffViewerControls.setViewMode(viewMode);
    }
  }, [viewMode, isDiffViewer]);

  const handleDragStart = () => {
    setIsLocalDragging(true);
  };

  const handleDragEnd = (event: any, info: any) => {
    setIsLocalDragging(false);
    
    if (!onPanelDrop || !panelRef.current) return;

    // Get the drop position
    const dropX = info.point.x;
    const dropY = info.point.y;

    // Find the sidecar container to determine drop zones
    const sidecarContainer = panelRef.current.closest('.sidecar-container');
    if (!sidecarContainer) return;

    const containerRect = sidecarContainer.getBoundingClientRect();
    const relativeX = dropX - containerRect.left;
    const relativeY = dropY - containerRect.top;
    
    // Determine drop zone based on position
    const centerX = containerRect.width / 2;
    const centerY = containerRect.height / 2;
    const threshold = 100; // pixels from edge to trigger zone

    let dropZone: DropZone | null = null;

    if (relativeX < threshold) {
      // Left zone - columns layout
      dropZone = {
        type: 'left',
        bounds: { x: 0, y: 0, width: centerX, height: containerRect.height },
        layoutMode: 'columns',
        position: 0
      };
    } else if (relativeX > containerRect.width - threshold) {
      // Right zone - columns layout
      dropZone = {
        type: 'right',
        bounds: { x: centerX, y: 0, width: centerX, height: containerRect.height },
        layoutMode: 'columns',
        position: 1
      };
    } else if (relativeY < threshold) {
      // Top zone - rows layout
      dropZone = {
        type: 'top',
        bounds: { x: 0, y: 0, width: containerRect.width, height: centerY },
        layoutMode: 'rows',
        position: 0
      };
    } else if (relativeY > containerRect.height - threshold) {
      // Bottom zone - rows layout
      dropZone = {
        type: 'bottom',
        bounds: { x: 0, y: centerY, width: containerRect.width, height: centerY },
        layoutMode: 'rows',
        position: 1
      };
    } else {
      // Center zone - determine grid position
      const gridX = relativeX < centerX ? 0 : 1;
      const gridY = relativeY < centerY ? 0 : 1;
      const gridPosition = gridY * 2 + gridX;
      
      dropZone = {
        type: `grid-${gridPosition + 1}` as DropZoneType,
        bounds: { 
          x: gridX * centerX, 
          y: gridY * centerY, 
          width: centerX, 
          height: centerY 
        },
        layoutMode: 'grid',
        position: gridPosition
      };
    }

    if (dropZone) {
      onPanelDrop(view.id, dropZone);
    }
  };

  // Render as a sidecar component (matching TabSidecar structure)
  return (
    <motion.div
      ref={panelRef}
      layout
      drag
      dragControls={dragControls}
      dragMomentum={false}
      dragElastic={0.1}
      onDragStart={handleDragStart}
      onDragEnd={handleDragEnd}
      whileDrag={{ 
        scale: 1.05, 
        zIndex: 1000,
        boxShadow: "0 10px 30px rgba(0,0,0,0.3)",
        rotate: 2
      }}
      initial={{ opacity: 0, scale: 0.95 }}
      animate={{ 
        opacity: isLocalDragging ? 0.8 : 1, 
        scale: isLocalDragging ? 1.05 : 1 
      }}
      exit={{ opacity: 0, scale: 0.95 }}
      transition={{ duration: 0.2 }}
      className={`h-full w-full shadow-2xl drop-shadow-2xl cursor-grab active:cursor-grabbing ${className || ''} ${isLocalDragging ? 'shadow-2xl' : ''}`}
      style={style}
    >
      <div className="bg-background-default overflow-hidden flex flex-col h-full">
        {/* Sidecar Header */}
        <div className="flex items-center justify-between px-4 py-2 border-b border-borderSubtle flex-shrink-0 flex-grow-0">
          <div className="flex items-center space-x-2">
            {/* Drag Handle */}
            <motion.div
              className="cursor-grab active:cursor-grabbing p-1 hover:bg-background-muted rounded mr-1"
              onPointerDown={(e) => dragControls.start(e)}
              whileHover={{ scale: 1.1 }}
              whileTap={{ scale: 0.9 }}
            >
              <GripVertical size={14} className="text-text-subtle" />
            </motion.div>
            
            {renderIcon(view.iconType)}
            <div className="flex flex-col">
              <span className="text-textStandard font-medium">{view.title}</span>
              {view.fileName && (
                <span className="text-xs font-mono text-text-muted">{view.fileName}</span>
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
                  onClick={() => onHideView(view.id)}
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
          {renderContent(view.contentType, view.contentProps, tabId)}
        </div>
      </div>
    </motion.div>
  );
};

export const MultiPanelTabSidecar: React.FC<MultiPanelTabSidecarProps> = ({
  sidecarState,
  onHideView,
  onShowView,
  tabId,
  className = ''
}) => {
  const [layoutMode, setLayoutMode] = useState<SidecarLayoutMode>('single');
  const [panelOrder, setPanelOrder] = useState<string[]>([]);
  const [isDragActive, setIsDragActive] = useState(false);
  const [dropZones, setDropZones] = useState<DropZone[]>([]);
  const containerRef = useRef<HTMLDivElement>(null);
  const unifiedSidecarContext = useUnifiedSidecarContextOptional();

  // Get active views - memoize to prevent recreation on every render
  const activeViews = React.useMemo(() => 
    sidecarState.views.filter(view => 
      sidecarState.activeViews.includes(view.id)
    ), 
    [sidecarState.views, sidecarState.activeViews]
  );

  // Register all active views with UnifiedSidecarContext for AI awareness
  useEffect(() => {
    if (!unifiedSidecarContext || activeViews.length === 0) {
      return;
    }

    console.log('🔧 MultiPanelTabSidecar: Registering', activeViews.length, 'views with unified context');

    // Register each active view
    activeViews.forEach(view => {
      let sidecarInfo;
      const sidecarId = `tab-${tabId}-${view.id}`;

      switch (view.contentType) {
        case 'diff':
          const diffLines = (view.contentProps.diffContent || '').split('\n');
          const addedLines = diffLines.filter((line: string) => line.startsWith('+')).length;
          const removedLines = diffLines.filter((line: string) => line.startsWith('-')).length;
          
          sidecarInfo = {
            id: sidecarId,
            type: 'diff-viewer' as const,
            title: view.title || 'Diff Viewer',
            fileName: view.fileName || 'File',
            filePath: undefined,
            addedLines,
            removedLines,
            totalChanges: addedLines + removedLines,
            viewMode: 'unified' as const,
            timestamp: Date.now(),
          };
          break;

        case 'localhost':
          const localhostUrl = view.contentProps.url || 'http://localhost:3000';
          const port = parseInt(new URL(localhostUrl).port) || 3000;
          
          sidecarInfo = {
            id: sidecarId,
            type: 'localhost-viewer' as const,
            title: view.title || 'Localhost Viewer',
            url: localhostUrl,
            port,
            protocol: new URL(localhostUrl).protocol.replace(':', '') as 'http' | 'https',
            isLocal: true,
            serviceType: 'development',
            timestamp: Date.now(),
          };
          break;

        case 'web':
          // Skip - WebBrowser component handles its own registration
          console.log('🔧 MultiPanelTabSidecar: Skipping web viewer registration (handled by WebBrowser component)');
          break;

        case 'file':
          const filePath = view.contentProps.path || '';
          const fileName = filePath.split('/').pop() || filePath;
          const fileExtension = fileName.split('.').pop() || '';
          
          sidecarInfo = {
            id: sidecarId,
            type: 'file-viewer' as const,
            title: view.title || 'File Viewer',
            filePath,
            fileName,
            fileSize: 0,
            fileType: fileExtension,
            isReadable: true,
            lastModified: Date.now(),
            timestamp: Date.now(),
          };
          break;

        case 'editor':
          const editorPath = view.contentProps.path;
          const editorFileName = editorPath 
            ? editorPath.split('/').pop() || editorPath 
            : view.fileName || 'Untitled Document';
          
          sidecarInfo = {
            id: sidecarId,
            type: 'document-editor' as const,
            title: view.title || 'Document Editor',
            filePath: editorPath,
            fileName: editorFileName,
            contentLength: (view.contentProps.content || '').length,
            hasUnsavedChanges: false,
            isNewDocument: !editorPath,
            language: editorPath ? editorPath.split('.').pop() : undefined,
            timestamp: Date.now(),
          };
          break;
      }

      if (sidecarInfo) {
        unifiedSidecarContext.registerSidecar(sidecarInfo);
        console.log('🔧 MultiPanelTabSidecar: Registered sidecar:', sidecarInfo.id, sidecarInfo.type);
      }
    });

    // Cleanup: unregister all views when they change or component unmounts
    return () => {
      activeViews.forEach(view => {
        // Skip web viewer cleanup since it handles its own
        if (view.contentType !== 'web') {
          const sidecarId = `tab-${tabId}-${view.id}`;
          unifiedSidecarContext.unregisterSidecar(sidecarId);
          console.log('🔧 MultiPanelTabSidecar: Unregistered sidecar:', sidecarId);
        }
      });
    };
  }, [unifiedSidecarContext, activeViews, tabId]);

  // Initialize panel order when views change
  React.useEffect(() => {
    const currentViewIds = activeViews.map(v => v.id);
    setPanelOrder(prev => {
      // Keep existing order for panels that still exist, add new ones at the end
      const existingOrder = prev.filter(id => currentViewIds.includes(id));
      const newPanels = currentViewIds.filter(id => !prev.includes(id));
      return [...existingOrder, ...newPanels];
    });
  }, [activeViews]);

  // Auto-adjust layout mode based on number of active views
  React.useEffect(() => {
    if (activeViews.length <= 1 && layoutMode !== 'single') {
      setLayoutMode('single');
    } else if (activeViews.length === 2 && layoutMode === 'single') {
      setLayoutMode('columns');
    } else if (activeViews.length > 2 && (layoutMode === 'single' || layoutMode === 'columns')) {
      setLayoutMode('grid');
    }
  }, [activeViews.length]); // Remove layoutMode from dependencies to prevent infinite loop

  const handlePanelDrop = useCallback((draggedViewId: string, dropZone: DropZone) => {
    console.log('Panel dropped:', draggedViewId, 'in zone:', dropZone);
    
    // Update layout mode based on drop zone
    setLayoutMode(prev => {
      if (dropZone.layoutMode !== prev) {
        return dropZone.layoutMode;
      }
      return prev;
    });

    // Reorder panels based on drop position
    if (dropZone.position !== undefined) {
      setPanelOrder(prev => {
        const newOrder = prev.filter(id => id !== draggedViewId);
        newOrder.splice(dropZone.position!, 0, draggedViewId);
        return newOrder;
      });
    }

    setIsDragActive(false);
    setDropZones([]);
  }, []); // Remove layoutMode dependency

  // Calculate drop zones when dragging starts
  const calculateDropZones = useCallback(() => {
    if (!containerRef.current) return [];

    const container = containerRef.current;
    const rect = container.getBoundingClientRect();
    const zones: DropZone[] = [];

    const threshold = 80;
    const centerX = rect.width / 2;
    const centerY = rect.height / 2;

    // Left zone (columns)
    zones.push({
      type: 'left',
      bounds: { x: 0, y: 0, width: threshold, height: rect.height },
      layoutMode: 'columns',
      position: 0
    });

    // Right zone (columns)
    zones.push({
      type: 'right',
      bounds: { x: rect.width - threshold, y: 0, width: threshold, height: rect.height },
      layoutMode: 'columns',
      position: 1
    });

    // Top zone (rows)
    zones.push({
      type: 'top',
      bounds: { x: 0, y: 0, width: rect.width, height: threshold },
      layoutMode: 'rows',
      position: 0
    });

    // Bottom zone (rows)
    zones.push({
      type: 'bottom',
      bounds: { x: 0, y: rect.height - threshold, width: rect.width, height: threshold },
      layoutMode: 'rows',
      position: 1
    });

    // Grid zones (center area)
    if (activeViews.length > 2) {
      for (let i = 0; i < 4; i++) {
        const gridX = (i % 2) * centerX;
        const gridY = Math.floor(i / 2) * centerY;
        
        zones.push({
          type: `grid-${i + 1}` as DropZoneType,
          bounds: { 
            x: gridX + threshold/2, 
            y: gridY + threshold/2, 
            width: centerX - threshold, 
            height: centerY - threshold 
          },
          layoutMode: 'grid',
          position: i
        });
      }
    }

    return zones;
  }, [activeViews.length]);

  // Handle drag start globally
  React.useEffect(() => {
    const handleDragStart = () => {
      setIsDragActive(true);
      setDropZones(calculateDropZones());
    };

    const handleDragEnd = () => {
      setIsDragActive(false);
      setDropZones([]);
    };

    // Listen for drag events on panels
    document.addEventListener('dragstart', handleDragStart);
    document.addEventListener('dragend', handleDragEnd);

    return () => {
      document.removeEventListener('dragstart', handleDragStart);
      document.removeEventListener('dragend', handleDragEnd);
    };
  }, []); // Empty dependency array to prevent infinite loop

  if (activeViews.length === 0) {
    return null;
  }

  const renderDropZones = () => {
    if (!isDragActive || dropZones.length === 0) return null;

    return (
      <div className="absolute inset-0 pointer-events-none z-50">
        {dropZones.map((zone, index) => (
          <motion.div
            key={`${zone.type}-${index}`}
            initial={{ opacity: 0 }}
            animate={{ opacity: 0.3 }}
            exit={{ opacity: 0 }}
            className={`absolute border-2 border-dashed rounded-lg ${
              zone.layoutMode === 'columns' ? 'border-blue-400 bg-blue-400/10' :
              zone.layoutMode === 'rows' ? 'border-green-400 bg-green-400/10' :
              zone.layoutMode === 'grid' ? 'border-purple-400 bg-purple-400/10' :
              'border-gray-400 bg-gray-400/10'
            }`}
            style={{
              left: zone.bounds.x,
              top: zone.bounds.y,
              width: zone.bounds.width,
              height: zone.bounds.height
            }}
          >
            <div className="absolute inset-0 flex items-center justify-center">
              <div className={`px-2 py-1 rounded text-xs font-medium ${
                zone.layoutMode === 'columns' ? 'bg-blue-400 text-white' :
                zone.layoutMode === 'rows' ? 'bg-green-400 text-white' :
                zone.layoutMode === 'grid' ? 'bg-purple-400 text-white' :
                'bg-gray-400 text-white'
              }`}>
                {zone.type === 'left' ? 'Left Column' :
                 zone.type === 'right' ? 'Right Column' :
                 zone.type === 'top' ? 'Top Row' :
                 zone.type === 'bottom' ? 'Bottom Row' :
                 zone.type.startsWith('grid-') ? `Grid ${zone.type.split('-')[1]}` :
                 'Drop Here'}
              </div>
            </div>
          </motion.div>
        ))}
      </div>
    );
  };

  // Render individual sidecar panels directly without wrapping container
  const renderPanels = () => {
    if (activeViews.length === 0) return null;

    if (activeViews.length === 1 || layoutMode === 'single') {
      return (
        <Panel
          view={activeViews[0]}
          tabId={tabId}
          onHideView={onHideView}
          onPanelDrop={handlePanelDrop}
          layoutMode={layoutMode}
          className="h-full"
        />
      );
    }

    switch (layoutMode) {
      case 'columns':
        return (
          <div className="h-full flex gap-2">
            <AnimatePresence mode="popLayout">
              {activeViews.slice(0, 2).map((view) => (
                <Panel
                  key={view.id}
                  view={view}
                  tabId={tabId}
                  onHideView={onHideView}
                  onPanelDrop={handlePanelDrop}
                  layoutMode={layoutMode}
                  style={{ flex: '1 1 0%', minWidth: '200px' }}
                />
              ))}
            </AnimatePresence>
          </div>
        );

      case 'rows':
        return (
          <div className="h-full flex flex-col gap-2">
            <AnimatePresence mode="popLayout">
              {activeViews.slice(0, 2).map((view) => (
                <Panel
                  key={view.id}
                  view={view}
                  tabId={tabId}
                  onHideView={onHideView}
                  onPanelDrop={handlePanelDrop}
                  layoutMode={layoutMode}
                  style={{ flex: '1 1 0%', minHeight: '150px' }}
                />
              ))}
            </AnimatePresence>
          </div>
        );

      case 'grid':
        return (
          <div className="h-full grid grid-cols-2 gap-2" style={{ gridTemplateRows: 'repeat(2, 1fr)' }}>
            <AnimatePresence mode="popLayout">
              {activeViews.slice(0, 4).map((view) => (
                <Panel
                  key={view.id}
                  view={view}
                  tabId={tabId}
                  onHideView={onHideView}
                  onPanelDrop={handlePanelDrop}
                  layoutMode={layoutMode}
                  style={{ minHeight: '150px', minWidth: '200px' }}
                />
              ))}
            </AnimatePresence>
          </div>
        );

      case 'custom':
        // For custom layout, we could implement drag-and-drop positioning
        return (
          <div className="h-full relative">
            <AnimatePresence mode="popLayout">
              {activeViews.map((view, index) => (
                <Panel
                  key={view.id}
                  view={view}
                  tabId={tabId}
                  onHideView={onHideView}
                  onPanelDrop={handlePanelDrop}
                  layoutMode={layoutMode}
                  style={{
                    position: 'absolute',
                    left: `${(index % 2) * 50}%`,
                    top: `${Math.floor(index / 2) * 50}%`,
                    width: '48%',
                    height: '48%',
                    minWidth: '200px',
                    minHeight: '150px'
                  }}
                />
              ))}
            </AnimatePresence>
          </div>
        );

      default:
        return (
          <Panel
            view={activeViews[0]}
            tabId={tabId}
            onHideView={onHideView}
            onPanelDrop={handlePanelDrop}
            layoutMode={layoutMode}
            className="h-full"
          />
        );
    }
  };

  // Return the panels directly with drop zones overlay, no additional container
  return (
    <div 
      ref={containerRef}
      className={`h-full w-full relative ${className}`}
    >
      {renderPanels()}
      {renderDropZones()}
    </div>
  );
};

export default MultiPanelTabSidecar;
