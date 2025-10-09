import React, { useState, useCallback, useRef, useEffect } from 'react';
import { useSidecar, Sidecar } from '../SidecarLayout';
import { Plus, X } from 'lucide-react';
import { Button } from '../ui/button';

interface Container {
  id: string;
  position: 'right-top' | 'right-main' | 'right-bottom';
  content: React.ReactNode;
}

interface ContainerContentProps {
  container: Container;
  onRemove: (id: string) => void;
  onSetContent: (id: string, content: React.ReactNode) => void;
}

function IndividualContainer({ container, onRemove, onSetContent }: ContainerContentProps) {
  const [selectedContent, setSelectedContent] = useState<'localhost' | 'sidecar' | null>(null);

  const handleContentSelect = (contentType: 'localhost' | 'sidecar') => {
    setSelectedContent(contentType);
    
    if (contentType === 'localhost') {
      onSetContent(container.id, (
        <div className="h-full bg-background-default rounded-lg border border-border-subtle overflow-hidden">
          <iframe 
            src="http://localhost:3000" 
            className="w-full h-full border-0"
            title="Localhost Viewer"
          />
        </div>
      ));
    } else if (contentType === 'sidecar') {
      onSetContent(container.id, <Sidecar className="h-full" />);
    }
  };

  return (
    <div className="h-full bg-background-muted rounded-lg border border-border-subtle relative group">
      <div className="absolute top-2 right-2 z-10">
        <Button
          onClick={() => onRemove(container.id)}
          variant="ghost"
          size="sm"
          className="w-6 h-6 rounded-full bg-background-default/80 hover:bg-background-default opacity-0 group-hover:opacity-100 transition-opacity"
        >
          <X className="w-3 h-3" />
        </Button>
      </div>

      {container.content || (
        <div className="h-full flex flex-col items-center justify-center p-4 space-y-3">
          <p className="text-text-muted text-sm text-center">Select content for this container:</p>
          <div className="flex gap-2">
            <Button
              onClick={() => handleContentSelect('localhost')}
              variant="outline"
              size="sm"
            >
              Localhost Viewer
            </Button>
            <Button
              onClick={() => handleContentSelect('sidecar')}
              variant="outline"
              size="sm"
            >
              Sidecar
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}

export const MainPanelLayout: React.FC<{
  children: React.ReactNode;
  removeTopPadding?: boolean;
  backgroundColor?: string;
}> = ({ children, removeTopPadding = false, backgroundColor = 'bg-background-default' }) => {
  const sidecar = useSidecar();
  const [containers, setContainers] = useState<Container[]>([]);
  const [hoveredEdge, setHoveredEdge] = useState<string | null>(null);
  const [horizontalSplitRatio, setHorizontalSplitRatio] = useState(0.7); // 70% left, 30% right
  const [rightColumnHeights, setRightColumnHeights] = useState({
    rightTopHeight: '33.33%',
    rightMainHeight: '33.33%', 
    rightBottomHeight: '33.33%'
  });
  
  const [isHorizontalResizing, setIsHorizontalResizing] = useState(false);
  const [isVerticalResizing, setIsVerticalResizing] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  // Get containers by position
  const rightTopContainer = containers.find(c => c.position === 'right-top');
  const rightMainContainer = containers.find(c => c.position === 'right-main');
  const rightBottomContainer = containers.find(c => c.position === 'right-bottom');

  // Check if main sidecar is visible
  const mainSidecarVisible = sidecar?.activeView && sidecar?.views.find(v => v.id === sidecar.activeView);

  const addContainer = useCallback((position: 'right-top' | 'right-main' | 'right-bottom') => {
    const newContainer: Container = {
      id: `container-${Date.now()}`,
      position,
      content: null
    };
    setContainers(prev => [...prev, newContainer]);
  }, []);

  const removeContainer = useCallback((id: string) => {
    setContainers(prev => prev.filter(c => c.id !== id));
  }, []);

  const setContainerContent = useCallback((id: string, content: React.ReactNode) => {
    setContainers(prev => prev.map(c => 
      c.id === id ? { ...c, content } : c
    ));
  }, []);

  // Horizontal resize handlers
  const handleHorizontalMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsHorizontalResizing(true);

    const handleMouseMove = (e: MouseEvent) => {
      if (!containerRef.current) return;
      
      const rect = containerRef.current.getBoundingClientRect();
      const newRatio = (e.clientX - rect.left) / rect.width;
      const clampedRatio = Math.max(0.2, Math.min(0.8, newRatio));
      setHorizontalSplitRatio(clampedRatio);
    };

    const handleMouseUp = () => {
      setIsHorizontalResizing(false);
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  }, []);

  // Vertical resize handlers
  const handleVerticalMouseDown = useCallback((e: React.MouseEvent, resizeType: 'right-top-main' | 'right-main-bottom') => {
    e.preventDefault();
    setIsVerticalResizing(true);

    const handleMouseMove = (e: MouseEvent) => {
      if (!containerRef.current) return;
      
      const rect = containerRef.current.getBoundingClientRect();
      const relativeY = (e.clientY - rect.top) / rect.height;
      
      if (resizeType === 'right-top-main') {
        const newTopHeight = Math.max(0.1, Math.min(0.8, relativeY));
        const remainingHeight = 1 - newTopHeight;
        
        setRightColumnHeights(prev => ({
          ...prev,
          rightTopHeight: `${newTopHeight * 100}%`,
          rightMainHeight: rightBottomContainer 
            ? `${remainingHeight * 0.5 * 100}%`
            : `${remainingHeight * 100}%`,
          rightBottomHeight: rightBottomContainer 
            ? `${remainingHeight * 0.5 * 100}%`
            : prev.rightBottomHeight
        }));
      } else if (resizeType === 'right-main-bottom') {
        const topHeight = parseFloat(rightColumnHeights.rightTopHeight) / 100;
        const availableHeight = 1 - topHeight;
        const mainRelativeHeight = (relativeY - topHeight) / availableHeight;
        const clampedMainHeight = Math.max(0.1, Math.min(0.9, mainRelativeHeight));
        
        setRightColumnHeights(prev => ({
          ...prev,
          rightMainHeight: `${clampedMainHeight * availableHeight * 100}%`,
          rightBottomHeight: `${(1 - clampedMainHeight) * availableHeight * 100}%`
        }));
      }
    };

    const handleMouseUp = () => {
      setIsVerticalResizing(false);
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  }, [rightColumnHeights, rightBottomContainer]);

  // Auto-adjust right column when containers are added/removed
  useEffect(() => {
    const rightContainers = containers.filter(c => c.position.startsWith('right-'));
    const count = rightContainers.length;
    
    if (count === 0) return;
    
    const equalHeight = `${100 / count}%`;
    setRightColumnHeights({
      rightTopHeight: rightTopContainer ? equalHeight : '0%',
      rightMainHeight: rightMainContainer ? equalHeight : '0%', 
      rightBottomHeight: rightBottomContainer ? equalHeight : '0%'
    });
  }, [containers.length, rightTopContainer, rightMainContainer, rightBottomContainer]);

  const hasRightContainers = containers.some(c => c.position.startsWith('right-'));

  return (
    <div className={`h-dvh`} ref={containerRef}>
      <div
        className={`flex ${backgroundColor} flex-1 min-w-0 h-full min-h-0 ${removeTopPadding ? '' : 'pt-[32px]'}`}
      >
        {/* Left Panel (Main Content) */}
        <div 
          className="flex flex-col flex-1 min-w-0"
          style={{ 
            width: hasRightContainers || mainSidecarVisible ? `${horizontalSplitRatio * 100}%` : '100%'
          }}
        >
          {children}
        </div>

        {/* Horizontal Resize Handle */}
        {(hasRightContainers || mainSidecarVisible) && (
          <div 
            className={`w-1 cursor-col-resize hover:bg-borderSubtle transition-colors group ${
              isHorizontalResizing ? 'bg-borderProminent' : ''
            }`}
            onMouseDown={handleHorizontalMouseDown}
          >
            <div 
              className={`h-8 w-0.5 bg-border-subtle group-hover:bg-border-strong rounded-full transition-colors my-auto ml-0.5 ${
                isHorizontalResizing ? 'bg-border-strong' : ''
              }`} 
            />
          </div>
        )}

        {/* Right Panel */}
        {(hasRightContainers || mainSidecarVisible) && (
          <div 
            className="flex flex-col relative"
            style={{ 
              width: `${(1 - horizontalSplitRatio) * 100}%`,
              minWidth: '200px'
            }}
          >
            {/* Main Sidecar (takes precedence when visible) */}
            {mainSidecarVisible && !hasRightContainers && (
              <div className="h-full">
                <Sidecar />
              </div>
            )}

            {/* Multi-container layout */}
            {hasRightContainers && (
              <div className="h-full flex flex-col relative">
                {/* Right Top Container */}
                {rightTopContainer && (
                  <div 
                    className="relative"
                    style={{ height: rightColumnHeights.rightTopHeight }}
                  >
                    <IndividualContainer
                      container={rightTopContainer}
                      onRemove={removeContainer}
                      onSetContent={setContainerContent}
                    />
                  </div>
                )}

                {/* Right Top-Main Resize Handle */}
                {rightTopContainer && rightMainContainer && (
                  <div 
                    className={`h-1 cursor-row-resize hover:bg-borderSubtle transition-colors group ${
                      isVerticalResizing ? 'bg-borderProminent' : ''
                    }`}
                    onMouseDown={(e) => handleVerticalMouseDown(e, 'right-top-main')}
                  >
                    <div 
                      className={`w-8 h-0.5 bg-border-subtle group-hover:bg-border-strong rounded-full transition-colors mx-auto mt-0.5 ${
                        isVerticalResizing ? 'bg-border-strong' : ''
                      }`} 
                    />
                  </div>
                )}

                {/* Right Main Container */}
                {rightMainContainer && (
                  <div 
                    className="relative"
                    style={{ height: rightColumnHeights.rightMainHeight }}
                  >
                    <IndividualContainer
                      container={rightMainContainer}
                      onRemove={removeContainer}
                      onSetContent={setContainerContent}
                    />

                    {/* Container addition zones for right main */}
                    {!rightTopContainer && (
                      <div
                        className="absolute top-0 left-0 right-0 h-4 z-20 pointer-events-auto"
                        onMouseEnter={() => setHoveredEdge('right-top')}
                        onMouseLeave={() => setHoveredEdge(null)}
                      >
                        {hoveredEdge === 'right-top' && (
                          <div className="absolute left-1/2 top-1 transform -translate-x-1/2">
                            <Button
                              onClick={() => addContainer('right-top')}
                              className="w-6 h-6 rounded-full bg-background-default border border-border-subtle shadow-lg hover:shadow-xl hover:scale-105 transition-all duration-200"
                              variant="ghost"
                              size="sm"
                            >
                              <Plus className="w-3 h-3" />
                            </Button>
                          </div>
                        )}
                      </div>
                    )}

                    {!rightBottomContainer && (
                      <div
                        className="absolute bottom-0 left-0 right-0 h-4 z-20 pointer-events-auto"
                        onMouseEnter={() => setHoveredEdge('right-bottom')}
                        onMouseLeave={() => setHoveredEdge(null)}
                      >
                        {hoveredEdge === 'right-bottom' && (
                          <div className="absolute left-1/2 bottom-1 transform -translate-x-1/2">
                            <Button
                              onClick={() => addContainer('right-bottom')}
                              className="w-6 h-6 rounded-full bg-background-default border border-border-subtle shadow-lg hover:shadow-xl hover:scale-105 transition-all duration-200"
                              variant="ghost"
                              size="sm"
                            >
                              <Plus className="w-3 h-3" />
                            </Button>
                          </div>
                        )}
                      </div>
                    )}

                    {/* Right-only container hover zones when no main sidecar */}
                    {!rightTopContainer && !mainSidecarVisible && (
                      <div
                        className="absolute top-0 left-0 right-0 h-4 z-20 pointer-events-auto"
                        onMouseEnter={() => setHoveredEdge('right-top')}
                        onMouseLeave={() => setHoveredEdge(null)}
                      >
                        {hoveredEdge === 'right-top' && (
                          <div className="absolute left-1/2 top-1 transform -translate-x-1/2">
                            <Button
                              onClick={() => addContainer('right-top')}
                              className="w-6 h-6 rounded-full bg-background-default border border-border-subtle shadow-lg hover:shadow-xl hover:scale-105 transition-all duration-200"
                              variant="ghost"
                              size="sm"
                            >
                              <Plus className="w-3 h-3" />
                            </Button>
                          </div>
                        )}
                      </div>
                    )}

                    {!rightBottomContainer && !mainSidecarVisible && (
                      <div
                        className="absolute bottom-0 left-0 right-0 h-4 z-20 pointer-events-auto"
                        onMouseEnter={() => setHoveredEdge('right-bottom')}
                        onMouseLeave={() => setHoveredEdge(null)}
                      >
                        {hoveredEdge === 'right-bottom' && (
                          <div className="absolute left-1/2 bottom-1 transform -translate-x-1/2">
                            <Button
                              onClick={() => addContainer('right-bottom')}
                              className="w-6 h-6 rounded-full bg-background-default border border-border-subtle shadow-lg hover:shadow-xl hover:scale-105 transition-all duration-200"
                              variant="ghost"
                              size="sm"
                            >
                              <Plus className="w-3 h-3" />
                            </Button>
                          </div>
                        )}
                      </div>
                    )}
                  </div>
                )}

                {/* Right Main-Bottom Resize Handle */}
                {rightMainContainer && rightBottomContainer && (
                  <div 
                    className={`h-1 cursor-row-resize hover:bg-borderSubtle transition-colors group ${
                      isVerticalResizing ? 'bg-borderProminent' : ''
                    }`}
                    onMouseDown={(e) => handleVerticalMouseDown(e, 'right-main-bottom')}
                  >
                    <div 
                      className={`w-8 h-0.5 bg-border-subtle group-hover:bg-border-strong rounded-full transition-colors mx-auto mt-0.5 ${
                        isVerticalResizing ? 'bg-border-strong' : ''
                      }`} 
                    />
                  </div>
                )}

                {/* Right Bottom Container */}
                {rightBottomContainer && (
                  <div 
                    className="relative"
                    style={{ height: rightColumnHeights.rightBottomHeight }}
                  >
                    <IndividualContainer
                      container={rightBottomContainer}
                      onRemove={removeContainer}
                      onSetContent={setContainerContent}
                    />
                  </div>
                )}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
};
