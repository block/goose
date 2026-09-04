import { defineConfig } from 'vite';

// https://vitejs.dev/config
export default defineConfig({
  define: {
    'process.env.GITHUB_OWNER': JSON.stringify(process.env.GITHUB_OWNER || 'aaif-goose'),
    'process.env.GITHUB_REPO': JSON.stringify(process.env.GITHUB_REPO || 'goose'),
    'process.env.GOOSE_BUNDLE_NAME': JSON.stringify(process.env.GOOSE_BUNDLE_NAME || 'Goose'),
    'MAIN_WINDOW_VITE_DEV_SERVER_URL': 'undefined',
    'MAIN_WINDOW_VITE_NAME': JSON.stringify('main_window'),
  },
});
