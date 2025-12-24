import { createSession as apiCreateSession, Session } from './api';
import type { setViewType } from './hooks/useNavigation';

export function resumeSession(session: Session, setView: setViewType) {
  setView('pair', {
    disableAnimation: true,
    resumeSessionId: session.id,
  });
}

export async function createSession(options?: {
  recipeId?: string;
  recipeDeeplink?: string;
}): Promise<Session> {
  const response = await apiCreateSession({
    body: {
      working_dir: window.appConfig.get('GOOSE_WORKING_DIR') as string,
      recipe_id: options?.recipeId,
      recipe_deeplink: options?.recipeDeeplink,
    },
    throwOnError: true,
  });
  return response.data;
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
