import React from 'react';

interface MainLayoutProps {
  children: React.ReactNode;
}

export const MainLayout: React.FC<MainLayoutProps> = ({ children }) => {
  return (
    <div className="min-h-screen bg-background-default dark:bg-zinc-800 transition-colors duration-200">
      <div className="titlebar-drag-region" />
      <div className="p-5 max-w-3xl mx-auto" style={{ paddingTop: '52px' }}>
        {children}
      </div>
    </div>
  );
};
