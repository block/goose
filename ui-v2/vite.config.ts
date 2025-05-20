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
    },
  },
  define: {
    'process.env.IS_ELECTRON': JSON.stringify(true),
  },
  build: {
    outDir: '.vite/build',
    emptyOutDir: true,
  },
});
