import React from 'react';
import { ProviderGrid } from '../settings/providers/ProviderGrid';
import { ScrollArea } from '../ui/scroll-area';
import BackButton from '../ui/BackButton';

interface WelcomeScreenProps {
  onSubmit?: () => void;
}

export function WelcomeScreen({ onSubmit }: WelcomeScreenProps) {
  return (
    <div className="h-screen w-full">
      {/* Add draggable title bar region */}
      <div className="h-[36px] w-full bg-transparent window-drag" />

      <div className="h-[calc(100vh-36px)] w-full bg-white dark:bg-gray-800 overflow-hidden p-2 pt-0">
        <ScrollArea className="h-full w-full">
          <div className="flex min-h-full">
            {/* Content Area */}
            <div className="flex-1 px-16 py-8 pt-[20px]">
              <div className="max-w-3xl space-y-12">
                <div className="flex items-center gap-4 mb-8">
                  <h1 className="text-2xl font-semibold tracking-tight">Choose a Provider</h1>
                </div>
                <ProviderGrid onSubmit={onSubmit} />
              </div>
            </div>
          </div>
        </ScrollArea>
      </div>
    </div>
  );
}
