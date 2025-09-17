import React, { useCallback, useRef } from 'react';
import SidecarPanel from '../Sidecar/SidecarPanel';
import { SidecarProvider, useSidecar } from '../Sidecar/SidecarContext';

export const MainPanelLayout: React.FC<{
  children: React.ReactNode;
  removeTopPadding?: boolean;
  backgroundColor?: string;
}> = ({ children, removeTopPadding = false, backgroundColor = 'bg-background-default' }) => {
  const ResizableSeparator = () => {
    const { isOpen, setWidthPct, close } = useSidecar();
    const dragging = useRef(false);

    const onMouseDown = useCallback(
      (e: React.MouseEvent) => {
        if (!isOpen) return;
        dragging.current = true;
        document.body.style.cursor = 'col-resize';
        e.preventDefault();
        e.stopPropagation();
      },
      [isOpen]
    );

    const onMouseMove = useCallback(
      (e: MouseEvent) => {
        if (!dragging.current) return;
        const container = document.querySelector('#main-panel-layout-container') as HTMLDivElement;
        if (!container) return;
        const rect = container.getBoundingClientRect();
        const totalWidth = rect.width || 1;
        const distanceFromRight = Math.max(0, Math.min(rect.width, rect.right - e.clientX));
        const next = distanceFromRight / totalWidth; // fraction of container taken by sidecar
        // Auto-collapse if below 12% (similar feel to left rail)
        if (next < 0.12) {
          setWidthPct(0.3); // store a sensible default when re-opened
          close();
          dragging.current = false;
          document.body.style.cursor = '';
          return;
        }
        setWidthPct(next);
      },
      [setWidthPct, close]
    );

    const onMouseUp = useCallback(() => {
      if (!dragging.current) return;
      dragging.current = false;
      document.body.style.cursor = '';
    }, []);

    React.useEffect(() => {
      window.addEventListener('mousemove', onMouseMove);
      window.addEventListener('mouseup', onMouseUp);
      return () => {
        window.removeEventListener('mousemove', onMouseMove);
        window.removeEventListener('mouseup', onMouseUp);
      };
    }, [onMouseMove, onMouseUp]);

    return (
      <div className={`relative ${isOpen ? 'opacity-100' : 'opacity-0 pointer-events-none'}`}>
        <div className="absolute inset-y-0 -left-[3px] w-[6px]" />
        <div
          role="separator"
          aria-orientation="vertical"
          className={`w-[6px] cursor-col-resize bg-transparent hover:bg-borderSubtle/60 active:bg-borderSubtle transition-colors`}
          onMouseDown={onMouseDown}
          title="Resize side panel"
        />
      </div>
    );
  };

  return (
    <SidecarProvider>
      <div className={`h-dvh`}>
        <div
          id="main-panel-layout-container"
          className={`flex ${backgroundColor} flex-1 min-w-0 h-full min-h-0`}
        >
          <div className={`flex flex-col flex-1 min-w-0 ${removeTopPadding ? '' : 'pt-[32px]'}`}>
            {children}
          </div>
          <ResizableSeparator />
          <SidecarPanel />
        </div>
      </div>
    </SidecarProvider>
  );
};
