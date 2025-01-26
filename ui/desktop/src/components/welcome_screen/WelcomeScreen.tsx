import React from 'react';
import { ProviderGrid } from './ProviderGrid';
import { ScrollArea } from '../ui/scroll-area';
import GooseSplashLogo from '../GooseSplashLogoGradient';
import { Button } from '../ui/button';

// Extending React CSSProperties to include custom webkit property
declare module 'react' {
  interface CSSProperties {
    WebkitAppRegion?: string;
  }
}

interface WelcomeScreenProps {
  onSubmit?: () => void;
}

export function WelcomeScreen({ onSubmit }: WelcomeScreenProps) {
  return (
    <div className="h-screen w-full select-none bg-white dark:bg-black">
      {/* Draggable title bar region */}
      <div className="h-[36px] w-full bg-transparent" style={{ WebkitAppRegion: 'drag' }} />

      {/* Content area - explicitly set as non-draggable */}
      <div
        className="h-[calc(100vh-36px)] w-full overflow-hidden"
        style={{ WebkitAppRegion: 'no-drag' }}
      >
        <ScrollArea className="h-full w-full">
          <div className="flex min-h-full flex-col justify-center px-4 py-8 md:px-16 max-w-4xl mx-auto">
            {/* Header Section */}
            <div className="mb-12 space-y-4">
              <GooseSplashLogo className="h-24 w-24 md:h-32 md:w-32" />
              <h1 className="text-4xl font-bold text-textStandard tracking-tight md:text-5xl">
                Welcome to goose
              </h1>
              <p className="text-lg text-textSubtle max-w-2xl">
                Your intelligent AI assistant for seamless productivity and creativity.
              </p>
            </div>

            {/* ProviderGrid */}
            <div className="w-full">
              <h2 className="text-3xl font-bold text-textStandard tracking-tight mb-2">
                Choose a Provider
              </h2>
              <p className="text-xl text-textStandard mb-4">
                Select an AI model provider to get started with goose.
              </p>
              <p className="text-sm text-textSubtle mb-8">
                Click on a provider to configure its API keys and start using goose. Your keys are
                stored securely and encrypted locally. You can change your provider and select
                specific models in the settings.
              </p>
              <ProviderGrid onSubmit={onSubmit} />
            </div>

            {/* Get started (now less prominent) */}
            <div className="mt-12">
              <p className="text-sm text-textSubtle">
                Not sure where to start?{' '}
                <Button
                  variant="link"
                  className="text-indigo-500 hover:text-indigo-600 p-0 h-auto"
                  onClick={() =>
                    window.open('https://block.github.io/goose/v1/docs/quickstart', '_blank')
                  }
                >
                  Quick Start Guide
                </Button>
              </p>
            </div>
          </div>
        </ScrollArea>
      </div>
    </div>
  );
}
