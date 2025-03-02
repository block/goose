import React from 'react';
import { Card } from '../../../../ui/card';

interface ProviderSetupOverlayProps {
  children: React.ReactNode;
}

/**
 * Renders the semi-transparent backdrop + blur for the modal.
 */
export default function ProviderSetupOverlay({ children }: ProviderSetupOverlayProps) {
  return (
    <div className="fixed inset-0 bg-black/20 dark:bg-white/20 backdrop-blur-sm transition-colors animate-[fadein_200ms_ease-in_forwards]">
      <Card className="fixed top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[500px] bg-bgApp rounded-xl overflow-hidden shadow-none p-[16px] pt-[24px] pb-0">
        <div className="px-4 pb-0 space-y-6">{children}</div>
      </Card>
    </div>
  );
}
