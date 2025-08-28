import React, { useState, useRef, useCallback } from 'react';
import { useLocation } from 'react-router-dom';
import { Plus, Globe, FileText, X } from 'lucide-react';
import { Button } from '../ui/button';
import { Tooltip, TooltipTrigger, TooltipContent } from '../ui/Tooltip';
import { Sidecar, useSidecar } from '../SidecarLayout';
import SidecarTabs from '../SidecarTabs';
import { FileViewer } from '../FileViewer';

interface ContainerContent {
  type: 'localhost' | 'file';
  title: string;
  icon: React.ReactNode;
  content: React.ReactNode;
  fileName?: string;
}

interface SidecarContainer {
  id: string;
  position: 'main' | 'top' | 'bottom' | 'right-main' | 'right-top' | 'right-bottom';
  content: ContainerContent | null;
  size: number; // Height percentage for vertical containers, width percentage for horizontal
}

interface IndividualContainerProps {
  container: SidecarContainer;
  onRemove: (containerId: string) => void;
  onSetContent: (containerId: string, content: ContainerContent) => void;
}

const IndividualContainer: React.FC<IndividualContainerProps> = ({
  container,
  onRemove,
  onSetContent,
}) => {
  const handleLocalhostClick = () => {
    const content: ContainerContent = {
      type: 'localhost',
      title: 'Localhost Viewer',
      icon: <Globe size={16} />,
      content: <SidecarTabs initialUrl="http://localhost:3000" />,
      fileName: 'http://localhost:3000',
    };
    onSetContent(container.id, content);
  };

  const handleFileViewerClick = async () => {
    try {
      const filePath = await window.electron.selectFileOrDirectory();
      if (filePath) {
        const fileName = filePath.split('/').pop() || filePath;
        const content: ContainerContent = {
          type: 'file',
          title: 'File Viewer',
          icon: <FileText size={16} />,
          content: <FileViewer filePath={filePath} />,
          fileName,
        };
        onSetContent(container.id, content);
      }
    } catch (error) {
      console.error('Error opening file dialog:', error);
    }
  };

  // If container has content, show it
  if (container.content) {
    return (
      <div className="bg-background-default overflow-hidden rounded-xl flex flex-col h-full">
        {/* Container Header */}
        <div className="flex items-center justify-between p-4 border-b border-borderSubtle flex-shrink-0">
          <div className="flex items-center space-x-2">
            {container.content.icon}
            <div className="flex flex-col">
              <span className="text-textStandard font-medium">{container.content.title}</span>
              {container.content.fileName && (
                <span className="text-xs font-mono text-text-muted">{container.content.fileName}</span>
              )}
            </div>
          </div>

          <div className="flex items-center space-x-2">
            {/* Close Button */}
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => onRemove(container.id)}
                  className="text-textSubtle hover:text-textStandard cursor-pointer focus:outline-none focus:ring-2 focus:ring-borderProminent focus:ring-offset-1"
                >
                  <X size={16} />
                </Button>
              </TooltipTrigger>
              <TooltipContent side="bottom">Close Container</TooltipContent>
            </Tooltip>
          </div>
        </div>

        {/* Container Content */}
        <div className="flex-1 overflow-hidden rounded-xl">
          {container.content.content}
        </div>
      </div>
    );
  }

  // Show content selection menu for empty containers
  return (
    <div className="relative h-full bg-background-default border border-border-subtle rounded-xl flex flex-col items-center justify-center">
      {/* Remove container button */}
      <Button
        onClick={() => onRemove(container.id)}
        className="absolute top-2 right-2 w-6 h-6 rounded-full bg-background-default border border-border-subtle shadow-sm hover:shadow-md hover:scale-105 transition-all duration-200 z-10"
        variant="ghost"
        size="sm"
      >
        <X className="w-3 h-3" />
      </Button>

      {/* Content selection menu */}
      <div className="flex flex-col items-center space-y-4 p-6">
        <div className="w-12 h-12 rounded-full bg-background-muted border border-border-subtle flex items-center justify-center">
          <Plus className="w-6 h-6 text-text-muted" />
        </div>
        
        <div className="text-center">
          <h3 className="text-sm font-medium text-text-standard mb-1">Add Content</h3>
          <p className="text-xs text-text-muted">Choose what to display in this container</p>
        </div>

        <div className="flex flex-col space-y-2 w-full max-w-[160px]">
          <Button
            onClick={handleLocalhostClick}
            className="w-full justify-start text-left hover:bg-background-medium transition-colors duration-150"
            variant="ghost"
            size="sm"
          >
            <Globe className="w-4 h-4 mr-2" />
            Localhost Viewer
          </Button>
          
          <Button
            onClick={handleFileViewerClick}
            className="w-full justify-start text-left hover:bg-background-medium transition-colors duration-150"
            variant="ghost"
            size="sm"
          >
            <FileText className="w-4 h-4 mr-2" />
            Open File
          </Button>
        </div>
      </div>
    </div>
  );
};

export const MainPanelLayout: React.FC<{
  children: React.ReactNode;
  removeTopPadding?: boolean;
  backgroundColor?: string;
}> = ({ children, removeTopPadding = false, backgroundColor = 'bg-background-default' }) => {
  const location = useLocation();
  const sidecar = useSidecar();
  
  // Multi-container state
  const [containers, setContainers] = useState<SidecarContainer[]>([]);
  const [hoveredEdge, setHoveredEdge] = useState<'top' | 'right' | 'bottom' | null>(null);
  
  // Only show sidecar on chat-related pages
  const shouldShowSidecar = location.pathname === '/' || location.pathname === '/chat' || location.pathname === '/pair';
  const mainSidecarVisible = shouldShowSidecar && sidecar?.activeView && sidecar?.views.find((v) => v.id === sidecar.activeView);
  const hasAnyContainers = containers.length > 0;
  const isVisible = mainSidecarVisible || hasAnyContainers;

  // State for resizing
  const [sidecarWidth, setSidecarWidth] = useState(50); // Percentage
  const [isResizing, setIsResizing] = useState(false);
  const [isVerticalResizing, setIsVerticalResizing] = useState(false);
  const [isHorizontalResizing, setIsHorizontalResizing] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  // Horizontal resize handler (main content vs sidecar)
  const handleMainHorizontalMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);

    const handleMouseMove = (e: MouseEvent) => {
      if (!containerRef.current) return;

      const containerRect = containerRef.current.getBoundingClientRect();
      const containerWidth = containerRect.width;
      const mouseX = e.clientX - containerRect.left;

      // Calculate percentage, with constraints
      const newPercentage = Math.max(20, Math.min(80, (mouseX / containerWidth) * 100));
      setSidecarWidth(100 - newPercentage); // Invert because we want sidecar width
    };

    const handleMouseUp = () => {
      setIsResizing(false);
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  }, []);

  // Vertical resize handler (between containers)
  const handleVerticalMouseDown = useCallback((e: React.MouseEvent, resizeType: 'top-main' | 'main-bottom') => {
    e.preventDefault();
    setIsVerticalResizing(true);

    const handleMouseMove = (e: MouseEvent) => {
      if (!containerRef.current) return;

      const sidecarRect = containerRef.current.querySelector('[data-sidecar-area]')?.getBoundingClientRect();
      if (!sidecarRect) return;

      const mouseY = e.clientY - sidecarRect.top;
      const sidecarHeight = sidecarRect.height;
      const mousePercentage = (mouseY / sidecarHeight) * 100;

      // Update container sizes based on resize type
      setContainers(prev => {
        return prev.map(container => {
          if (resizeType === 'top-main') {
            if (container.position === 'top') {
              return { ...container, size: Math.max(15, Math.min(70, mousePercentage)) };
            }
          } else if (resizeType === 'main-bottom') {
            if (container.position === 'bottom') {
              return { ...container, size: Math.max(15, Math.min(70, 100 - mousePercentage)) };
            }
          }
          return container;
        });
      });
    };

    const handleMouseUp = () => {
      setIsVerticalResizing(false);
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  }, []);

  // Horizontal resize handler (between left and right columns)
  const handleColumnHorizontalMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    setIsHorizontalResizing(true);

    const handleMouseMove = (e: MouseEvent) => {
      if (!containerRef.current) return;

      const sidecarRect = containerRef.current.querySelector('[data-sidecar-area]')?.getBoundingClientRect();
      if (!sidecarRect) return;

      const mouseX = e.clientX - sidecarRect.left;
      const sidecarWidth = sidecarRect.width;
      const mousePercentage = (mouseX / sidecarWidth) * 100;

      // Update all right container sizes to stay in sync for width calculation
      const newRightWidth = Math.max(20, Math.min(80, mousePercentage));
      setContainers(prev => {
        return prev.map(container => {
          if (container.position.startsWith('right-')) {
            return { ...container, size: newRightWidth };
          }
          return container;
        });
      });
    };

    const handleMouseUp = () => {
      setIsHorizontalResizing(false);
      document.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('mouseup', handleMouseUp);
    };

    document.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('mouseup', handleMouseUp);
  }, []);

  const addContainer = (position: 'top' | 'bottom' | 'right-main' | 'right-top' | 'right-bottom') => {
    console.log('Adding container at position:', position);
    const newContainer: SidecarContainer = {
      id: `${position}-${Date.now()}`,
      position,
      content: null,
      size: position.startsWith('right-') ? 
      (position === 'right-main' ? 50 : 33) : // 50% width for right-main, 33% height for right-top/bottom
      33, // 33% height for top/bottom
    };

    setContainers(prev => [...prev, newContainer]);
  };

  const removeContainer = (containerId: string) => {
    // Special handling for when user closes the main sidecar area
    if (containerId === 'main-sidecar') {
      if (sidecar) {
        sidecar.hideView();
      }
      // If there are additional containers, promote one to be the main content
      setContainers(prev => {
        if (prev.length > 0) {
          // Find the first container with content to promote to main
          const containerWithContent = prev.find(c => c.content);
          if (containerWithContent) {
            // Move its content to the main sidecar
            if (containerWithContent.content) {
              if (containerWithContent.content.type === 'localhost') {
                sidecar?.showLocalhostViewer(containerWithContent.content.fileName || 'http://localhost:3000');
              } else if (containerWithContent.content.type === 'file' && containerWithContent.content.fileName) {
                sidecar?.showFileViewer(containerWithContent.content.fileName);
              }
            }
            // Remove the promoted container from additional containers
            return prev.filter(c => c.id !== containerWithContent.id);
          }
        }
        return prev;
      });
      return;
    }

    setContainers(prev => {
      const containerToRemove = prev.find(c => c.id === containerId);
      const filtered = prev.filter(c => c.id !== containerId);
      
      // If we're removing the top container and there's a bottom container,
      // reorganize the layout to eliminate the gap
      if (containerToRemove?.position === 'top') {
        const bottomContainer = filtered.find(c => c.position === 'bottom');
        if (bottomContainer) {
          // Move bottom container to top position and reset sizes
          return filtered.map(container => {
            if (container.position === 'bottom') {
              return { ...container, position: 'top' as const, size: 50 };
            }
            return container;
          });
        }
      }
      
      // If we're removing the bottom container and there's a top container,
      // just reset the top container size to give more space to main
      if (containerToRemove?.position === 'bottom') {
        return filtered.map(container => {
          if (container.position === 'top') {
            return { ...container, size: 50 }; // Give it more space
          }
          return container;
        });
      }
      
      // For other cases, reset sizes for remaining containers
      return filtered.map(container => {
        // Reset vertical containers to default sizes when others are removed
        if (container.position === 'top' || container.position === 'bottom') {
          return { ...container, size: 50 };
        }
        // Keep right container size as is
        return container;
      });
    });
  };

  const setContainerContent = (containerId: string, content: ContainerContent) => {
    setContainers(prev => 
      prev.map(container => 
        container.id === containerId 
          ? { ...container, content }
          : container
      )
    );
  };

  // Get containers by position
  const topContainer = containers.find(c => c.position === 'top');
  const bottomContainer = containers.find(c => c.position === 'bottom');
  const rightMainContainer = containers.find(c => c.position === 'right-main');
  const rightTopContainer = containers.find(c => c.position === 'right-top');
  const rightBottomContainer = containers.find(c => c.position === 'right-bottom');

  // Calculate heights for vertical containers
  const calculateHeights = () => {
    const hasTop = !!topContainer;
    const hasBottom = !!bottomContainer;
    
    // If main sidecar is not visible, redistribute space among remaining containers
    if (!mainSidecarVisible) {
      if (hasTop && hasBottom) {
        return {
          topHeight: '50%',
          mainHeight: '0%',
          bottomHeight: '50%',
        };
      } else if (hasTop) {
        return {
          topHeight: '100%',
          mainHeight: '0%',
          bottomHeight: '0%',
        };
      } else if (hasBottom) {
        return {
          topHeight: '0%',
          mainHeight: '0%',
          bottomHeight: '100%',
        };
      }
      
      return {
        topHeight: '0%',
        mainHeight: '0%',
        bottomHeight: '0%',
      };
    }
    
    // Original logic when main sidecar is visible
    if (hasTop && hasBottom) {
      // Both containers exist - use their sizes but ensure they don't exceed 100%
      const topSize = Math.min(topContainer.size, 70);
      const bottomSize = Math.min(bottomContainer.size, 70);
      const totalUsed = topSize + bottomSize;
      
      // If total exceeds 85%, proportionally reduce both
      if (totalUsed > 85) {
        const scale = 85 / totalUsed;
        const adjustedTopSize = topSize * scale;
        const adjustedBottomSize = bottomSize * scale;
        const mainSize = 100 - adjustedTopSize - adjustedBottomSize;
        
        return {
          topHeight: `${adjustedTopSize}%`,
          mainHeight: `${mainSize}%`,
          bottomHeight: `${adjustedBottomSize}%`,
        };
      }
      
      const mainSize = 100 - topSize - bottomSize;
      return {
        topHeight: `${topSize}%`,
        mainHeight: `${mainSize}%`,
        bottomHeight: `${bottomSize}%`,
      };
    } else if (hasTop) {
      // Only top container - it takes its size, main takes the rest
      const topSize = Math.min(topContainer.size, 70);
      const mainSize = 100 - topSize;
      return {
        topHeight: `${topSize}%`,
        mainHeight: `${mainSize}%`,
        bottomHeight: '0%',
      };
    } else if (hasBottom) {
      // Only bottom container - it takes its size, main takes the rest
      const bottomSize = Math.min(bottomContainer.size, 70);
      const mainSize = 100 - bottomSize;
      return {
        topHeight: '0%',
        mainHeight: `${mainSize}%`,
        bottomHeight: `${bottomSize}%`,
      };
    }
    
    // No additional containers - main takes full height
    return {
      topHeight: '0%',
      mainHeight: '100%',
      bottomHeight: '0%',
    };
  };

  const heights = calculateHeights();

  // Calculate heights for right column containers
  const calculateRightColumnHeights = () => {
    const hasRightTop = !!rightTopContainer;
    const hasRightBottom = !!rightBottomContainer;
    const hasRightMain = !!rightMainContainer;
    
    if (!hasRightMain && !hasRightTop && !hasRightBottom) {
      return { rightTopHeight: '0%', rightMainHeight: '0%', rightBottomHeight: '0%' };
    }
    
    if (hasRightTop && hasRightMain && hasRightBottom) {
      return {
        rightTopHeight: `${rightTopContainer.size}%`,
        rightMainHeight: `${100 - rightTopContainer.size - rightBottomContainer.size}%`,
        rightBottomHeight: `${rightBottomContainer.size}%`,
      };
    }
    
    if (hasRightTop && hasRightMain) {
      return {
        rightTopHeight: `${rightTopContainer.size}%`,
        rightMainHeight: `${100 - rightTopContainer.size}%`,
        rightBottomHeight: '0%',
      };
    }
    
    if (hasRightMain && hasRightBottom) {
      return {
        rightTopHeight: '0%',
        rightMainHeight: `${100 - rightBottomContainer.size}%`,
        rightBottomHeight: `${rightBottomContainer.size}%`,
      };
    }
    
    if (hasRightMain) {
      return {
        rightTopHeight: '0%',
        rightMainHeight: '100%',
        rightBottomHeight: '0%',
      };
    }
    
    if (hasRightTop && hasRightBottom) {
      return {
        rightTopHeight: '50%',
        rightMainHeight: '0%',
        rightBottomHeight: '50%',
      };
    }
    
    if (hasRightTop) {
      return {
        rightTopHeight: '100%',
        rightMainHeight: '0%',
        rightBottomHeight: '0%',
      };
    }
    
    if (hasRightBottom) {
      return {
        rightTopHeight: '0%',
        rightMainHeight: '0%',
        rightBottomHeight: '100%',
      };
    }
    
    return { rightTopHeight: '0%', rightMainHeight: '0%', rightBottomHeight: '0%' };
  };
  const rightColumnHeights = calculateRightColumnHeights();

  // Calculate widths for horizontal split
  const hasLeftColumnContent = mainSidecarVisible || topContainer || bottomContainer;
  const hasRightColumnContent = rightMainContainer || rightTopContainer || rightBottomContainer;
  
  // Get the size from any right container (they should all have the same width size)
  const rightContainerSize = rightMainContainer?.size || rightTopContainer?.size || rightBottomContainer?.size || 50;
  
  // Dynamic width calculation based on content presence and container sizes
  let leftColumnWidth: string;
  let rightColumnWidth: string;
  
  if (hasLeftColumnContent && hasRightColumnContent) {
    // Both columns present - use container size for resizing
    rightColumnWidth = `${rightContainerSize}%`;
    leftColumnWidth = `${100 - rightContainerSize}%`;
  } else if (hasLeftColumnContent && !hasRightColumnContent) {
    // Only left column
    leftColumnWidth = '100%';
    rightColumnWidth = '0%';
  } else if (!hasLeftColumnContent && hasRightColumnContent) {
    // Only right column
    leftColumnWidth = '0%';
    rightColumnWidth = '100%';
  } else {
    // No columns
    leftColumnWidth = '0%';
    rightColumnWidth = '0%';
  }

  return (
    <div className={`h-dvh`}>
      <div
        ref={containerRef}
        className={`flex ${backgroundColor} flex-1 min-w-0 h-full min-h-0 ${removeTopPadding ? '' : 'pt-[32px]'}`}
      >
        {/* Main Content Area */}
        <div
          className="flex flex-col min-w-0 transition-all duration-300 ease-out"
          style={{
            width: isVisible ? `${100 - sidecarWidth}%` : '100%',
            minWidth: '450px',
            transition: isResizing ? 'none' : 'width 300ms ease-out',
          }}
        >
          {children}
        </div>

        {/* Resize Handle */}
        {isVisible && (
          <div
            className={`flex items-center justify-center w-1 cursor-col-resize hover:bg-borderSubtle transition-colors group ${
              isResizing ? 'bg-borderProminent' : ''
            }`}
            onMouseDown={handleMainHorizontalMouseDown}
          >
            <div
              className={`w-0.5 h-8 bg-border-subtle group-hover:bg-border-strong rounded-full transition-colors ${
                isResizing ? 'bg-border-strong' : ''
              }`}
            />
          </div>
        )}

        {/* Sidecar Panel with Multi-Container Support */}
        {isVisible && (
          <div
            className="flex transition-all duration-300 ease-out h-full"
            style={{
              width: `${sidecarWidth}%`,
              transition: isResizing ? 'none' : 'width 300ms ease-out',
            }}
            data-sidecar-area
          >
            {/* Left Column (Main + Top/Bottom containers) - Only render if has content */}
            {(mainSidecarVisible || topContainer || bottomContainer) && (
              <div 
                className="flex flex-col"
                style={{ 
                  width: leftColumnWidth,
                  transition: isHorizontalResizing ? 'none' : 'width 300ms ease-out',
                }}
              >
              {/* Top Container */}
              {topContainer && (
                <div 
                  className="relative"
                  style={{ height: heights.topHeight }}
                >
                  <IndividualContainer
                    container={topContainer}
                    onRemove={removeContainer}
                    onSetContent={setContainerContent}
                  />
                </div>
              )}

              {/* Horizontal Resize Handle (between top and main) */}
              {topContainer && (
                <div 
                  className={`h-1 cursor-row-resize hover:bg-borderSubtle transition-colors group ${
                    isVerticalResizing ? 'bg-borderProminent' : ''
                  }`}
                  onMouseDown={(e) => handleVerticalMouseDown(e, 'top-main')}
                >
                  <div 
                    className={`w-8 h-0.5 bg-border-subtle group-hover:bg-border-strong rounded-full transition-colors mx-auto mt-0.5 ${
                      isVerticalResizing ? 'bg-border-strong' : ''
                    }`} 
                  />
                </div>
              )}

              {/* Main Sidecar Container */}
              <div 
                className="relative flex-1"
                style={{ height: heights.mainHeight }}
              >
                {mainSidecarVisible && <Sidecar />}
                
                {/* Edge Hover Zones - Only show when main sidecar is visible and no container exists in that position */}
                {mainSidecarVisible && !topContainer && (
                  <div
                    className="absolute top-0 left-0 right-0 h-4 z-20 pointer-events-auto"
                    onMouseEnter={() => setHoveredEdge('top')}
                    onMouseLeave={() => setHoveredEdge(null)}
                  >
                    {hoveredEdge === 'top' && (
                      <div className="absolute top-1 left-1/2 transform -translate-x-1/2">
                        <Button
                          onClick={() => addContainer('top')}
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

                {mainSidecarVisible && !rightMainContainer && (
                  <div
                    className="absolute top-0 right-0 bottom-0 w-4 z-20 pointer-events-auto"
                    onMouseEnter={() => setHoveredEdge('right')}
                    onMouseLeave={() => setHoveredEdge(null)}
                  >
                    {hoveredEdge === 'right' && (
                      <div className="absolute right-1 top-1/2 transform -translate-y-1/2">
                        <Button
                          onClick={() => addContainer('right-main')}
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

                {mainSidecarVisible && !bottomContainer && (
                  <div
                    className="absolute bottom-0 left-0 right-0 h-4 z-20 pointer-events-auto"
                    onMouseEnter={() => setHoveredEdge('bottom')}
                    onMouseLeave={() => setHoveredEdge(null)}
                  >
                    {hoveredEdge === 'bottom' && (
                      <div className="absolute bottom-1 left-1/2 transform -translate-x-1/2">
                        <Button
                          onClick={() => addContainer('bottom')}
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

              {/* Horizontal Resize Handle (between main and bottom) */}
              {bottomContainer && (
                <div 
                  className={`h-1 cursor-row-resize hover:bg-borderSubtle transition-colors group ${
                    isVerticalResizing ? 'bg-borderProminent' : ''
                  }`}
                  onMouseDown={(e) => handleVerticalMouseDown(e, 'main-bottom')}
                >
                  <div 
                    className={`w-8 h-0.5 bg-border-subtle group-hover:bg-border-strong rounded-full transition-colors mx-auto mt-0.5 ${
                      isVerticalResizing ? 'bg-border-strong' : ''
                    }`} 
                  />
                </div>
              )}

              {/* Bottom Container */}
              {bottomContainer && (
                <div 
                  className="relative"
                  style={{ height: heights.bottomHeight }}
                >
                  <IndividualContainer
                    container={bottomContainer}
                    onRemove={removeContainer}
                    onSetContent={setContainerContent}
                  />
                </div>
              )}
              </div>
            )}

            {/* Vertical Resize Handle (between left and right columns) */}
            {(rightMainContainer || rightTopContainer || rightBottomContainer) && (
              <div 
                className={`w-1 cursor-col-resize hover:bg-borderSubtle transition-colors group ${
                  isHorizontalResizing ? 'bg-borderProminent' : ''
                }`}
                onMouseDown={handleColumnHorizontalMouseDown}
              >
                <div 
                  className={`w-0.5 h-8 bg-border-subtle group-hover:bg-border-strong rounded-full transition-colors absolute top-1/2 left-1/2 transform -translate-x-1/2 -translate-y-1/2 ${
                    isHorizontalResizing ? 'bg-border-strong' : ''
                  }`} 
                />
              </div>
            )}

            {/* Right Column */}
            {(rightMainContainer || rightTopContainer || rightBottomContainer) && (
              <div 
                className="flex flex-col"
                style={{ 
                  width: rightColumnWidth,
                  transition: isHorizontalResizing ? 'none' : 'width 300ms ease-out',
                }}
              >
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
                    className="relative flex-1"
                    style={{ height: rightColumnHeights.rightMainHeight }}
                  >
                    <IndividualContainer
                      container={rightMainContainer}
                      onRemove={removeContainer}
                      onSetContent={setContainerContent}
                    />
                    
                    {/* Right Column Hover Zones */}
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
