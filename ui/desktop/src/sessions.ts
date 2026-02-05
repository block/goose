import { Session, startAgent, ExtensionConfig } from './api';
import type { setViewType } from './hooks/useNavigation';
import {
  getExtensionConfigsWithOverrides,
  clearExtensionOverrides,
  hasExtensionOverrides,
} from './store/extensionOverrides';
import type { FixedExtensionEntry } from './components/ConfigContext';
import { AppEvents } from './constants/events';
import type { AgentBackend } from './hooks/useAgentChat';

export function shouldShowNewChatTitle(session: Session): boolean {
  if (session.recipe) {
    return false;
  }
  return !session.user_set_name && session.message_count === 0;
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

/**
 * Create a new session using the Goose backend (goosed).
 */
export async function createGooseSession(
  workingDir: string,
  options?: {
    recipeId?: string;
    recipeDeeplink?: string;
    extensionConfigs?: ExtensionConfig[];
    allExtensions?: FixedExtensionEntry[];
  }
): Promise<Session> {
  const body: {
    working_dir: string;
    recipe_id?: string;
    recipe_deeplink?: string;
    extension_overrides?: ExtensionConfig[];
  } = {
    working_dir: workingDir,
  };

  if (options?.recipeId) {
    body.recipe_id = options.recipeId;
  } else if (options?.recipeDeeplink) {
    body.recipe_deeplink = options.recipeDeeplink;
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

/**
 * Create a new session using the Pi backend.
 */
export async function createPiSession(workingDir: string): Promise<Session> {
  const result = await window.electron.pi.createSession({ workingDir });

  if (!result.success || !result.session) {
    throw new Error(result.error || 'Failed to create Pi session');
  }

  // Convert Pi session to Goose Session type
  const piSession = result.session;
  return {
    id: piSession.id,
    name: piSession.name,
    created_at: piSession.created_at,
    updated_at: piSession.updated_at,
    working_dir: piSession.working_dir,
    message_count: piSession.message_count,
    conversation: piSession.conversation as Session['conversation'],
    input_tokens: piSession.input_tokens,
    output_tokens: piSession.output_tokens,
    total_tokens: piSession.total_tokens,
    accumulated_input_tokens: piSession.accumulated_input_tokens,
    accumulated_output_tokens: piSession.accumulated_output_tokens,
    accumulated_total_tokens: piSession.accumulated_total_tokens,
    extension_data: {},
  };
}

/**
 * Create a new session using the configured backend.
 */
export async function createSession(
  workingDir: string,
  backend: AgentBackend,
  options?: {
    recipeId?: string;
    recipeDeeplink?: string;
    extensionConfigs?: ExtensionConfig[];
    allExtensions?: FixedExtensionEntry[];
  }
): Promise<Session> {
  if (backend === 'pi') {
    return createPiSession(workingDir);
  }
  return createGooseSession(workingDir, options);
}

export async function startNewSession(
  initialText: string | undefined,
  setView: setViewType,
  workingDir: string,
  backend: AgentBackend,
  options?: {
    recipeId?: string;
    recipeDeeplink?: string;
    allExtensions?: FixedExtensionEntry[];
  }
): Promise<Session> {
  const session = await createSession(workingDir, backend, options);

  // Include session data so sidebar can add it immediately (before it has messages)
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
