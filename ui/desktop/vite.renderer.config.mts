import { defineConfig } from 'vite';
import tailwindcss from '@tailwindcss/vite';
import { resolve } from 'path';

// https://vitejs.dev/config
export default defineConfig({
  define: {
    // This replaces process.env.ALPHA with a literal at build time
    'process.env.ALPHA': JSON.stringify(process.env.ALPHA === 'true'),
  },

  plugins: [tailwindcss()],

  resolve: {
    alias: {
      '@': resolve(__dirname, './src'),
      '@components': resolve(__dirname, './src/components'),
      '@ui': resolve(__dirname, './src/components/ui'),
      '@features': resolve(__dirname, './src/components/features'),
      '@utils': resolve(__dirname, './src/utils'),
      '@types': resolve(__dirname, './src/types'),
      '@icons': resolve(__dirname, './src/components/icons'),
    },
  },
});
