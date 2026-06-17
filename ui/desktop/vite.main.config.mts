import { defineConfig } from 'vite';
import { readFileSync } from 'node:fs';

const productMetadata = JSON.parse(
  readFileSync(new URL('../../distro/security-cn/branding/product-metadata.json', import.meta.url), 'utf8')
) as { productName?: string };

// https://vitejs.dev/config
export default defineConfig({
  define: {
    'process.env.GITHUB_OWNER': JSON.stringify(process.env.GITHUB_OWNER || 'aaif-goose'),
    'process.env.GITHUB_REPO': JSON.stringify(process.env.GITHUB_REPO || 'goose'),
    'process.env.GOOSE_BUNDLE_NAME': JSON.stringify(
      process.env.GOOSE_BUNDLE_NAME || productMetadata.productName || 'Goose'
    ),
  },
});
