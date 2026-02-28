import { startAgent, type ExtensionConfig, type Session } from '@/api';
import { AppEvents } from '@/constants/events';
import type { FixedExtensionEntry } from '@/contexts/ConfigContext';
import type { setViewType } from '@/hooks/useNavigation';
import { decodeRecipe, type Recipe } from '@/recipe';
import {
  clearExtensionOverrides,
  getExtensionConfigsWithOverrides,
  hasExtensionOverrides,
} from '@/store/extensionOverrides';

export function shouldShowNewChatTitle(session: Session): boolean {
  if (session.recipe) {
    return false;
  }
  return !session.user_set_name && session.message_count === 0;
}

export function resumeSession(session: Session, setView: setViewType) {
  window.dispatchEvent(
    new CustomEvent(AppEvents.ADD_ACTIVE_SESSION, {
      detail: {
        sessionId: session.id,
        initialMessage: undefined,
      },
    })
  );

  // If the user clicks the currently-active session again, React Router may treat the navigation
  // as a no-op (same path + same search params). Including a unique state value forces the
  // navigation to re-run and ensures the session is reliably reloaded.
  setView('session', {
    disableAnimation: true,
    resumeSessionId: session.id,
    __navNonce: crypto.randomUUID(),
  } as unknown as Parameters<setViewType>[1]);
}

export async function createSession(
  workingDir: string,
  options?: {
    recipeDeeplink?: string;
    extensionConfigs?: ExtensionConfig[];
    allExtensions?: FixedExtensionEntry[];
  }
): Promise<Session> {
  const body: {
    working_dir: string;
    recipe?: Recipe;
    extension_overrides?: ExtensionConfig[];
  } = {
    working_dir: workingDir,
  };

  if (options?.recipeDeeplink) {
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

  setView('session', {
    disableAnimation: true,
    initialMessage,
    resumeSessionId: session.id,
  });
  return session;
}
