import React from 'react';

export const MainPanelLayout: React.FC<{
  children: React.ReactNode;
  removeTopPadding?: boolean;
}> = ({ children, removeTopPadding = false }) => {
  return (
    <div className={`h-dvh`}>
      {/* Padding top matches the app toolbar drag area height - can be removed for full bleed */}
      <div className={`flex flex-col bg-background-muted flex-1 min-w-0 h-full min-h-0 ${removeTopPadding ? '' : 'pt-[32px]'}`}>
        {children}
      </div>
    </div>
  );
};
