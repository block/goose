import React, { useState, useRef, useCallback } from 'react';
import { useLocation } from 'react-router-dom';
import { Sidecar, useSidecar } from '../SidecarLayout';

export const MainPanelLayout: React.FC<{
  children: React.ReactNode;
  removeTopPadding?: boolean;
  backgroundColor?: string;
}> = ({ children, removeTopPadding = false, backgroundColor = 'bg-background-default' }) => {
  const location = useLocation();
  const sidecar = useSidecar();
  
  // Only show sidecar on chat-related pages
  const shouldShowSidecar = location.pathname === '/' || location.pathname === '/chat' || location.pathname === '/pair';
  const isVisible = shouldShowSidecar && sidecar?.activeView && sidecar?.views.find((v) => v.id === sidecar.activeView);

  // State for resizing
  const [sidecarWidth, setSidecarWidth] = useState(50); // Percentage
  const [isResizing, setIsResizing] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
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

  return (
    <div className={`h-dvh`}>
      {/* Padding top matches the app toolbar drag area height - can be removed for full bleed */}
      <div
        ref={containerRef}
        className={`flex ${backgroundColor} flex-1 min-w-0 h-full min-h-0 ${removeTopPadding ? '' : 'pt-[32px]'}`}
      >
        {/* Main Content Area */}
        <div
          className="flex flex-col min-w-0 transition-all duration-300 ease-out"
          style={{
            width: isVisible ? `${100 - sidecarWidth}%` : '100%',
            minWidth: '450px', // Ensure main content never goes below 450px
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
            onMouseDown={handleMouseDown}
          >
            <div
              className={`w-0.5 h-8 bg-border-subtle group-hover:bg-border-strong rounded-full transition-colors ${
                isResizing ? 'bg-border-strong' : ''
              }`}
            />
          </div>
        )}

        {/* Sidecar Panel */}
        {isVisible && (
          <div
            className="transition-all duration-300 ease-out h-full"
            style={{
              width: `${sidecarWidth}%`,
              transition: isResizing ? 'none' : 'width 300ms ease-out',
            }}
          >
            <Sidecar />
          </div>
        )}
      </div>
    </div>
  );
};
