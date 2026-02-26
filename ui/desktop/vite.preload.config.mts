import { defineConfig } from 'vite';
import { resolve } from 'path';

// https://vitejs.dev/config
export default defineConfig({
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
  },
  build: {
    ssr: true,
    outDir: '.vite/build',
    rollupOptions: {
      input: 'src/preload.ts',
      output: {
        format: 'cjs',
        entryFileNames: 'preload.js'
      },
      external: ['electron']
    }
  }
});
