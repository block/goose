import { defineConfig } from 'vite';
import tailwindcss from '@tailwindcss/vite';
import path from 'path';

// Web-only Vite config — builds the React UI as static files
// for serving via goose-web (no Electron, no preload).
export default defineConfig({
  define: {
    'process.env.ALPHA': JSON.stringify(false),
    'process.env.GOOSE_TUNNEL': JSON.stringify(false),
  },

  plugins: [tailwindcss()],

  resolve: {
    alias: {
      // Ensure the same src root as electron renderer
      '@': path.resolve(__dirname, 'src'),
    },
  },

  build: {
    target: 'esnext',
    outDir: 'dist-web',
    emptyOutDir: true,
  },
});
