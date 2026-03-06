import type { WhiteLabelConfig } from './types';

declare global {
  /**
   * Injected at build time by the whitelabel Vite plugin.
   * Contains the fully resolved white-label configuration.
   */
  const __WHITELABEL_CONFIG__: WhiteLabelConfig;
}
