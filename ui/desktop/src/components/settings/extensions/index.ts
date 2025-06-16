// Export public API
export { DEFAULT_EXTENSION_TIMEOUT, nameToKey } from './utils';

// Export extension management functions
export {
  activateExtension,
  addToAgentOnStartup,
  updateExtension,
  toggleExtension,
  deleteExtension,
} from './extension-manager';

// Export built-in extension functions
export { syncBundledExtensions, initializeBundledExtensions } from './bundled-extensions';

// Export deeplink handling
export { addExtensionFromDeepLink } from './deeplink';

// Export agent API functions
export { addToAgent as AddToAgent, removeFromAgent as RemoveFromAgent } from './agent-api';
