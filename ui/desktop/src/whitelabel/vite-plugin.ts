/**
 * Vite plugin that loads whitelabel.yaml at build time and injects
 * the resolved config as a global constant `__WHITELABEL_CONFIG__`.
 */
import type { Plugin } from 'vite';
import { loadWhiteLabelConfig } from './loader';

export function whiteLabelPlugin(projectRoot: string): Plugin {
  return {
    name: 'whitelabel-config',
    config() {
      const config = loadWhiteLabelConfig(projectRoot);
      return {
        define: {
          __WHITELABEL_CONFIG__: JSON.stringify(config),
        },
      };
    },
  };
}
