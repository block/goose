import { createRouter } from '@tanstack/react-router';

import { rootRoute, timelineRoute } from './routes';

const routeTree = rootRoute.addChildren([timelineRoute]);

export const router = createRouter({ routeTree });

// Register the router instance for type safety
declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}
