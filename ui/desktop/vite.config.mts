import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { resolve } from 'path';
import tailwindcss from '@tailwindcss/vite';

// https://vitejs.dev/config
export default defineConfig({
  plugins: [react(), tailwindcss()],
  base: './',
  build: {
    rollupOptions: {
      input: {
        main: resolve(__dirname, 'src/main.ts'),
        index: resolve(__dirname, 'index.html'),
      },
    },
  },
});
