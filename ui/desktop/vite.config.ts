import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';

// Vite config for Tauri — replaces the three Electron vite configs
// (vite.main.config.mts, vite.preload.config.mts, vite.renderer.config.mts)
export default defineConfig({
  define: {
    'process.env.ALPHA': JSON.stringify(process.env.ALPHA === 'true'),
    'process.env.GOOSE_TUNNEL': JSON.stringify(
      process.env.GOOSE_TUNNEL !== 'no' && process.env.GOOSE_TUNNEL !== 'none'
    ),
  },

  plugins: [react(), tailwindcss()],

  build: {
    target: 'esnext',
    outDir: 'dist',
  },

  // Prevent Vite from obscuring Rust errors
  clearScreen: false,

  server: {
    port: 5173,
    strictPort: true,
  },
});
