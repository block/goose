import React from 'react';

export const ChatLayout = ({ children, mode }) => (
  <div className="relative w-screen h-screen overflow-hidden bg-bgApp flex flex-col">
    <div className="titlebar-drag-region" />
    <div style={{ display: mode === 'expanded' ? 'block' : 'none' }}>{children}</div>
  </div>
);
