import { defineConfig } from 'vite';

// https://vitejs.dev/config
export default defineConfig({
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
