import React from 'react';

interface ProviderSetupOverlayProps {
  children: React.ReactNode;
}

/**
 * Renders the semi-transparent backdrop + blur for the modal.
 */
export default function ProviderSetupOverlay({ children }: ProviderSetupOverlayProps) {
  return (
    <div className="fixed inset-0 bg-black/20 dark:bg-white/20 backdrop-blur-sm transition-colors animate-[fadein_200ms_ease-in_forwards]">
      {children}
    </div>
  );
}
