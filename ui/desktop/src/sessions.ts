import { Session, startAgent, ExtensionConfig } from './api';
import { DEFAULT_CHAT_TITLE } from './contexts/ChatContext';
import type { setViewType } from './hooks/useNavigation';
import {
  getExtensionConfigsWithOverrides,
  clearExtensionOverrides,
  hasExtensionOverrides,
} from './store/extensionOverrides';
import type { FixedExtensionEntry } from './components/ConfigContext';
import { AppEvents } from './constants/events';
import { decodeRecipe, Recipe } from './recipe';

export function getSessionDisplayName(session: Session): string {
  return session.name || DEFAULT_CHAT_TITLE;
}

export function resumeSession(session: Session, setView: setViewType) {
  const eventDetail = {
    sessionId: session.id,
    initialMessage: undefined,
  };

  window.dispatchEvent(
    new CustomEvent(AppEvents.ADD_ACTIVE_SESSION, {
      detail: eventDetail,
    })
  );

  setView('pair', {
    disableAnimation: true,
    resumeSessionId: session.id,
  });
}

export async function createSession(
  workingDir: string,
  options?: {
    recipeDeeplink?: string;
    recipeId?: string;
    extensionConfigs?: ExtensionConfig[];
    allExtensions?: FixedExtensionEntry[];
  }
): Promise<Session> {
  const body: {
    working_dir: string;
    recipe?: Recipe;
    recipe_id?: string;
    extension_overrides?: ExtensionConfig[];
  } = {
    working_dir: workingDir,
  };

  if (options?.recipeId) {
    body.recipe_id = options.recipeId;
  } else if (options?.recipeDeeplink) {
    body.recipe = await decodeRecipe(options.recipeDeeplink);
  }

  if (options?.extensionConfigs && options.extensionConfigs.length > 0) {
    body.extension_overrides = options.extensionConfigs;
  } else if (options?.allExtensions) {
    const extensionConfigs = getExtensionConfigsWithOverrides(options.allExtensions);
    if (extensionConfigs.length > 0) {
      body.extension_overrides = extensionConfigs;
    }
    if (hasExtensionOverrides()) {
      clearExtensionOverrides();
    }
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
  workingDir: string,
  options?: {
    recipeDeeplink?: string;
    recipeId?: string;
    allExtensions?: FixedExtensionEntry[];
  }
): Promise<Session> {
  const session = await createSession(workingDir, options);
  window.dispatchEvent(new CustomEvent(AppEvents.SESSION_CREATED, { detail: { session } }));

  const initialMessage = initialText ? { msg: initialText, images: [] } : undefined;

  const eventDetail = {
    sessionId: session.id,
    initialMessage,
  };

  window.dispatchEvent(
    new CustomEvent(AppEvents.ADD_ACTIVE_SESSION, {
      detail: eventDetail,
    })
  );

  setView('pair', {
    disableAnimation: true,
    initialMessage,
    resumeSessionId: session.id,
  });
  return session;
}
