import { defineConfig } from 'vite';

// https://vitejs.dev/config
export default defineConfig({
    define: {
        // This replaces process.env.ALPHA with a literal at build time
        'process.env.ALPHA': JSON.stringify(process.env.ALPHA === 'true'),
    },
});
