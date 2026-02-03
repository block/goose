import { withSentryConfig } from "@sentry/nextjs";
import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  images: {
    remotePatterns: [
      {
        protocol: "https",
        hostname: "**", // allow all https hosts
      },
    ],
  },

  // Enable browser source maps in production to help with debugging
  productionBrowserSourceMaps: true,

  // TODO: directly sending requests to API URL right now
  // reenable rewrite after we switched to secure http cookie
  // for dev portal authentication

  // rewrite does not forward Authorization header, so it
  // doesn't work with bearer token auth
  // rewrites: async () => {
  //   return [
  //     {
  //       source: "/v1/:path*",
  //       destination: `${process.env.API_URL}/v1/:path*`,
  //     },
  //   ];
  // },
};

export default withSentryConfig(nextConfig, {
  // For all available options, see:
  // https://www.npmjs.com/package/@sentry/webpack-plugin#options
  reactComponentAnnotation: {
    enabled: true,
  },

  org: "aipotheosis-labs",

  project: "mcp-gateway-portal",

  authToken: process.env.SENTRY_AUTH_TOKEN,

  // Only print logs for uploading source maps in CI
  silent: !process.env.CI,

  // For all available options, see:
  // https://docs.sentry.io/platforms/javascript/guides/nextjs/manual-setup/

  // Upload a larger set of source maps for prettier stack traces (increases build time)
  widenClientFileUpload: true,

  // Route browser requests to Sentry through a Next.js rewrite to circumvent ad-blockers.
  // This can increase your server load as well as your hosting bill.
  // Note: Check that the configured route will not match with your Next.js middleware, otherwise reporting of client-
  // side errors will fail.
  tunnelRoute: true,

  // Automatically tree-shake Sentry logger statements to reduce bundle size
  disableLogger: true,

  // Enables automatic instrumentation of Vercel Cron Monitors. (Does not yet work with App Router route handlers.)
  // See the following for more information:
  // https://docs.sentry.io/product/crons/
  // https://vercel.com/docs/cron-jobs
  automaticVercelMonitors: true,
});
