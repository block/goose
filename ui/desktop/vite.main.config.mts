import { defineConfig } from 'vite';
import { resolve } from 'path';

// https://vitejs.dev/config
export default defineConfig({
  define: {
    'process.env.GITHUB_OWNER': JSON.stringify(process.env.GITHUB_OWNER || 'block'),
    'process.env.GITHUB_REPO': JSON.stringify(process.env.GITHUB_REPO || 'goose'),
  },
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
      input: 'src/main.ts',
      output: {
        format: 'cjs',
        entryFileNames: 'main.js',
      },
      external: ['electron'],
    },
  },
});
