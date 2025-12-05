import { defineConfig } from 'vite';
import tailwindcss from '@tailwindcss/vite';
import { resolve } from 'path';
import { copyFileSync, existsSync, mkdirSync } from 'fs';

// https://vitejs.dev/config
export default defineConfig({
  define: {
    // This replaces process.env.ALPHA with a literal at build time
    'process.env.ALPHA': JSON.stringify(process.env.ALPHA === 'true'),
  },

  plugins: [
    tailwindcss(),
    // Custom plugin to copy Olm files to public directory
    {
      name: 'copy-olm-files',
      buildStart() {
        // Ensure public directory exists
        if (!existsSync('public')) {
          mkdirSync('public', { recursive: true });
        }
        
        // Copy Olm WebAssembly file to public directory
        const olmWasmSrc = resolve('node_modules/@matrix-org/olm/olm.wasm');
        const olmWasmDest = resolve('public/olm.wasm');
        
        if (existsSync(olmWasmSrc)) {
          copyFileSync(olmWasmSrc, olmWasmDest);
          console.log('✅ Copied olm.wasm to public directory');
        } else {
          console.warn('⚠️ olm.wasm not found at:', olmWasmSrc);
        }
        
        // Also copy the JS file
        const olmJsSrc = resolve('node_modules/@matrix-org/olm/olm.js');
        const olmJsDest = resolve('public/olm.js');
        
        if (existsSync(olmJsSrc)) {
          copyFileSync(olmJsSrc, olmJsDest);
          console.log('✅ Copied olm.js to public directory');
        }
      }
    }
  ],

  build: {
    target: 'esnext'
  },

  // Configure WebAssembly support
  server: {
    fs: {
      allow: ['..', 'node_modules/@matrix-org/olm', 'public']
    }
  },
  
  // Ensure public directory is served correctly
  publicDir: 'public',

  // Ensure WebAssembly files are properly handled
  assetsInclude: ['**/*.wasm'],
  
  optimizeDeps: {
    exclude: ['@matrix-org/olm']
  }
});
