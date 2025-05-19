import { builtinModules } from 'module';
import path from 'path';

import { defineConfig } from 'vite';

// https://vitejs.dev/config/
export default defineConfig({
  build: {
    outDir: '.vite/build',
    lib: {
      entry: {
        main: path.join(__dirname, 'electron/main.ts'),
        preload: path.join(__dirname, 'electron/preload.ts'),
      },
      formats: ['cjs'],
    },
    rollupOptions: {
      external: ['electron', ...builtinModules],
      output: {
        format: 'cjs',
        entryFileNames: '[name].js',
      },
    },
    emptyOutDir: false,
    sourcemap: true,
    minify: false,
  },
});
