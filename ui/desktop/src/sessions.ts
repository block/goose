import { Session, startAgent } from './api';
import type { setViewType } from './hooks/useNavigation';

export function resumeSession(session: Session, setView: setViewType) {
  const eventDetail = {
    sessionId: session.id,
    initialMessage: undefined,
    isNewSession: false,
  };

  // Dispatch event to add session to activeSessions
  // Don't set isNewSession since this is resuming an existing session
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
    console.log('[createSession] Using recipe_id:', options.recipeId);
  } else if (options?.recipeDeeplink) {
    body.recipe_deeplink = options.recipeDeeplink;
    console.log('[createSession] Using recipe_deeplink:', options.recipeDeeplink);
  }

  console.log('[createSession] Calling startAgent with body:', body);
  const newAgent = await startAgent({
    body,
    throwOnError: true,
  });
  console.log('[createSession] Response from startAgent:', newAgent.data);
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

  // Dispatch event so sidebar can refresh the session list
  window.dispatchEvent(new CustomEvent('session-created'));

  const eventDetail = {
    sessionId: session.id,
    initialMessage: initialText,
    isNewSession: true,
  };

  // Mark as isNewSession: true so the initial message gets submitted
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
