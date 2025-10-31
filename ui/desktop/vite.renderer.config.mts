import { defineConfig } from 'vite';
import tailwindcss from '@tailwindcss/vite';

// https://vitejs.dev/config
export default defineConfig({
  define: {
    // This replaces process.env.ALPHA with a literal at build time
    'process.env.ALPHA': JSON.stringify(process.env.ALPHA === 'true'),
  },

  plugins: [
    tailwindcss(),
  ],

  build: {
    target: 'esnext',
    rollupOptions: {
      output: {
        // Manually chunk Monaco Editor to ensure it loads properly
        manualChunks: {
          'monaco-editor': ['monaco-editor'],
          'monaco-react': ['@monaco-editor/react'],
        },
      },
    },
  },

  optimizeDeps: {
    include: ['monaco-editor', '@monaco-editor/react'],
    esbuildOptions: {
      target: 'es2020',
    },
  },

  worker: {
    format: 'es',
  },
});
