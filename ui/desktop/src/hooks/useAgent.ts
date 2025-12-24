import { useCallback, useRef, useState } from 'react';
import { useConfig } from '../components/ConfigContext';
import { ChatType } from '../types/chat';
import { loadExtensionsProgressively } from '../utils/extensionUtils';
import {
  backupConfig,
  initConfig,
  readAllConfig,
  Recipe,
  recoverConfig,
  createSession,
  openSession,
  getSession,
  validateConfig,
} from '../api';

export enum AgentState {
  UNINITIALIZED = 'uninitialized',
  INITIALIZING = 'initializing',
  NO_PROVIDER = 'no_provider',
  INITIALIZED = 'initialized',
  ERROR = 'error',
}

export interface InitializationContext {
  recipe?: Recipe;
  resumeSessionId?: string;
  setAgentWaitingMessage: (msg: string | null) => void;
  setIsExtensionsLoading?: (isLoading: boolean) => void;
}

interface UseAgentReturn {
  agentState: AgentState;
  resetChat: () => void;
  loadCurrentChat: (context: InitializationContext) => Promise<ChatType>;
}

export class NoProviderOrModelError extends Error {
  constructor() {
    super('No provider or model configured');
    this.name = this.constructor.name;
  }
}

export function useAgent(): UseAgentReturn {
  const [agentState, setAgentState] = useState<AgentState>(AgentState.UNINITIALIZED);
  const [sessionId, setSessionId] = useState<string | null>(null);
  const initPromiseRef = useRef<Promise<ChatType> | null>(null);
  const deletedSessionsRef = useRef<Set<string>>(new Set());
  const recipeIdFromConfig = useRef<string | null>(
    (window.appConfig.get('recipeId') as string | null | undefined) ?? null
  );
  const recipeDeeplinkFromConfig = useRef<string | null>(
    (window.appConfig.get('recipeDeeplink') as string | null | undefined) ?? null
  );
  const scheduledJobIdFromConfig = useRef<string | null>(
    (window.appConfig.get('scheduledJobId') as string | null | undefined) ?? null
  );
  const { getExtensions, addExtension, read } = useConfig();

  const resetChat = useCallback(() => {
    setSessionId(null);
    setAgentState(AgentState.UNINITIALIZED);
    recipeIdFromConfig.current = null;
    recipeDeeplinkFromConfig.current = null;
    scheduledJobIdFromConfig.current = null;
    deletedSessionsRef.current.clear();
  }, []);

  const agentIsInitialized = agentState === AgentState.INITIALIZED;
  const currentChat = useCallback(
    async (initContext: InitializationContext): Promise<ChatType> => {
      // Skip deleted sessions
      if (
        initContext.resumeSessionId &&
        deletedSessionsRef.current.has(initContext.resumeSessionId)
      ) {
        initContext.resumeSessionId = undefined;

        // Clear from URL
        const url = new URL(window.location.href);
        url.searchParams.delete('resumeSessionId');
        window.history.replaceState({}, '', url.toString());
      }

      if (sessionId && deletedSessionsRef.current.has(sessionId)) {
        setSessionId(null);
      }

      // If already initialized, just get the session data
      if (agentIsInitialized && sessionId && !deletedSessionsRef.current.has(sessionId)) {
        let sessionResponse;
        try {
          sessionResponse = await getSession({
            path: { session_id: sessionId },
            throwOnError: true,
          });
        } catch {
          // Mark session as deleted and clear state
          deletedSessionsRef.current.add(sessionId);
          setSessionId(null);

          // Clear from URL
          const url = new URL(window.location.href);
          if (url.searchParams.get('resumeSessionId')) {
            url.searchParams.delete('resumeSessionId');
            window.history.replaceState({}, '', url.toString());
          }
        }

        // Fall through to create new session
        if (sessionResponse?.data) {
          const agentSession = sessionResponse.data;
          const messages = agentSession.conversation || [];
          return {
            sessionId: agentSession.id,
            name: agentSession.recipe?.title || agentSession.name,
            messages,
            recipe: agentSession.recipe,
            recipeParameterValues: agentSession.user_recipe_values || null,
          };
        }
      }

      if (initPromiseRef.current) {
        return initPromiseRef.current;
      }

      const initPromise = (async () => {
        setAgentState(AgentState.INITIALIZING);
        const agentWaitingMessage = initContext.setAgentWaitingMessage;
        agentWaitingMessage('Agent is initializing');

        try {
          const config = window.electron.getConfig();
          const provider = (await read('GOOSE_PROVIDER', false)) ?? config.GOOSE_DEFAULT_PROVIDER;
          const model = (await read('GOOSE_MODEL', false)) ?? config.GOOSE_DEFAULT_MODEL;

          if (!provider || !model) {
            setAgentState(AgentState.NO_PROVIDER);
            throw new NoProviderOrModelError();
          }

          let agentSession;
          try {
            if (initContext.resumeSessionId) {
              const response = await openSession({
                path: { session_id: initContext.resumeSessionId },
                body: { provider: provider as string, model: model as string },
                throwOnError: true,
              });
              agentSession = response.data;
            } else {
              const response = await createSession({
                body: {
                  working_dir: window.appConfig.get('GOOSE_WORKING_DIR') as string,
                  provider: provider as string,
                  model: model as string,
                  ...buildRecipeInput(
                    initContext.recipe,
                    recipeIdFromConfig.current,
                    recipeDeeplinkFromConfig.current
                  ),
                },
                throwOnError: true,
              });
              agentSession = response.data;
            }
          } catch (error) {
            // If resuming fails, mark session as deleted and create new session
            if (initContext.resumeSessionId) {
              deletedSessionsRef.current.add(initContext.resumeSessionId);

              // Clear from URL
              const url = new URL(window.location.href);
              url.searchParams.delete('resumeSessionId');
              window.history.replaceState({}, '', url.toString());

              const response = await createSession({
                body: {
                  working_dir: window.appConfig.get('GOOSE_WORKING_DIR') as string,
                  provider: provider as string,
                  model: model as string,
                  ...buildRecipeInput(
                    initContext.recipe,
                    recipeIdFromConfig.current,
                    recipeDeeplinkFromConfig.current
                  ),
                },
                throwOnError: true,
              });
              agentSession = response.data;

              // Clear resume flag
              initContext.resumeSessionId = undefined;
            } else {
              throw error;
            }
          }

          if (!agentSession) {
            throw Error('Failed to get session info');
          }
          setSessionId(agentSession.id);

          if (!initContext.recipe && agentSession.recipe && scheduledJobIdFromConfig.current) {
            agentSession.recipe = {
              ...agentSession.recipe,
              scheduledJobId: scheduledJobIdFromConfig.current,
              isScheduledExecution: true,
            } as Recipe;
            scheduledJobIdFromConfig.current = null;
          }

          recipeIdFromConfig.current = null;
          recipeDeeplinkFromConfig.current = null;

          agentWaitingMessage('Agent is loading config');

          await initConfig();

          try {
            await readAllConfig({ throwOnError: true });
          } catch (error) {
            console.warn('Initial config read failed, attempting recovery:', error);
            await handleConfigRecovery();
          }

          agentWaitingMessage('Extensions are loading');

          await loadExtensionsProgressively(agentSession.id, {
            getExtensions,
            addExtension,
            setIsExtensionsLoading: initContext.setIsExtensionsLoading,
          });

          const recipe = initContext.recipe || agentSession.recipe;
          const conversation = agentSession.conversation || [];
          // If we're loading a recipe from initContext (new recipe load), start with empty messages
          // Otherwise, use the messages from the session
          const messages = initContext.recipe && !initContext.resumeSessionId ? [] : conversation;
          let initChat: ChatType = {
            sessionId: agentSession.id,
            name: agentSession.recipe?.title || agentSession.name,
            messages: messages,
            recipe: recipe,
            recipeParameterValues: agentSession.user_recipe_values || null,
          };

          setAgentState(AgentState.INITIALIZED);

          return initChat;
        } catch (error) {
          if (
            (error + '').includes('Failed to create provider') ||
            error instanceof NoProviderOrModelError
          ) {
            setAgentState(AgentState.NO_PROVIDER);
            throw error;
          }
          setAgentState(AgentState.ERROR);
          if (typeof error === 'object' && error !== null && 'message' in error) {
            let error_message = error.message as string;
            throw new Error(error_message);
          }
          throw error;
        } finally {
          agentWaitingMessage(null);
          initPromiseRef.current = null;
        }
      })();

      initPromiseRef.current = initPromise;
      return initPromise;
    },
    [agentIsInitialized, sessionId, read, getExtensions, addExtension]
  );

  return {
    agentState,
    resetChat,
    loadCurrentChat: currentChat,
  };
}

const handleConfigRecovery = async () => {
  const configVersion = localStorage.getItem('configVersion');
  const shouldMigrateExtensions = !configVersion || parseInt(configVersion, 10) < 3;

  if (shouldMigrateExtensions) {
    try {
      await backupConfig({ throwOnError: true });
      await initConfig();
    } catch (migrationError) {
      console.error('Migration failed:', migrationError);
    }
  }

  try {
    await validateConfig({ throwOnError: true });
    await readAllConfig({ throwOnError: true });
  } catch {
    try {
      await recoverConfig({ throwOnError: true });
      await readAllConfig({ throwOnError: true });
    } catch {
      console.warn('Config recovery failed, reinitializing...');
      await initConfig();
    }
  }
};

const buildRecipeInput = (
  recipeOverride?: Recipe,
  recipeId?: string | null,
  recipeDeeplink?: string | null
) => {
  if (recipeId) {
    return { recipe_id: recipeId };
  }

  if (recipeDeeplink) {
    return { recipe_deeplink: recipeDeeplink };
  }

  if (recipeOverride) {
    return { recipe: recipeOverride };
  }

  return {};
};
