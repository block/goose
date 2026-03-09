import { defineConfig } from 'vite';
import tailwindcss from '@tailwindcss/vite';
import { whiteLabelPlugin } from './src/whitelabel/vite-plugin';

// https://vitejs.dev/config
export default defineConfig({
  define: {
    // This replaces process.env.ALPHA with a literal at build time
    'process.env.ALPHA': JSON.stringify(process.env.ALPHA === 'true'),
    'process.env.GOOSE_TUNNEL': JSON.stringify(process.env.GOOSE_TUNNEL !== 'no' && process.env.GOOSE_TUNNEL !== 'none'),
  },

  plugins: [tailwindcss(), whiteLabelPlugin(__dirname)],

  build: {
    target: 'esnext'
  },
});
