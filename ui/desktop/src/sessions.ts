import { Session, startAgent } from './api';
import type { setViewType } from './hooks/useNavigation';
import { DEFAULT_CHAT_TITLE } from './contexts/ChatContext';

/**
 * Check if a session name is a default/temporary name that should be updated
 * @param name - The session name to check
 * @returns true if the name is a default/temporary name
 */
export function isDefaultSessionName(name: string | undefined | null): boolean {
  if (!name) return false;

  // Check if it's exactly the default title or starts with it (for numbered versions)
  return name === DEFAULT_CHAT_TITLE || name.startsWith(DEFAULT_CHAT_TITLE);
}

export function resumeSession(session: Session, setView: setViewType) {
  const eventDetail = {
    sessionId: session.id,
    initialMessage: undefined,
  };

  window.dispatchEvent(
    new CustomEvent('add-active-session', {
      detail: eventDetail,
    })
  );

  setView('pair', {
    disableAnimation: true,
    resumeSessionId: session.id,
  });
}

export async function createSession(options?: {
  recipeId?: string;
  recipeDeeplink?: string;
}): Promise<Session> {
  const body: {
    working_dir: string;
    recipe_id?: string;
    recipe_deeplink?: string;
  } = {
    working_dir: window.appConfig.get('GOOSE_WORKING_DIR') as string,
  };

  if (options?.recipeId) {
    body.recipe_id = options.recipeId;
  } else if (options?.recipeDeeplink) {
    body.recipe_deeplink = options.recipeDeeplink;
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

  window.dispatchEvent(new CustomEvent('session-created'));

  const eventDetail = {
    sessionId: session.id,
    initialMessage: initialText,
  };

  window.dispatchEvent(
    new CustomEvent('add-active-session', {
      detail: eventDetail,
    })
  );

  setView('pair', {
    disableAnimation: true,
    initialMessage: initialText,
    resumeSessionId: session.id,
  });
  return session;
}
