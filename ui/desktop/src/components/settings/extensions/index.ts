export { DEFAULT_EXTENSION_TIMEOUT, nameToKey } from './utils';

export {
  activateExtensionDefault,
  toggleExtensionDefault,
  deleteExtension,
} from './extension-manager';

export { syncBundledExtensions, initializeBundledExtensions } from './bundled-extensions';

export { addExtensionFromDeepLink } from './deeplink';

export { addToAgent, removeFromAgent } from './agent-api';
