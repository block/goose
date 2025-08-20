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
      // Force @mcp-ui/client to use the same React version as the main app
      react: resolve(__dirname, 'node_modules/react'),
      'react-dom': resolve(__dirname, 'node_modules/react-dom'),
    },
    // Deduplicate React packages
    dedupe: ['react', 'react-dom'],
  },

  build: {
    target: 'esnext',
  },
});
