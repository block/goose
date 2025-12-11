import { Session, startAgent, restartAgent, ExtensionConfig } from './api';
import type { setViewType } from './hooks/useNavigation';
import {
  getWorkingDir,
  getExtensionConfigsWithOverrides,
  clearExtensionOverrides,
  hasExtensionOverrides,
} from './store/newChatState';
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
    working_dir: getWorkingDir(),
  };

  // Note: We intentionally don't clear workingDir from newChatState here
  // so that new sessions in the same window continue to use the last selected directory

  if (options?.recipeId) {
    body.recipe_id = options.recipeId;
  } else if (options?.recipeDeeplink) {
    body.recipe_deeplink = options.recipeDeeplink;
  }

  // Get extension configs with any overrides applied
  if (options?.allExtensions && hasExtensionOverrides()) {
    const extensionConfigs = getExtensionConfigsWithOverrides(options.allExtensions);
    if (extensionConfigs.length > 0) {
      body.extension_overrides = extensionConfigs;
    }
    // Clear the overrides after using them
    clearExtensionOverrides();
  }

  const newAgent = await startAgent({
    body,
    throwOnError: true,
  });

  const session = newAgent.data;

  // Restart agent to ensure it picks up the session's working dir
  await restartAgent({
    body: { session_id: session.id },
  });

  return session;
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
