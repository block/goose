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
    // main + preload share the same outDir; don't wipe files produced by the other build.
    emptyOutDir: false,
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
