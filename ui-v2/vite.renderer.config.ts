import path from 'path';
import { fileURLToPath } from 'url';

import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      '@shared': path.resolve(__dirname, './shared'),
      '@platform': path.resolve(__dirname, './src/services/platform/electron'),
    },
  },
  base: './',
  define: {
    'process.env.IS_ELECTRON': JSON.stringify(true),
  },
  build: {
    outDir: '.vite/build/renderer',
    emptyOutDir: true,
    rollupOptions: {
      input: {
        index: path.join(__dirname, 'index.html'),
      },
    },
    // Ensure production builds don't need unsafe-eval
    target: 'esnext',
    minify: 'esbuild',
  },
  root: path.join(__dirname, ''),
  publicDir: 'public',
  clearScreen: false,
  server: {
    port: 3001,
    strictPort: true,
  },
});
