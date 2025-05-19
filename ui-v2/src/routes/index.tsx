import { createRootRoute, createRoute } from '@tanstack/react-router';

import App from '../App';

export const rootRoute = createRootRoute({
  component: App,
});

export const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/',
  component: () => (
    <div className="p-5">
      <h2>Welcome to Goose v2</h2>
    </div>
  ),
});

export const aboutRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: '/about',
  component: () => (
    <div className="p-5">
      <h2>About Goose v2</h2>
      <p>An AI assistant for developers</p>
    </div>
  ),
});
