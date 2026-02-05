import { defineConfig } from 'vite';
import { builtinModules } from 'node:module';

// Pi and all its transitive deps that have native/ws issues
const piExternals = [
  '@mariozechner/pi-coding-agent',
  '@mariozechner/pi-agent-core', 
  '@mariozechner/pi-ai',
  '@mariozechner/pi-tui',
  '@mariozechner/jiti',
  '@mariozechner/clipboard',
  '@google/genai',
  'openai',
  'ws',
  'bufferutil',
  'utf-8-validate',
];

// https://vitejs.dev/config
export default defineConfig({
  define: {
    'process.env.GITHUB_OWNER': JSON.stringify(process.env.GITHUB_OWNER || 'block'),
    'process.env.GITHUB_REPO': JSON.stringify(process.env.GITHUB_REPO || 'goose'),
    // Disable ws native module lookups - they break Vite bundling
    'process.env.WS_NO_BUFFER_UTIL': JSON.stringify('1'),
    'process.env.WS_NO_UTF_8_VALIDATE': JSON.stringify('1'),
  },
  build: {
    rollupOptions: {
      external: (id) => {
        // Node builtins
        if (builtinModules.includes(id) || id.startsWith('node:')) return true;
        // Pi and its problematic transitive deps
        for (const ext of piExternals) {
          if (id === ext || id.startsWith(ext + '/')) return true;
        }
        return false;
      },
    },
  },
});
