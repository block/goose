import path from 'path';
import { fileURLToPath } from 'url';

import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@shared': path.resolve(__dirname, './shared'),
      '@platform': path.resolve(__dirname, './src/services/platform/web'),
    },
  },
  define: {
    'process.env.IS_ELECTRON': JSON.stringify(false),
  },
  build: {
    outDir: 'dist/web',
    emptyOutDir: true,
  },
  server: {
    port: 3000,
    strictPort: true,
  },
});
