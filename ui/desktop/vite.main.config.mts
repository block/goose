import { defineConfig } from 'vite';

// https://vitejs.dev/config
export default defineConfig({
  build: {
    rollupOptions: {
      external: [
        // ws optional peer dependencies - we don't include them, use JS fallback
        'bufferutil',
        'utf-8-validate',
      ],
    },
  },
});
