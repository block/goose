import React from 'react';
import { Sidecar, useSidecar } from '../SidecarLayout';

export const MainPanelLayout: React.FC<{
  children: React.ReactNode;
  removeTopPadding?: boolean;
  backgroundColor?: string;
}> = ({ children, removeTopPadding = false, backgroundColor = 'bg-background-default' }) => {
  const sidecar = useSidecar();
  const isVisible = sidecar?.activeView && sidecar?.views.find((v) => v.id === sidecar.activeView);

  return (
    <div className={`h-dvh`}>
      {/* Padding top matches the app toolbar drag area height - can be removed for full bleed */}
      <div
        className={`flex ${backgroundColor} flex-1 min-w-0 h-full min-h-0 ${removeTopPadding ? '' : 'pt-[32px]'}`}
      >
        {/* Main Content Area */}
        <div className="flex flex-col flex-1 min-w-0 transition-all duration-300 ease-out">
          {children}
        </div>

        {/* Sidecar Panel */}
        {isVisible && <Sidecar className="flex-1 transition-all duration-300 ease-out" />}
      </div>
    </div>
  );
};
