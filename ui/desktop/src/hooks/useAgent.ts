import { useCallback, useRef, useState } from 'react';
import { useConfig } from '../components/ConfigContext';
import { ChatType } from '../types/chat';
import { initializeSystem } from '../utils/providerUtils';
import { initializeCostDatabase } from '../utils/costDatabase';
import {
  backupConfig,
  initConfig,
  Message as ApiMessage,
  readAllConfig,
  Recipe,
  recoverConfig,
  resumeAgent,
  startAgent,
  validateConfig,
} from '../api';
import { COST_TRACKING_ENABLED } from '../updates';
import { convertApiMessageToFrontendMessage } from '../components/context_management';

export enum AgentState {
  UNINITIALIZED = 'uninitialized',
  INITIALIZING = 'initializing',
  NO_PROVIDER = 'no_provider',
  INITIALIZED = 'initialized',
  ERROR = 'error',
}

export interface InitializationContext {
  recipeConfig?: Recipe;
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
  const [recipeFromAppConfig, setRecipeFromAppConfig] = useState<Recipe | null>(
    (window.appConfig.get('recipe') as Recipe) || null
  );

  const { getExtensions, addExtension, read } = useConfig();

  const resetChat = useCallback(() => {
    setSessionId(null);
    setAgentState(AgentState.UNINITIALIZED);
    setRecipeFromAppConfig(null);
  }, []);

  const agentIsInitialized = agentState === AgentState.INITIALIZED;
  const currentChat = useCallback(
    async (initContext: InitializationContext): Promise<ChatType> => {
      if (agentIsInitialized && sessionId) {
        const agentResponse = await resumeAgent({
          body: {
            session_id: sessionId,
          },
          throwOnError: true,
        });

        const agentSessionInfo = agentResponse.data;
        const sessionMetadata = agentSessionInfo?.session;
        const messages = agentSessionInfo?.session.conversation || [];
        return {
          sessionId: agentSessionInfo.session_id,
          title: sessionMetadata.recipe?.title || sessionMetadata.description,
          messageHistoryIndex: 0,
          messages: messages?.map((message: ApiMessage) =>
            convertApiMessageToFrontendMessage(message)
          ),
          recipeConfig: sessionMetadata.recipe,
        };
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

          const agentResponse = initContext.resumeSessionId
            ? await resumeAgent({
                body: {
                  session_id: initContext.resumeSessionId,
                },
                throwOnError: true,
              })
            : await startAgent({
                body: {
                  working_dir: window.appConfig.get('GOOSE_WORKING_DIR') as string,
                  recipe: recipeFromAppConfig ?? initContext.recipeConfig,
                },
                throwOnError: true,
              });

          const agentSessionInfo = agentResponse.data.session;
          if (!agentSessionInfo) {
            throw Error('Failed to get session info');
          }
          setSessionId(agentSessionInfo.id);

          agentWaitingMessage('Agent is loading config');

          await initConfig();

          try {
            await readAllConfig({ throwOnError: true });
          } catch (error) {
            console.warn('Initial config read failed, attempting recovery:', error);
            await handleConfigRecovery();
          }

          agentWaitingMessage('Extensions are loading');
          await initializeSystem(agentSessionInfo.id, provider as string, model as string, {
            getExtensions,
            addExtension,
            setIsExtensionsLoading: initContext.setIsExtensionsLoading,
          });

          if (COST_TRACKING_ENABLED) {
            try {
              await initializeCostDatabase();
            } catch (error) {
              console.error('Failed to initialize cost database:', error);
            }
          }

          const messages = agentSessionInfo.conversation || [];
          let initChat: ChatType = {
            sessionId: agentSessionInfo.id,
            title: agentSessionInfo.recipe?.title || agentSessionInfo.description,
            messageHistoryIndex: 0,
            messages: messages.map((message: ApiMessage) =>
              convertApiMessageToFrontendMessage(message)
            ),
            recipeConfig: agentSessionInfo.recipe,
          };

          setAgentState(AgentState.INITIALIZED);

          return initChat;
        } catch (error) {
          if ((error + '').includes('Failed to create provider')) {
            setAgentState(AgentState.NO_PROVIDER);
          } else {
            setAgentState(AgentState.ERROR);
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
    [agentIsInitialized, sessionId, read, recipeFromAppConfig, getExtensions, addExtension]
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
