import { createRootRoute, createRoute } from '@tanstack/react-router';

import App from '../App';
import Timeline from '../components/Timeline';
import { TimelineProvider } from '../components/TimelineContext';

export const rootRoute = createRootRoute({
  component: App,
});

export const timelineRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  component: () => (
    <TimelineProvider>
      <div className="flex flex-col h-full">
        <Timeline />
      </div>
    </TimelineProvider>
  ),
});
