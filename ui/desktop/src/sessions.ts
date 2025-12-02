import { Session, startAgent, ExtensionConfig } from './api';
import type { setViewType } from './hooks/useNavigation';
import {
  getHubExtensionConfigs,
  clearHubExtensionOverrides,
} from './components/bottom_menu/BottomMenuExtensionSelection';
import type { FixedExtensionEntry } from './components/ConfigContext';

export function resumeSession(session: Session, setView: setViewType) {
  setView('pair', {
    disableAnimation: true,
    resumeSessionId: session.id,
  });
}

export async function createSession(options?: {
  recipeId?: string;
  recipeDeeplink?: string;
  allExtensions?: FixedExtensionEntry[];
}): Promise<Session> {
  const body: {
    working_dir: string;
    recipe_id?: string;
    recipe_deeplink?: string;
    extension_overrides?: ExtensionConfig[];
  } = {
    working_dir: window.appConfig.get('GOOSE_WORKING_DIR') as string,
  };

  if (options?.recipeId) {
    body.recipe_id = options.recipeId;
  } else if (options?.recipeDeeplink) {
    body.recipe_deeplink = options.recipeDeeplink;
  }

  // Get hub extension configs with any overrides applied
  if (options?.allExtensions) {
    const extensionConfigs = getHubExtensionConfigs(options.allExtensions);
    if (extensionConfigs.length > 0) {
      body.extension_overrides = extensionConfigs;
    }
    // Clear the overrides after using them
    clearHubExtensionOverrides();
  }

  const newAgent = await startAgent({
    body,
    throwOnError: true,
  });
  return newAgent.data;
}

export async function startNewSession(
  initialText: string | undefined,
  setView: setViewType,
  options?: {
    recipeId?: string;
    recipeDeeplink?: string;
  }
): Promise<Session> {
  const session = await createSession(options);

  setView('pair', {
    disableAnimation: true,
    initialMessage: initialText,
    resumeSessionId: session.id,
  });

  return session;
}
