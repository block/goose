import { defineConfig } from '@hey-api/openapi-ts';

export default defineConfig({
  input: './openapi.json',
  output: './src/api',
  plugins: [
    {
      name: '@hey-api/client-fetch',
      // Disable SSE support to avoid requiring SSE options on all requests
      sse: false,
    },
  ],
});
