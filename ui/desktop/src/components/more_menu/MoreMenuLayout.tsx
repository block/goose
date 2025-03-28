import MoreMenu from './MoreMenu';
import React from 'react';
import type { View } from '../../App';

export default function MoreMenuLayout({
  setView,
  setIsGoosehintsModalOpen,
  toggleWebView,
}: {
  setView: (view: View, viewOptions?: Record<any, any>) => void;
  setIsGoosehintsModalOpen: (isOpen: boolean) => void;
  toggleWebView?: () => void;
}) {
  return (
    <div className="relative flex items-center h-[36px] w-full bg-bgSubtle border-b border-borderSubtle">
      <div className="flex-1"></div>
      <div className="flex items-center h-full">
        <div className="flex items-center justify-center h-full px-2 mr-2">
          <MoreMenu
            setView={setView}
            setIsGoosehintsModalOpen={setIsGoosehintsModalOpen}
            toggleWebView={toggleWebView}
          />
        </div>
      </div>
    </div>
  );
}
