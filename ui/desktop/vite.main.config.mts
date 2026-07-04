import { defineConfig } from 'vite';

// https://vitejs.dev/config
export default defineConfig({
  define: {
    'process.env.GITHUB_OWNER': JSON.stringify(process.env.GITHUB_OWNER || 'repo-makeover'),
    'process.env.GITHUB_REPO': JSON.stringify(process.env.GITHUB_REPO || 'gosling'),
    'process.env.GOSLING_BUNDLE_NAME': JSON.stringify(process.env.GOSLING_BUNDLE_NAME || 'Gosling'),
  },
});
