import { builtinModules } from 'module';

import { defineConfig } from 'vite';

export default defineConfig({
  root: process.cwd(),
  build: {
    outDir: '.vite/build/preload',
    lib: {
      entry: 'electron/preload.ts',
      formats: ['cjs'],
      fileName: () => 'preload.js',
    },
    rollupOptions: {
      external: ['electron', ...builtinModules],
      output: {
        format: 'cjs',
        entryFileNames: 'preload.js',
        sourcemap: false,
      },
    },
    emptyOutDir: true,
    sourcemap: false,
    minify: false,
  },
});
