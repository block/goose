import { defineConfig } from '@hey-api/openapi-ts';

export default defineConfig({
  input: './openapi.json',
  output: './src/api',
  plugins: ['@hey-api/client-fetch'],
});
