// This file configures the initialization of Sentry on the client.
// The config you add here will be used whenever a users loads a page in their browser.
// https://docs.sentry.io/platforms/javascript/guides/nextjs/

import * as Sentry from "@sentry/nextjs";
import { SentryOptions } from "./sentryoptions";

if (process.env.NEXT_PUBLIC_ENVIRONMENT && process.env.NEXT_PUBLIC_ENVIRONMENT !== "local") {
  Sentry.init({
    dsn: process.env.NEXT_PUBLIC_SENTRY_DSN || "",

    environment: process.env.NEXT_PUBLIC_ENVIRONMENT,

    // Add optional integrations for additional features
    integrations: [Sentry.replayIntegration(), Sentry.browserTracingIntegration()],

    sendDefaultPii: true,

    // Define how likely traces are sampled. Adjust this value in production, or use tracesSampler for greater control.
    tracesSampleRate:
      SentryOptions[process.env.NEXT_PUBLIC_ENVIRONMENT as "development" | "production"]
        .tracesSampleRate,

    // Define how likely Replay events are sampled.
    // This sets the sample rate to be 10%. You may want this to be 100% while
    // in development and sample at a lower rate in production
    replaysSessionSampleRate:
      SentryOptions[process.env.NEXT_PUBLIC_ENVIRONMENT as "development" | "production"]
        .replaysSessionSampleRate,

    // Define how likely Replay events are sampled when an error occurs.
    replaysOnErrorSampleRate:
      SentryOptions[process.env.NEXT_PUBLIC_ENVIRONMENT as "development" | "production"]
        .replaysOnErrorSampleRate,

    // Setting this option to true will print useful information to the console while you're setting up Sentry.
    debug: false,
  });
}
