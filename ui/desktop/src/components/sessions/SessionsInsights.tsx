import { useState } from 'react';
import { Card, CardContent } from '../ui/card';
import { Greeting } from '../common/Greeting';
import { Goose } from '../icons/Goose';
import { Skeleton } from '../ui/skeleton';
import { RecentDirectories } from '../RecentDirectories';
import { RecentFiles } from '../RecentFiles';

export function SessionInsights() {
  const [isCreatingSession, setIsCreatingSession] = useState(false);

  const renderSkeleton = () => (
    <div className="bg-background-muted flex flex-col h-full">
      {/* Header container with rounded bottom */}
      <div className="bg-background-default rounded-b-2xl mb-0.5">
        <div className="px-8 pb-8 pt-19 space-y-4">
          <div className="origin-bottom-left goose-icon-animation">
            <Goose className="size-8" />
          </div>
          <Greeting />
        </div>
      </div>

      {/* Recent directories section header */}
      <div className="px-8 py-4">
        <h2 className="text-lg font-medium text-text-default">Recent Projects</h2>
        <p className="text-sm text-text-muted">Start a session in one of your recent directories</p>
      </div>

      {/* Recent directories skeleton */}
      <div className="flex flex-col flex-1 space-y-0.5 px-0">
        {[1, 2, 3].map((i) => (
          <Card key={i} className="w-full py-6 px-6 border-none rounded-2xl bg-background-default">
            <CardContent className="p-0">
              <div className="flex items-center gap-4">
                <Skeleton className="h-12 w-12 rounded-xl" />
                <div className="flex-1">
                  <Skeleton className="h-6 w-48 mb-2" />
                  <Skeleton className="h-4 w-64" />
                </div>
              </div>
            </CardContent>
          </Card>
        ))}
        {/* Filler container */}
        <div className="bg-background-default rounded-2xl flex-1"></div>
      </div>
    </div>
  );

  if (isCreatingSession) {
    return renderSkeleton();
  }

  return (
    <div className="bg-background-muted flex flex-col h-full">
      {/* Header container with rounded bottom */}
      <div className="bg-background-default rounded-b-2xl mb-0.5">
        <div className="px-8 pb-8 pt-19 space-y-4">
          <div className="origin-bottom-left goose-icon-animation">
            <Goose className="size-8" />
          </div>
          <Greeting />
        </div>
      </div>

      {/* Recent directories section header */}
      <div className="px-8 py-4">
        <h2 className="text-lg font-medium text-text-default">Recent Projects</h2>
        <p className="text-sm text-text-muted">Start a session in one of your recent directories</p>
      </div>

      {/* Recent directories panels */}
      <div className="flex flex-col flex-1 space-y-0.5 overflow-auto">
        <RecentDirectories onSessionStarting={() => setIsCreatingSession(true)} />

        {/* Recent files section */}
        <div className="bg-background-default rounded-2xl px-6 py-4">
          <h3 className="text-sm font-medium text-text-default mb-1">Pick up where you left off</h3>
          <p className="text-xs text-text-muted mb-3">Recently touched files and projects</p>
          <RecentFiles onSessionStarting={() => setIsCreatingSession(true)} />
        </div>

        {/* Filler container - extends to fill remaining space */}
        <div className="bg-background-default rounded-2xl flex-1"></div>
      </div>
    </div>
  );
}
